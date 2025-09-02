/// Brother P-Touch raster command builder
///
/// Based on Raster Command Reference (4. Printing Command Details)
/// <https://download.brother.com/welcome/docp100407/cv_ptp900_eng_raster_102.pdf>
#[derive(Clone, Copy, Debug)]
pub enum CommandMode {
    /// ESC/P mode (traditional mode)
    EscP = 0,
    /// Raster mode (recommended for P-Touch)
    Raster = 1,
    /// Template mode
    Template = 3,
}

/// Page position for multi-page printing
///
/// Used in [`RasterCommand::print_information_command`] to indicate page sequence
#[derive(Clone, Copy, Debug)]
pub enum PageType {
    /// First page of multi-page print job
    FirstPage = 0,
    /// Middle page of multi-page print job
    MiddlePage = 1,
    /// Last page or single page print job
    LastPage = 2,
}

/// Builder for Brother P-Touch raster commands
///
/// This struct provides a fluent interface to build command sequences for
/// Brother P-Touch label printers. Commands are based on the Raster Command Reference.
///
/// Reference: <https://download.brother.com/welcome/docp100407/cv_ptp900_eng_raster_102.pdf>
///
/// # Example
///
/// ```
/// use ptouch::raster_command::{RasterCommand, CommandMode, PageType};
///
/// let mut cmd = RasterCommand::new();
/// cmd.invalidate()
///    .initialize()
///    .switch_dynamic_command_mode(CommandMode::Raster)
///    .print_information_command(
///        false,                    // quality_mode
///        true,                     // recover_mode
///        Some(0),                  // media_type
///        Some(12),                 // media_width (12mm)
///        None,                     // media_length
///        100,                      // raster_number
///        PageType::LastPage,
///    )
///    .print_command_with_feeding();
///
/// let command_data = cmd.build();
/// // Send command_data to printer...
/// ```
pub struct RasterCommand {
    buffer: Vec<u8>,
}

impl RasterCommand {
    /// Create a new empty command builder
    pub fn new() -> Self {
        RasterCommand { buffer: Vec::new() }
    }

    /// Add invalidate sequence (200 null bytes)
    ///
    /// This command clears any previous data and prepares the printer
    /// for a new command sequence. Should be called at the beginning
    /// of each print job.
    pub fn invalidate(&mut self) -> &mut Self {
        self.buffer.extend_from_slice(&[0x00; 200]);
        self
    }

    /// Add ESC @ (initialize) command
    ///
    /// Resets the printer to its default state. This should be called
    /// after invalidate and before other commands.
    pub fn initialize(&mut self) -> &mut Self {
        self.buffer.extend_from_slice(b"\x1B\x40");
        self
    }

    /// Add status information request command
    ///
    /// Requests the printer to send back its current status.
    /// The printer will respond with a 32-byte status packet.
    ///
    /// # Note
    /// This command only works with USB connections. TCP-connected P-Touch
    /// printers do not respond to status requests.
    ///
    /// See: <https://github.com/masatomizuta/py-brotherlabel/issues/3>
    pub fn status_information_request(&mut self) -> &mut Self {
        self.buffer.extend_from_slice(b"\x1B\x69\x53");
        self
    }

    /// Switch to dynamic command mode
    ///
    /// Sets the printer to the specified command mode. For P-Touch printers,
    /// Raster mode is recommended for optimal performance.
    ///
    /// # Arguments
    /// * `mode` - The command mode to switch to
    pub fn switch_dynamic_command_mode(&mut self, mode: CommandMode) -> &mut Self {
        self.buffer.extend_from_slice(b"\x1B\x69\x61");
        self.buffer.push(mode as u8);
        self
    }

    /// Set print information command
    ///
    /// Configures print settings including media type, dimensions, and flags.
    /// This command must be sent before raster data transfer.
    ///
    /// # Arguments
    /// * `quality_mode` - Priority given to print quality (Not used)
    /// * `recover_mode` - Use Bi-directional communication. Printer sends status on printing.
    /// * `media_type` - Media type (0 for laminated tape)
    /// * `media_width` - Media width in mm
    /// * `media_length` - Media length in mm (None for continuous)
    /// * `raster_number` - Number of raster lines to follow
    /// * `page_type` - Page type (first/middle/last)
    #[allow(clippy::too_many_arguments)]
    pub fn print_information_command(
        &mut self,
        quality_mode: bool,
        recover_mode: bool,
        media_type: Option<u8>,
        media_width: Option<u8>,
        media_length: Option<u8>,
        raster_number: u32,
        page_type: PageType,
    ) -> &mut Self {
        let mut flag = 0u8;

        let media_type_val = match media_type {
            Some(val) => {
                flag |= 0x02;
                val
            }
            None => 0,
        };

        let media_width_val = match media_width {
            Some(val) => {
                flag |= 0x04;
                val
            }
            None => 0,
        };

        let media_length_val = match media_length {
            Some(val) => {
                flag |= 0x08;
                val
            }
            None => 0,
        };

        flag |= (quality_mode as u8) << 6; // 0x40
        flag |= (recover_mode as u8) << 7; // 0x80

        self.buffer.extend_from_slice(b"\x1B\x69\x7A");
        self.buffer.push(flag);
        self.buffer.push(media_type_val);
        self.buffer.push(media_width_val);
        self.buffer.push(media_length_val);
        // raster_number as little-endian u32
        self.buffer.push((raster_number & 0xFF) as u8);
        self.buffer.push(((raster_number >> 8) & 0xFF) as u8);
        self.buffer.push(((raster_number >> 16) & 0xFF) as u8);
        self.buffer.push(((raster_number >> 24) & 0xFF) as u8);
        self.buffer.push(page_type as u8);
        self.buffer.push(0x00);
        self
    }

