use crate::Result;
use crate::tape::TapeSpec;
use png::ColorType;

pub struct PrintableImage {
    png_data: Vec<u8>,
    tape_spec: TapeSpec,
}

impl PrintableImage {
    pub fn from_png_data(png_data: Vec<u8>, tape_spec: TapeSpec) -> Result<Self> {
        // Validate PNG dimensions match tape width
        let decoder = png::Decoder::new(png_data.as_slice());
        let reader = decoder.read_info()?;
        let png_info = reader.info();
        let png_height = png_info.height;

        if png_height != tape_spec.width_dots {
            return Err(format!(
                "PNG height mismatch: PNG height is {} pixels, but {} mm tape requires {} pixels",
                png_height, tape_spec.width_mm, tape_spec.width_dots
            )
            .into());
        }

        Ok(PrintableImage {
            png_data,
            tape_spec,
        })
    }

    pub fn to_raster_lines(&self) -> Result<Vec<Vec<u8>>> {
        png_to_raster_lines(&self.png_data, &self.tape_spec)
    }

    pub fn tape_spec(&self) -> &TapeSpec {
        &self.tape_spec
    }
}

fn png_to_raster_lines(png_data: &[u8], tape_spec: &TapeSpec) -> Result<Vec<Vec<u8>>> {
    let decoder = png::Decoder::new(png_data);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;

    let gray_buf = convert_to_grayscale(&buf, info.color_type)?;

    let width = info.width as usize;
    let height = info.height as usize;
    let bytes_per_raster = (tape_spec.total_pins / 8) as usize;
    let mut raster_lines = Vec::new();

    for x in 0..width {
        let mut raster_line = vec![0u8; bytes_per_raster];

        // Mapping the Y-range (margin, margin+inner-1) of the PNG to
        // (right_pin, right_pin+inner-1)
        let margin = ((tape_spec.width_dots - tape_spec.inner_dots) / 2) as usize;
        let inner = tape_spec.inner_dots as usize;
        let right_pin = tape_spec.right_pins as usize;

        for y in margin..(margin + inner).min(height) {
            let pin = right_pin + (y - margin);

            if pin < tape_spec.total_pins as usize {
                let pixel_idx = y * width + x;
                if pixel_idx < gray_buf.len() {
                    let pixel = gray_buf[pixel_idx];
                    if pixel < 127 {
                        let byte_idx = pin / 8;
                        let bit_idx = 7 - (pin % 8);
                        raster_line[byte_idx] |= 1 << bit_idx;
                    }
                }
            }
        }

        raster_lines.push(raster_line);
    }

    Ok(raster_lines)
}

fn convert_to_grayscale(buf: &[u8], color_type: ColorType) -> Result<Vec<u8>> {
    match color_type {
        ColorType::Grayscale => Ok(buf.to_vec()),
        ColorType::Rgb => Ok(buf
            .chunks(3)
            .map(|rgb| ((rgb[0] as u32 + rgb[1] as u32 + rgb[2] as u32) / 3) as u8)
            .collect()),
        ColorType::Rgba => Ok(buf
            .chunks(4)
            .map(|rgba| {
                let alpha = rgba[3] as f32 / 255.0;
                let r = (rgba[0] as f32 * alpha + 255.0 * (1.0 - alpha)) as u32;
                let g = (rgba[1] as f32 * alpha + 255.0 * (1.0 - alpha)) as u32;
                let b = (rgba[2] as f32 * alpha + 255.0 * (1.0 - alpha)) as u32;
                ((r + g + b) / 3) as u8
            })
            .collect()),
        _ => Err("Unsupported color type".into()),
    }
}

fn take_consecutive_run(data: &[u8]) -> &[u8] {
    if data.len() < 2 || data[0] != data[1] {
        return &[];
    }

    let first_byte = data[0];
    let mut len = 1;

    while len < data.len() && data[len] == first_byte && len < 255 {
        len += 1;
    }

    &data[..len]
}

fn take_literal_run(data: &[u8]) -> &[u8] {
    let mut len = 0;

    while len < data.len() && len < 127 {
        let remaining = &data[len..];
        if !take_consecutive_run(remaining).is_empty() {
            break;
        }
        len += 1;
    }

    &data[..len]
}

