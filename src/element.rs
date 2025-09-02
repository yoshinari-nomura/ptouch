use fontdb::Database;
use qrcode;
use resvg::{tiny_skia, usvg};
use std::sync::Arc;
use svg::node::element;
use svg::Node;

#[derive(Clone, Copy, Debug, Default)]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug)]
pub struct RowOptions {
    pub align: VerticalAlign,
    pub padding: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundingBox {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl BoundingBox {
    pub fn new(width: f32, height: f32, x: f32, y: f32) -> Self {
        Self {
            width,
            height,
            x,
            y,
        }
    }

    /// Append another bounding box horizontally
    pub fn h_append(&self, other: Self) -> Self {
        Self {
            width: self.width + other.width,
            height: self.height.max(other.height),
            x: self.x,
            y: self.y,
        }
    }

    /// Append another bounding box vertically
    pub fn v_append(&self, other: Self) -> Self {
        Self {
            width: self.width.max(other.width),
            height: self.height + other.height,
            x: self.x,
            y: self.y,
        }
    }
}

impl std::fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}+{}+{}", self.width, self.height, self.x, self.y)
    }
}

/// Helper function to wrap a single element in a group
fn enclose_group(node: impl Into<Box<dyn Node>>) -> element::Group {
    element::Group::new().add(node)
}

/// Common interface for all renderable elements in the layout system
pub trait Element {
    /// Calculate the bounding box of this element
    fn bounding_box(&self) -> Result<BoundingBox, Box<dyn std::error::Error>>;

    /// Render this element as an SVG Group
    fn render(&self) -> Result<element::Group, Box<dyn std::error::Error>>;

    /// Render this element at a specific position with proper coordinate transformation
    fn render_at(&self, x: f32, y: f32) -> Result<element::Group, Box<dyn std::error::Error>> {
        let bbox = self.bounding_box()?;
        let group = self.render()?;

        // Combine bbox correction (-bbox.x, -bbox.y) and position placement (x, y)
        let transform = format!("translate({}, {})", x - bbox.x, y - bbox.y);

        Ok(group.set("transform", transform))
    }
}

#[derive(Clone)]
pub struct TextOptions {
    pub font_name: String,
    pub font_size: u32,
    pub line_height: u32,
    pub fontdb: Arc<Database>,
}

pub struct Text {
    options: TextOptions,
    texts: Vec<String>,
}

impl Text {
    pub fn new(texts: &[String], options: TextOptions) -> Result<Self, Box<dyn std::error::Error>> {
        validate_font(&options.font_name, &options.fontdb)?;

        Ok(Text {
            options,
            texts: texts.to_vec(),
        })
    }
}

impl Element for Text {
    fn bounding_box(&self) -> Result<BoundingBox, Box<dyn std::error::Error>> {
        calculate_text_bbox(
            &self.options.font_name,
            self.options.font_size,
            self.options.line_height,
            &self.texts,
            &self.options.fontdb,
        )
    }

    fn render(&self) -> Result<element::Group, Box<dyn std::error::Error>> {
        let text_element = create_text_element(
            &self.options.font_name,
            self.options.font_size,
            self.options.line_height,
            &self.texts,
        );
        Ok(enclose_group(text_element))
    }
}

fn create_text_element(
    font_name: &str,
    font_size: u32,
    line_height: u32,
    texts: &[String],
) -> element::Text {
    let mut text = element::Text::new("")
        .set("font-family", font_name)
        .set("font-size", font_size)
        .set("fill", "black")
        .set("text-anchor", "start")
        .set("y", 0);

    // Use larger dy for first line to ensure positive bbox coordinates
    let mut dy = font_size * 2; // Double the font size for first line
    for line in texts {
        let tspan = element::TSpan::new(line.clone()).set("x", 0).set("dy", dy);
        text = text.add(tspan);
        dy = line_height; // Subsequent lines use normal line height
    }

    text
}

