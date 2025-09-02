use crate::raster_command::RasterCommand;
use crate::status::Status;
use snmp2::{SyncSession, Value};
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

pub trait Backend {
    fn send_command(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    fn get_status(&mut self) -> Result<Status, Box<dyn std::error::Error>>;
}

impl Backend for Box<dyn Backend> {
    fn send_command(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        (**self).send_command(data)
    }

    fn get_status(&mut self) -> Result<Status, Box<dyn std::error::Error>> {
        (**self).get_status()
    }
}

pub struct NetworkBackend {
    stream: TcpStream,
    host: String,
}

impl NetworkBackend {
    pub fn new(host: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Default to port 9100 for P-Touch printers
        let address = if host.contains(':') {
            host.to_string()
        } else {
            format!("{}:9100", host)
        };

        let stream = TcpStream::connect(&address)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        Ok(NetworkBackend {
            stream,
            host: host.to_string(),
        })
    }
}

impl Backend for NetworkBackend {
    fn send_command(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.write_all(data)?;
        self.stream.flush()?;
        Ok(())
    }

    fn get_status(&mut self) -> Result<Status, Box<dyn std::error::Error>> {
        // Use SNMP to get status from Brother P-Touch printer
        // OID: 1.3.6.1.4.1.2435.3.3.9.1.6.1.0
        let oid = "1.3.6.1.4.1.2435.3.3.9.1.6.1.0"
            .parse()
            .map_err(|e| format!("Invalid OID: {:?}", e))?;

        // Extract hostname for SNMP (remove port if specified)
        let snmp_host = if let Some(pos) = self.host.find(':') {
            &self.host[..pos]
        } else {
            &self.host
        };

        let snmp_addr = format!("{}:161", snmp_host);
        let mut session = SyncSession::new_v2c(snmp_addr, b"public", None, 0)?;

        let mut response = session.get(&oid)?;

        // Get the first (and should be only) varbind from the response
        if let Some((_oid, value)) = response.varbinds.next() {
            match value {
                Value::OctetString(data) => {
                    if data.len() == 32 {
                        let mut status_data = [0u8; 32];
                        status_data.copy_from_slice(data);
                        Ok(Status::new(status_data))
                    } else {
                        Err(format!(
                            "Invalid status data length: expected 32 bytes, got {}",
                            data.len()
                        )
                        .into())
                    }
                }
                _ => Err("Invalid SNMP response type: expected OctetString".into()),
            }
        } else {
            Err("No SNMP response received".into())
        }
    }
}

pub struct UsbBackend {
    device: rusb::DeviceHandle<rusb::GlobalContext>,
    endpoint_in: u8,
    endpoint_out: u8,
    timeout: Duration,
}

impl UsbBackend {
    // device_specifier is in the form of vendor_id:product_id (e.g., "04f9:2085")
    pub fn new(device_specifier: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (vendor_id, product_id) =
            if let Some((vendor_str, product_str)) = device_specifier.split_once(':') {
                let vendor = u16::from_str_radix(vendor_str.trim_start_matches("0x"), 16)?;
                let product = u16::from_str_radix(product_str.trim_start_matches("0x"), 16)?;
                (vendor, product)
            } else {
                return Err("USB device specifier must be in format vendor_id:product_id".into());
            };

        let devices = rusb::devices()?;
        let mut target_device = None;

        for device in devices.iter() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == vendor_id && device_desc.product_id() == product_id {
                target_device = Some(device);
                break;
            }
        }

        let device = target_device.ok_or("Brother P-Touch printer not found via USB")?;
        let handle = device.open()?;

        if handle.kernel_driver_active(0)? {
            handle.detach_kernel_driver(0)?;
        }

        handle.set_active_configuration(1)?;

        let config_desc = device.config_descriptor(0)?;
        let mut printer_interface = None;
        let mut interface_number = 0;

        for interface in config_desc.interfaces() {
            for descriptor in interface.descriptors() {
                if descriptor.class_code() == 7 {
                    printer_interface = Some(descriptor);
                    interface_number = interface.number();
                    break;
                }
            }
            if printer_interface.is_some() {
                break;
            }
        }

        let interface_desc = printer_interface.ok_or("No printer interface found")?;
        handle.claim_interface(interface_number)?;

        let mut endpoint_in = 0;
        let mut endpoint_out = 0;

        for endpoint_desc in interface_desc.endpoint_descriptors() {
            match endpoint_desc.direction() {
                rusb::Direction::In => endpoint_in = endpoint_desc.address(),
                rusb::Direction::Out => endpoint_out = endpoint_desc.address(),
            }
        }

        if endpoint_in == 0 || endpoint_out == 0 {
            return Err("Could not find required USB endpoints".into());
        }

        eprintln!("USB connection established:");
        eprintln!("  Interface: {}", interface_number);
        eprintln!("  Endpoint IN: 0x{:02x}", endpoint_in);
        eprintln!("  Endpoint OUT: 0x{:02x}", endpoint_out);

        Ok(UsbBackend {
            device: handle,
            endpoint_in,
            endpoint_out,
            timeout: Duration::from_secs(10),
        })
    }
}

impl Backend for UsbBackend {
    fn send_command(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let bytes_written = self
            .device
            .write_bulk(self.endpoint_out, data, self.timeout)?;
        eprintln!(
            "USB write: {} bytes written out of {} bytes",
            bytes_written,
            data.len()
        );
        if bytes_written != data.len() {
            return Err(format!(
                "Incomplete USB write: {} of {} bytes",
                bytes_written,
                data.len()
            )
            .into());
        }
        Ok(())
    }

    fn get_status(&mut self) -> Result<Status, Box<dyn std::error::Error>> {
        // Send status information request via USB
        let mut cmd = RasterCommand::new();
        cmd.invalidate().initialize().status_information_request();
        let buf = cmd.build();

        eprintln!("Sending command ({} bytes)...", buf.len());
        self.send_command(&buf)?;

        eprintln!("Command sent, waiting for response...");

        // Give the printer some time to process the command
        std::thread::sleep(Duration::from_millis(200));

        // Read status response with polling
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(3);
        let mut response_buffer = [0u8; 32];

        loop {
            if start_time.elapsed() > timeout {
                return Err("Status response timeout".into());
            }

            match self
                .device
                .read_bulk(self.endpoint_in, &mut response_buffer, self.timeout)
            {
                Ok(n) if n >= 32 => {
                    eprintln!("Successfully read {} bytes", n);
                    break;
                }
                Ok(n) => {
                    eprintln!("Partial read: {} bytes, continuing...", n);
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(rusb::Error::Timeout) => {
                    if start_time.elapsed() < Duration::from_secs(2) {
                        eprintln!("No data yet, waiting...");
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    } else {
                        eprintln!("Connection closed by printer (timeout after no response)");
                        return Err("Connection closed by printer".into());
                    }
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
            }
        }

        Ok(Status::new(response_buffer))
    }
}

/// Create a backend based on the host specifier
///
/// # Arguments
/// * `host` - Host specifier: hostname for network or vid:pid for USB
///
/// # Returns
/// * Backend implementation (NetworkBackend or UsbBackend)
pub fn from_host(host: &str) -> Result<Box<dyn Backend>, Box<dyn std::error::Error>> {
    fn is_usb_specifier(host: &str) -> bool {
        host.contains(':') && host.chars().all(|c| c.is_ascii_hexdigit() || c == ':')
    }

    if is_usb_specifier(host) {
        Ok(Box::new(UsbBackend::new(host)?))
    } else {
        Ok(Box::new(NetworkBackend::new(host)?))
    }
}
