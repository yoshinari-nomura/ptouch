use crate::Result;
use fontdb::Database;
use qrcode;
use resvg::{tiny_skia, usvg};
use std::fmt::{self, Display};
use std::sync::Arc;
use svg::node::element as svge;

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

    /// Check if this bounding box is empty (no content)
    pub fn is_empty(&self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }

    /// Calculate the union of two bounding boxes for layering.
    /// The x, y coordinates are always taken from self (base layer).
    pub fn union(&self, other: &Self) -> Self {
        let mx = (self.x + self.width).max(other.x + other.width);
        let my = (self.y + self.height).max(other.y + other.height);

        Self {
            x: self.x, // Base layer coordinates
            y: self.y, // Base layer coordinates
            width: mx - self.x,
            height: my - self.y,
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
fn enclose_group(node: impl Into<Box<dyn svg::Node>>) -> svge::Group {
    svge::Group::new().add(node)
}

/// Common interface for all renderable elements in the layout system
pub trait Element: Display {
    /// Calculate the bounding box of this element
    fn bounding_box(&self) -> Result<BoundingBox>;

    /// Render this element as an SVG Group
    fn render(&self) -> Result<svge::Group>;

    /// Return true if this element is visible (should be rendered)
    fn is_visible(&self) -> bool {
        true
    }

    /// Render this element at a specific position with proper coordinate transformation
    fn render_at(&self, x: f32, y: f32) -> Result<svge::Group> {
        let bbox = self.bounding_box()?;
        let group = self.render()?;

        // Combine bbox correction (-bbox.x, -bbox.y) and position placement (x, y)
        let tr = format!("translate({}, {})", x - bbox.x, y - bbox.y);

        Ok(group.set("transform", tr))
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
    pub fn new(texts: &[String], options: TextOptions) -> Result<Self> {
        validate_font(&options.font_name, &options.fontdb)?;

        Ok(Text {
            options,
            texts: texts.to_vec(),
        })
    }
}

impl Element for Text {
    fn bounding_box(&self) -> Result<BoundingBox> {
        calculate_text_bbox(
            &self.options.font_name,
            self.options.font_size,
            self.options.line_height,
            &self.texts,
            &self.options.fontdb,
        )
    }

    fn render(&self) -> Result<svge::Group> {
        let text_element = create_text_element(
            &self.options.font_name,
            self.options.font_size,
            self.options.line_height,
            &self.texts,
        );
        Ok(enclose_group(text_element))
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Text({})", self.texts.join(","))
    }
}

fn create_text_element(
    font_name: &str,
    font_size: u32,
    line_height: u32,
    texts: &[String],
) -> svge::Text {
    let mut text = svge::Text::new("")
        .set("font-family", font_name)
        .set("font-size", font_size)
        .set("fill", "black")
        .set("text-anchor", "start")
        .set("xml:space", "preserve")
        // ImageMagick `convert` does not respect dominant-baseline.
        // So, if ptouch creates an SVG with dominant-baseline
        // and convert it by ImageMagick, PNG will be broken.
        // It is sad.
        // .set("dominant-baseline", "hanging")
        .set("y", 0);

    // As for resvg crate, which is used in ptouch,
    // it renders the text out of ViewBox without dominant-baseline="hanging"
    //
    // Therefore, we have to use large `dy` at the first line to
    // put the whole line in ViewBox. Double the font size will work.
    //
    // If you do not care about ImageMagic, enable dominant-baseline="hanging" and:
    //   let mut dy = 0;
    // is enough.
    let mut dy = font_size * 2;

    for line in texts {
        let str = if line.is_empty() {
            // Empty tspan not rendered / dy-value ignored
            // https://stackoverflow.com/questions/34078357/empty-tspan-not-rendered-dy-value-ignored
            " ".into()
        } else {
            line.clone()
        };
        let tspan = svge::TSpan::new(str).set("x", 0).set("dy", dy);
        text = text.add(tspan);
        dy = line_height; // Subsequent lines use normal line height
    }

    text
}

fn validate_font(font_name: &str, fontdb: &Database) -> Result<()> {
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
) -> Result<BoundingBox> {
    // Create a temporary SVG for pre-rendering
    let max_line_length = texts.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let line_count = texts.len();

    let vw = max_line_length * font_size as usize + 500;
    let vh = line_count * font_size as usize + 500;

    let txt = create_text_element(font_name, font_size, line_height, texts);
    let doc = svg::Document::new()
        .set("viewBox", (0, 0, vw, vh))
        .set("xmlns", "http://www.w3.org/2000/svg")
        .add(txt);
    let svg = doc.to_string();

    // let result = calculate_text_logical_bbox(&svg, fontdb)?;
    let result = calculate_pixel_bbox(&svg, fontdb)?;

    Ok(result)
}

pub fn render_svg_to_pixmap(
    svg_data: &str,
    fontdb: &Arc<Database>,
    enable_antialiasing: bool,
) -> Result<tiny_skia::Pixmap> {
    let options = if enable_antialiasing {
        usvg::Options {
            fontdb: fontdb.clone(),
            ..Default::default()
        }
    } else {
        usvg::Options {
            fontdb: fontdb.clone(),
            text_rendering: usvg::TextRendering::OptimizeSpeed,
            shape_rendering: usvg::ShapeRendering::CrispEdges,
            ..Default::default()
        }
    };

    let tree = usvg::Tree::from_str(svg_data, &options)?;
    let size = tree.size().to_int_size();

    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("Failed to create pixmap")?;

    // Render SVG to pixmap
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    Ok(pixmap)
}

// Calculate text bounding box using SVG text metrics instead of pixel scanning
// **doesn't actually work**
//
// This is because this function assumes that Text has `dominant-baseline="hanging"`
// and the first tspan is created with `dy="0"`.  The SVG created by
// `create_text_element` doesn't satisfy these conditions.
//
// Additionally, this function generates a larger bbox than pixel
// scanning. While it's not suitable for creating a PNG that fits
// perfectly on tape, I've kept it as it may be useful for debugging
// purposes.
//
#[allow(dead_code)]
fn calculate_text_logical_bbox(
    svg_data: &str,
    fontdb: &Arc<Database>,
    enable_antialiasing: bool,
) -> Result<BoundingBox> {
    let options = if enable_antialiasing {
        usvg::Options {
            fontdb: fontdb.clone(),
            ..Default::default()
        }
    } else {
        usvg::Options {
            fontdb: fontdb.clone(),
            text_rendering: usvg::TextRendering::OptimizeSpeed,
            shape_rendering: usvg::ShapeRendering::CrispEdges,
            ..Default::default()
        }
    };

    let tree = usvg::Tree::from_str(svg_data, &options)?;

    // Try to get the overall bounding box of the SVG tree
    // This should include all elements including text with proper whitespace positioning
    let bbox = tree.root().abs_bounding_box();

    // Check if the bounding box has valid dimensions
    if bbox.width() > 0.0 && bbox.height() > 0.0 {
        Ok(BoundingBox {
            width: bbox.width(),
            height: bbox.height(),
            x: bbox.left(),
            y: bbox.top(),
        })
    } else {
        // Fallback to pixel-based calculation if no valid bounding box found
        calculate_pixel_bbox(svg_data, fontdb)
    }
}

fn calculate_pixel_bbox(svg_data: &str, fontdb: &Arc<Database>) -> Result<BoundingBox> {
    // Use shared rendering logic
    let pixmap = render_svg_to_pixmap(svg_data, fontdb, false)?;

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

    // If you want to get a bbox that only cuts out the real drawing area,
    // you should not set min_x to 0, but I set it to 0 for the following reasons:
    //
    // 1. preserve the spaces that users put at the beginning of lines.
    // 2. The size of the left side bearing of the first character differs
    //    for each glyph, so when Text elements are arranged vertically, they
    //    may become uneven.
    //
    // Therefore, it will result in generating a wider bbox with the
    // left-side bearing of the first character.
    //
    min_x = 0;

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
    pub fn new(data: String) -> Result<Self> {
        // Validate that the data can be encoded as QR code
        qrcode::QrCode::new(&data)?;

        Ok(QrCode {
            data,
            module_size: 5.0, // 5 SVG units ≈ 0.35mm at 360dpi FIXME: 360DPI
        })
    }

    /// Compact version of render with optimized path data
    fn render_compact(&self) -> Result<Box<dyn svg::Node>> {
        let qr = qrcode::QrCode::new(&self.data)?;
        let modules = qr.to_colors();
        let width = qr.width();

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

        let path = svge::Path::new()
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
    fn bounding_box(&self) -> Result<BoundingBox> {
        if self.elements.is_empty() {
            return Ok(BoundingBox::default());
        }

        let padding = BoundingBox::new(self.options.padding, 0.0, 0.0, 0.0);
        let mut combined = BoundingBox::default();
        let mut prev_was_visible = false;

        for elm in &self.elements {
            let bbox = elm.bounding_box()?;

            // Add padding between visible elements
            if elm.is_visible() && prev_was_visible {
                combined = combined.h_append(padding);
            }

            combined = combined.h_append(bbox);

            // Update flag for next iteration
            prev_was_visible = elm.is_visible();
        }

        Ok(combined)
    }

    fn render(&self) -> Result<svge::Group> {
        let mut group = svge::Group::new();
        let mut x = 0.0;

        // Get maximum height from our own bounding box
        let height = self.bounding_box()?.height;
        let mut prev_was_visible = false;

        for elm in &self.elements {
            let bbox = elm.bounding_box()?;

            // Add padding between visible elements
            if elm.is_visible() && prev_was_visible {
                x += self.options.padding;
            }

            // Calculate Y offset based on alignment
            let y = match self.options.align {
                VerticalAlign::Top => 0.0,
                VerticalAlign::Center => (height - bbox.height) / 2.0,
                VerticalAlign::Bottom => height - bbox.height,
            };

            // Only render visible elements
            if elm.is_visible() {
                let eg = elm.render_at(x, y)?;
                group = group.add(eg);
            }

            x += bbox.width;

            // Update flag for next iteration
            prev_was_visible = elm.is_visible();
        }

        Ok(group)
    }
}

impl Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children: Vec<String> = self.elements.iter().map(|e| format!("{}", e)).collect();
        write!(f, "Row({})", children.join(","))
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
    fn bounding_box(&self) -> Result<BoundingBox> {
        if self.elements.is_empty() {
            return Ok(BoundingBox::default());
        }

        let padding = BoundingBox::new(0.0, self.padding, 0.0, 0.0);
        let mut combined = BoundingBox::default();
        let mut prev_was_visible = false;

        for elm in &self.elements {
            let bbox = elm.bounding_box()?;

            // Add padding between visible elements
            if elm.is_visible() && prev_was_visible {
                combined = combined.v_append(padding);
            }

            combined = combined.v_append(bbox);

            // Update flag for next iteration
            prev_was_visible = elm.is_visible();
        }

        Ok(combined)
    }

    fn render(&self) -> Result<svge::Group> {
        let mut group = svge::Group::new();
        let mut y = 0.0;
        let mut prev_was_visible = false;

        for elm in &self.elements {
            let bbox = elm.bounding_box()?;

            // Add padding between visible elements
            if elm.is_visible() && prev_was_visible {
                y += self.padding;
            }

            // Only render visible elements
            if elm.is_visible() {
                let eg = elm.render_at(0.0, y)?;
                group = group.add(eg);
            }

            y += bbox.height;

            // Update flag for next iteration
            prev_was_visible = elm.is_visible();
        }

        Ok(group)
    }
}

impl Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children: Vec<String> = self.elements.iter().map(|e| format!("{}", e)).collect();
        write!(f, "Column({})", children.join(","))
    }
}

impl Element for QrCode {
    fn bounding_box(&self) -> Result<BoundingBox> {
        let qr = qrcode::QrCode::new(&self.data)?;
        let width = qr.width() as f32;
        let size = width * self.module_size;

        Ok(BoundingBox {
            width: size,
            height: size,
            x: 0.0,
            y: 0.0,
        })
    }

    fn render(&self) -> Result<svge::Group> {
        let path = self.render_compact()?;
        Ok(enclose_group(path))
    }
}

impl Display for QrCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QrCode({})", self.data)
    }
}

