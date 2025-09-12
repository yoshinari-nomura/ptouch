pub struct Status {
    raw_data: [u8; 32],
}

impl Status {
    pub fn new(data: [u8; 32]) -> Self {
        Status { raw_data: data }
    }

    pub fn raw_data(&self) -> &[u8; 32] {
        &self.raw_data
    }

    pub fn has_errors(&self) -> bool {
        self.error_info1() != 0x00 || self.error_info2() != 0x00
    }

    pub fn error_info1(&self) -> u8 {
        self.raw_data[8]
    }

    pub fn error_info2(&self) -> u8 {
        self.raw_data[9]
    }

    pub fn media_width_mm(&self) -> u8 {
        self.raw_data[10]
    }

    pub fn media_type(&self) -> u8 {
        self.raw_data[11]
    }

    pub fn printer_dpi(&self) -> u32 {
        match self.raw_data[4] {
            0x6F | 0x70 | 0x71 | 0x78 => 360, // PT-P900W, PT-P950NW, PT-P900, PT-P910BT
            0x5A => 180,                      // PT-9200PC
            _ => 360,
        }
    }

    pub fn print_status_info(&self, verbose: bool) {
        if verbose {
            println!("Raw status response ({} bytes):", self.raw_data.len());
            print!("  Hex: ");
            for byte in &self.raw_data {
                print!("{:02X} ", byte);
            }
            println!();
            println!();
        }

        if !self.has_errors() {
            println!("Status: OK - No errors");
        } else {
            println!("Status: ERROR");
            self.print_error_details();
        }

        println!("Media width: {} mm", self.media_width_mm());
        println!("Media type: 0x{:02X}", self.media_type());

        if verbose {
            self.print_detailed_breakdown();
        }
    }

    fn print_error_details(&self) {
        let error_info1 = self.error_info1();
        let error_info2 = self.error_info2();

        if error_info1 & 0x01 != 0 {
            println!("  - No media");
        }
        if error_info1 & 0x02 != 0 {
            println!("  - End of media");
        }
        if error_info1 & 0x04 != 0 {
            println!("  - Cutter jam");
        }
        if error_info1 & 0x08 != 0 {
            println!("  - Weak batteries");
        }
        if error_info1 & 0x10 != 0 {
            println!("  - Printer in use");
        }
        if error_info1 & 0x40 != 0 {
            println!("  - High-voltage adapter");
        }

        if error_info2 & 0x01 != 0 {
            println!("  - Wrong media");
        }
        if error_info2 & 0x02 != 0 {
            println!("  - Expansion buffer full");
        }
        if error_info2 & 0x04 != 0 {
            println!("  - Communication error");
        }
        if error_info2 & 0x08 != 0 {
            println!("  - Communication buffer full");
        }
        if error_info2 & 0x10 != 0 {
            println!("  - Cover open");
        }
        if error_info2 & 0x20 != 0 {
            println!("  - Overheating");
        }
        if error_info2 & 0x40 != 0 {
            println!("  - Tape leader mark not detected");
        }
        if error_info2 & 0x80 != 0 {
            println!("  - System error");
        }
    }

    fn print_detailed_breakdown(&self) {
        let error_info1 = self.error_info1();
        let error_info2 = self.error_info2();

        println!();
        println!("Detailed status breakdown:");
        println!("  Error info 1 (0x{:02X}):", error_info1);
        for bit in 0..8 {
            if error_info1 & (1 << bit) != 0 {
                println!("    Bit {}: Set", bit);
            }
        }
        println!("  Error info 2 (0x{:02X}):", error_info2);
        for bit in 0..8 {
            if error_info2 & (1 << bit) != 0 {
                println!("    Bit {}: Set", bit);
            }
        }
    }
}
