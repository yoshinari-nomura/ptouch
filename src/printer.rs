use crate::backend::Backend;
use crate::printable_image::{PrintableImage, compress_tiff_group4};
use crate::raster_command::{CommandMode, PageType, RasterCommand};
use crate::status::Status;

pub struct Printer<B: Backend> {
    backend: B,
}

impl<B: Backend> Printer<B> {
    pub fn new(backend: B) -> Self {
        Printer { backend }
    }

    pub fn get_status(&mut self) -> Result<Status, Box<dyn std::error::Error>> {
        self.backend.get_status()
    }

    pub fn print(
        &mut self,
        printable: &PrintableImage,
        continuous: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert to raster lines
        let raster_lines = printable.to_raster_lines()?;
        let raster_count = raster_lines.len() as u32;
        let tape_spec = printable.tape_spec();

        // Build raster command sequence
        let mut cmd = RasterCommand::new();
        cmd.invalidate()
            .initialize()
            .switch_dynamic_command_mode(CommandMode::Raster)
            .print_information_command(
                false,                    // quality_mode
                true,                     // recover_mode
                Some(0),                  // media_type
                Some(tape_spec.width_mm), // media_width
                Some(0),                  // media_length
                raster_count,
                PageType::LastPage,
            )
            .various_mode_settings(!continuous, false) // auto_cut=true if !continuous, mirror=false
            .specify_page_number(1) // always 1 for single page
            .advanced_mode_settings(
                false,       // draft
                true,        // half_cut
                !continuous, // no_chain: true=cut last label, false=continuous
                false,       // special_tape
                false,       // high_resolution
                false,       // no_buffer_clear
            )
            .specify_margin_amount(14) // 14 dots = 1mm
            .select_compression_mode(true); // TIFF compression

        // Add raster lines
        for raster_line in &raster_lines {
            let compressed_data = compress_tiff_group4(raster_line)?;
            cmd.raster_graphics_transfer(&compressed_data);
        }

        // Add print command
        cmd.print_command_with_feeding();

        let command_data = cmd.build();

        // Send to printer
        self.backend.send_command(&command_data)?;

        println!("Print command sent successfully");
        Ok(())
    }
}