fn validate_font(font_name: &str, fontdb: &Database) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the specified font family exists in the database
    let font_found = fontdb.faces().any(|face| {
        face.families
            .iter()
            .any(|(family_name, _)| family_name.eq_ignore_ascii_case(font_name))
    });

    if !font_found {
        return Err(format!("Font '{}' not found.", font_name).into());
    }

    Ok(())
}

fn calculate_text_bbox(
    font_name: &str,
    font_size: u32,
    line_height: u32,
    texts: &[String],
    fontdb: &Arc<Database>,
) -> Result<BoundingBox, Box<dyn std::error::Error>> {
    // Create a temporary SVG for pre-rendering
    let max_line_length = texts.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let line_count = texts.len();

    let vw = max_line_length * font_size as usize + 500;
    let vh = line_count * font_size as usize + 500;

    let text_element = create_text_element(font_name, font_size, line_height, texts);

    let document = svg::Document::new()
        .set("viewBox", (0, 0, vw, vh))
        .set("xmlns", "http://www.w3.org/2000/svg")
        .add(text_element);

    let svg_data = document.to_string();

    // Use pixel-based bounding box calculation
    let result = calculate_pixel_bbox(&svg_data, fontdb)?;

    Ok(result)
}

pub fn render_svg_to_pixmap(
    svg_data: &str,
    fontdb: &Arc<Database>,
) -> Result<tiny_skia::Pixmap, Box<dyn std::error::Error>> {
    let options = usvg::Options {
        fontdb: fontdb.clone(),
        ..Default::default()
    };

    let tree = usvg::Tree::from_str(svg_data, &options)?;
    let svg_size = tree.size().to_int_size();

    let mut pixmap = tiny_skia::Pixmap::new(svg_size.width(), svg_size.height())
        .ok_or("Failed to create pixmap")?;

    // Render SVG to pixmap
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    Ok(pixmap)
}