    /// Set various mode settings
    ///
    /// # Arguments
    /// * `auto_cut` - Enable automatic cutting after print
    /// * `mirror` - Enable mirror printing
    pub fn various_mode_settings(&mut self, auto_cut: bool, mirror: bool) -> &mut Self {
        let param = (auto_cut as u8) << 6  // 0x40
                  | (mirror as u8)   << 7; // 0x80
        self.buffer.extend_from_slice(b"\x1B\x69\x4D");
        self.buffer.push(param);
        self
    }

    /// Set advanced mode settings
    ///
    /// # Arguments
    /// * `draft` - Enable draft mode (faster printing)
    /// * `half_cut` - Enable half-cut (partial cut for easy peeling)
    /// * `no_chain` - Disable chain printing (cut after last label)
    /// * `special_tape` - Enable special tape mode
    /// * `high_resolution` - Enable high resolution mode
    /// * `no_clear` - Disable buffer clearing
    pub fn advanced_mode_settings(
        &mut self,
        draft: bool,
        half_cut: bool,
        no_chain: bool,
        special_tape: bool,
        high_resolution: bool,
        no_clear: bool,
    ) -> &mut Self {
        let param = (draft as u8)                 // 0x01
                  | (half_cut as u8)        << 2  // 0x04
                  | (no_chain as u8)        << 3  // 0x08
                  | (special_tape as u8)    << 4  // 0x10
                  | (high_resolution as u8) << 6  // 0x40
                  | (no_clear as u8)        << 7; // 0x80
        self.buffer.extend_from_slice(b"\x1B\x69\x4B");
        self.buffer.push(param);
        self
    }

    /// Specify margin amount
    ///
    /// Sets the margin of both sides in dots (1 dot = 1/360 inch at 360 DPI).
    ///
    /// # Arguments
    /// * `dots` - Margin size in dots (14 dots = 1 mm)
    ///
    /// # Warning
    /// Do not set this to 0. Zero margin can cause issues with label boundary
    /// detection, especially when switching between continuous and normal printing
    /// modes. The second label may appear blank. Use at least 14 dots (1mm) for
    /// reliable operation.
    pub fn specify_margin_amount(&mut self, dots: u16) -> &mut Self {
        self.buffer.extend_from_slice(b"\x1B\x69\x64");
        // little-endian u16
        self.buffer.push((dots & 0xFF) as u8);
        self.buffer.push(((dots >> 8) & 0xFF) as u8);
        self
    }

    /// Specify page number for multi-page jobs
    ///
    /// # Arguments
    /// * `n` - Page number (0-255). If 0 is set, label will not be cut.
    ///
    ///   When `auto_cut` is specified in [`RasterCommand::various_mode_settings`],
    ///   `n` specifies page number (1 - 255) in  "cut each * labels"
    pub fn specify_page_number(&mut self, n: u8) -> &mut Self {
        self.buffer.extend_from_slice(b"\x1B\x69\x41");
        self.buffer.push(n);
        self
    }

    /// Select compression mode
    ///
    /// # Arguments
    /// * `tiff` - true for TIFF Group 4 compression, false for no compression
    pub fn select_compression_mode(&mut self, tiff: bool) -> &mut Self {
        self.buffer.extend_from_slice(b"\x4D");
        self.buffer.push(if tiff { 0x02 } else { 0x00 });
        self
    }

    /// Transfer raster graphics data
    ///
    /// Sends one line of raster image data to the printer. The data should
    /// be compressed using TIFF Group 4 compression when compression is enabled.
    ///
    /// # Arguments
    /// * `data` - Compressed raster line data (max 65535 bytes)
    pub fn raster_graphics_transfer(&mut self, data: &[u8]) -> &mut Self {
        self.buffer.push(0x47); // 'G'
        let len = data.len() as u16;
        // little-endian u16
        self.buffer.push((len & 0xFF) as u8);
        self.buffer.push(((len >> 8) & 0xFF) as u8);
        self.buffer.extend_from_slice(data);
        self
    }

    /// Transfer zero raster graphics (blank line)
    ///
    /// Sends a blank raster line. More efficient than sending
    /// a line filled with zeros.
    pub fn zero_raster_graphics(&mut self) -> &mut Self {
        self.buffer.push(0x5A); // 'Z'
        self
    }

    /// Send print command
    ///
    /// Initiates printing without feeding. Use this for continuous
    /// label printing.
    pub fn print_command(&mut self) -> &mut Self {
        self.buffer.push(0x0C);
        self
    }

    /// Send print command with feeding
    ///
    /// Initiates printing and feeds the label. This is the most
    /// commonly used print command for single labels.
    pub fn print_command_with_feeding(&mut self) -> &mut Self {
        self.buffer.push(0x1A);
        self
    }

    /// Build and return the complete command sequence
    ///
    /// Consumes the builder and returns the raw command bytes
    /// ready to be sent to the printer.
    pub fn build(self) -> Vec<u8> {
        self.buffer
    }
}

impl Default for RasterCommand {
    fn default() -> Self {
        Self::new()
    }
}