pub fn compress_tiff_group4(data: &[u8]) -> Result<Vec<u8>> {
    // TIFF Group 4 Run Length Encoding for Brother P-Touch
    // Based on cv_ptp900_eng_raster_102.pdf "Select compression mode" example:
    // - Run data (consecutive same bytes): negative count (two's complement) + byte
    // - Literal data (non-consecutive): positive count + raw bytes

    let mut compressed = Vec::new();

    if data.is_empty() {
        return Ok(compressed);
    }

    let mut remaining = data;

    while !remaining.is_empty() {
        let consecutive_run = take_consecutive_run(remaining);

        let count = if !consecutive_run.is_empty() {
            let count = consecutive_run.len();
            let negative_count = (256 - (count - 1)) as u8;
            compressed.push(negative_count);
            compressed.push(consecutive_run[0]);
            count
        } else {
            let literal_run = take_literal_run(remaining);
            let count = literal_run.len();
            compressed.push((count - 1) as u8);
            compressed.extend_from_slice(literal_run);
            count
        };

        remaining = &remaining[count..];
    }

    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_tiff_group4_all_black() {
        // Test case: 70 bytes of 0x00 should compress to bb 00
        let black_data = vec![0x00u8; 70];
        let result = compress_tiff_group4(&black_data).unwrap();

        // Should compress to exactly: bb 00
        assert_eq!(
            result,
            vec![0xbb, 0x00],
            "70 bytes of 0x00 should compress to [bb 00]"
        );
    }

    #[test]
    fn test_compress_tiff_group4_all_white() {
        // Test case: 70 bytes of 0xFF should compress to bb ff
        let white_data = vec![0xFFu8; 70];
        let result = compress_tiff_group4(&white_data).unwrap();

        // Should compress to exactly: bb ff
        assert_eq!(
            result,
            vec![0xbb, 0xff],
            "70 bytes of 0xFF should compress to [bb ff]"
        );
    }

    #[test]
    fn test_compress_tiff_group4_alternating() {
        // Test case: Alternating pattern (should be handled as literal data)
        let mut alt_data = Vec::new();
        for i in 0..70 {
            alt_data.push(if i % 2 == 0 { 0x00 } else { 0xFF });
        }

        let result = compress_tiff_group4(&alt_data).unwrap();

        // Alternating pattern should be treated as literal data
        // First byte should be positive (literal count), followed by raw data
        assert!(!result.is_empty(), "Should produce some output");
        assert!(
            result[0] < 128,
            "First byte should be positive (literal mode)"
        );

        // Should not compress well - likely larger than 70% of original
        let compression_ratio = result.len() as f64 / alt_data.len() as f64;
        assert!(
            compression_ratio > 0.7,
            "Alternating pattern should not compress well"
        );
    }

    #[test]
    fn test_compress_tiff_group4_empty() {
        let empty_data = Vec::new();
        let result = compress_tiff_group4(&empty_data).unwrap();
        assert_eq!(
            result,
            Vec::<u8>::new(),
            "Empty input should produce empty output"
        );
    }

    #[test]
    fn test_compress_tiff_group4_single_byte() {
        let single_data = vec![0x42];
        let result = compress_tiff_group4(&single_data).unwrap();

        // Single byte should be literal: count=0 (1-1), then the byte
        assert_eq!(result, vec![0, 0x42], "Single byte should produce [00 42]");
    }

    #[test]
    fn test_compress_tiff_group4_literal_data() {
        // Test case: Non-consecutive data should be handled as literal
        let literal_data = vec![0x23, 0xBA, 0xBF, 0xA2, 0x22, 0x2B];
        let result = compress_tiff_group4(&literal_data).unwrap();

        // Should be literal data: count=5 (6-1), then all 6 bytes
        let expected = vec![
            5, // count = 6-1 = 5 (positive number for literal)
            0x23, 0xBA, 0xBF, 0xA2, 0x22, 0x2B, // raw data
        ];
        assert_eq!(result, expected, "Non-consecutive data should be literal");
    }

    #[test]
    fn test_compress_tiff_group4_mixed_data() {
        // Test case: Mix of literal and run data
        let mixed_data = vec![0x23, 0xBA, 0xBF, 0xFF, 0xFF, 0xFF, 0xA2, 0x22, 0x2B];
        let result = compress_tiff_group4(&mixed_data).unwrap();

        // Should be: literal [23 BA BF] + run [3x FF] + literal [A2 22 2B]
        let expected = vec![
            2, // literal count = 3-1 = 2
            0x23, 0xBA, 0xBF, // literal data
            0xFE, // run count = -(3-1) = -2 = 254 (two's complement)
            0xFF, // run value
            2,    // literal count = 3-1 = 2
            0xA2, 0x22, 0x2B, // literal data
        ];
        assert_eq!(
            result, expected,
            "Mixed literal and run data should be handled correctly"
        );
    }
}