pub struct Gap {
    width: f32,
    height: f32,
    visible: bool,
}

impl Gap {
    pub fn new(width: f32, height: f32, visible: bool) -> Self {
        Gap {
            width,
            height,
            visible,
        }
    }

    pub fn parse(spec: &str, visible: bool) -> Result<Self> {
        if let Some(x) = spec.find('x') {
            let ws = &spec[..x];
            let hs = &spec[x + 1..];

            let width: f32 = ws
                .parse()
                .map_err(|_| format!("Invalid gap/box spec '{}'", spec))?;
            let height: f32 = hs
                .parse()
                .map_err(|_| format!("Invalid gap/box spec '{}'", spec))?;

            Ok(Gap::new(width, height, visible))
        } else {
            // Single number means square gap/box
            let size: f32 = spec
                .parse()
                .map_err(|_| format!("Invalid gap/box spec: {}", spec))?;
            Ok(Gap::new(size, size, visible))
        }
    }
}

impl Element for Gap {
    fn bounding_box(&self) -> Result<BoundingBox> {
        Ok(BoundingBox {
            width: self.width,
            height: self.height,
            x: 0.0,
            y: 0.0,
        })
    }

    fn render(&self) -> Result<svge::Group> {
        if self.visible {
            let rect = svge::Rectangle::new()
                .set("width", self.width)
                .set("height", self.height)
                .set("fill", "black");
            Ok(enclose_group(rect))
        } else {
            // Gap is invisible - just empty group
            Ok(svge::Group::new())
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Display for Gap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.visible {
            write!(f, "Box({}x{})", self.width, self.height)
        } else {
            write!(f, "Gap({}x{})", self.width, self.height)
        }
    }
}

pub struct Overlay {
    elements: Vec<Box<dyn Element>>,
}

impl Overlay {
    pub fn new(elements: Vec<Box<dyn Element>>) -> Self {
        Overlay { elements }
    }
}

impl Element for Overlay {
    fn bounding_box(&self) -> Result<BoundingBox> {
        self.elements
            .iter()
            .map(|e| e.bounding_box())
            .try_fold(BoundingBox::default(), |acc, bbox| Ok(acc.union(&bbox?)))
    }

    fn render(&self) -> Result<svge::Group> {
        let mut group = svge::Group::new();

        // Stack layers in order (later layers render on top)
        for element in &self.elements {
            let layer_group = element.render_at(0.0, 0.0)?;
            group = group.add(layer_group);
        }

        Ok(group)
    }

    fn is_visible(&self) -> bool {
        // At least one layer is visible
        self.elements.iter().any(|e| e.is_visible())
    }
}

impl Display for Overlay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let layers: Vec<String> = self.elements.iter().map(|e| format!("{}", e)).collect();
        write!(f, "Overlay({})", layers.join(","))
    }
}