fn calculate_pixel_bbox(
    svg_data: &str,
    fontdb: &Arc<Database>,
) -> Result<BoundingBox, Box<dyn std::error::Error>> {
    // Use shared rendering logic
    let pixmap = render_svg_to_pixmap(svg_data, fontdb)?;

    // Find actual pixel bounds (like ImageMagick's %@)
    let pixels = pixmap.data();
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;

    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;
    let mut found_pixel = false;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4; // RGBA
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            let a = pixels[idx + 3];

            // Check if pixel is not white (has content)
            if a > 0 && (r < 255 || g < 255 || b < 255) {
                found_pixel = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    let result = if found_pixel {
        BoundingBox {
            x: min_x as f32,
            y: min_y as f32,
            width: (max_x - min_x + 1) as f32,
            height: (max_y - min_y + 1) as f32,
        }
    } else {
        // Fallback if no pixels found (shouldn't happen in practice)
        BoundingBox {
            width: width as f32,
            height: height as f32,
            x: 0.0,
            y: 0.0,
        }
    };

    Ok(result)
}

pub struct QrCode {
    data: String,
    module_size: f32,
}

impl QrCode {
    pub fn new(data: String) -> Result<Self, Box<dyn std::error::Error>> {
        // Validate that the data can be encoded as QR code
        qrcode::QrCode::new(&data)?;

        Ok(QrCode {
            data,
            module_size: 5.0, // 5 SVG units ≈ 0.35mm at 360dpi
        })
    }

    /// Compact version of render with optimized path data
    fn render_compact(&self) -> Result<Box<dyn Node>, Box<dyn std::error::Error>> {
        let qr_code = qrcode::QrCode::new(&self.data)?;
        let modules = qr_code.to_colors();
        let width = qr_code.width();

        let mut path_data = String::new();

        for y in 0..width {
            let y_pos = y as f32 * self.module_size;
            let mut x = 0;

            while x < width {
                let index = y * width + x;

                if modules[index] == qrcode::Color::Dark {
                    // Find consecutive dark modules in this row
                    let start_x = x;
                    while x < width {
                        let next_index = y * width + x;
                        if modules[next_index] != qrcode::Color::Dark {
                            break;
                        }
                        x += 1;
                    }
                    let run_length = x - start_x;

                    let x_pos = start_x as f32 * self.module_size;
                    let width_val = run_length as f32 * self.module_size;

                    // Always use absolute positioning for clarity
                    path_data.push_str(&format!("M{},{}", x_pos, y_pos));

                    // Draw rectangle: horizontal line, vertical line, horizontal back, close
                    path_data.push_str(&format!(
                        "h{}v{}h-{}z",
                        width_val, self.module_size, width_val
                    ));
                } else {
                    x += 1;
                }
            }
        }

        let path = element::Path::new()
            .set("d", path_data)
            .set("fill", "black")
            .set("fill-rule", "evenodd");

        Ok(Box::new(path))
    }
}

pub struct Row {
    elements: Vec<Box<dyn Element>>,
    options: RowOptions,
}

impl Row {
    pub fn new(elements: Vec<Box<dyn Element>>, options: RowOptions) -> Self {
        Row { elements, options }
    }
}

impl Element for Row {
    fn bounding_box(&self) -> Result<BoundingBox, Box<dyn std::error::Error>> {
        if self.elements.is_empty() {
            return Ok(BoundingBox::default());
        }

        let padding_box = BoundingBox::new(self.options.padding, 0.0, 0.0, 0.0);
        let mut combined = BoundingBox::default();
        let mut first = true;

        for element in &self.elements {
            if !first {
                combined = combined.h_append(padding_box);
            }
            first = false;
            let bbox = element.bounding_box()?;
            combined = combined.h_append(bbox);
        }

        Ok(combined)
    }

    fn render(&self) -> Result<element::Group, Box<dyn std::error::Error>> {
        let mut group = element::Group::new();
        let mut current_x = 0.0;

        // Get maximum height from our own bounding box
        let row_height = self.bounding_box()?.height;

        for element in &self.elements {
            let element_bbox = element.bounding_box()?;

            // Calculate Y offset based on alignment
            let y_offset = match self.options.align {
                VerticalAlign::Top => 0.0,
                VerticalAlign::Center => (row_height - element_bbox.height) / 2.0,
                VerticalAlign::Bottom => row_height - element_bbox.height,
            };

            let element_group = element.render_at(current_x, y_offset)?;
            group = group.add(element_group);
            current_x += element_bbox.width + self.options.padding;
        }

        Ok(group)
    }
}

pub struct Column {
    elements: Vec<Box<dyn Element>>,
    padding: f32,
}

impl Column {
    pub fn new(elements: Vec<Box<dyn Element>>, padding: f32) -> Self {
        Column { elements, padding }
    }
}

impl Element for Column {
    fn bounding_box(&self) -> Result<BoundingBox, Box<dyn std::error::Error>> {
        if self.elements.is_empty() {
            return Ok(BoundingBox::default());
        }

        let padding_box = BoundingBox::new(0.0, self.padding, 0.0, 0.0);
        let mut combined = BoundingBox::default();
        let mut first = true;

        for element in &self.elements {
            if !first {
                combined = combined.v_append(padding_box);
            }
            first = false;
            let bbox = element.bounding_box()?;
            combined = combined.v_append(bbox);
        }

        Ok(combined)
    }

    fn render(&self) -> Result<element::Group, Box<dyn std::error::Error>> {
        let mut group = element::Group::new();
        let mut current_y = 0.0;

        for element in &self.elements {
            let element_bbox = element.bounding_box()?;
            let element_group = element.render_at(0.0, current_y)?;
            group = group.add(element_group);
            current_y += element_bbox.height + self.padding;
        }

        Ok(group)
    }
}

impl Element for QrCode {
    fn bounding_box(&self) -> Result<BoundingBox, Box<dyn std::error::Error>> {
        let qr_code = qrcode::QrCode::new(&self.data)?;
        let width = qr_code.width() as f32;
        let size = width * self.module_size;

        Ok(BoundingBox {
            width: size,
            height: size,
            x: 0.0,
            y: 0.0,
        })
    }

    fn render(&self) -> Result<element::Group, Box<dyn std::error::Error>> {
        let path = self.render_compact()?;
        Ok(enclose_group(path))
    }
}
