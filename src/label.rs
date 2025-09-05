use crate::Result;
use crate::element::{Element, render_svg_to_pixmap};
use crate::tape::TapeSpec;
use fontdb::Database;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use svg::Document;

#[derive(Clone, Copy, Debug)]
pub enum Placement {
    Top,
    Center,
    Bottom,
}

impl std::fmt::Display for Placement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Placement::Top => write!(f, "top"),
            Placement::Center => write!(f, "center"),
            Placement::Bottom => write!(f, "bottom"),
        }
    }
}

pub struct LabelOptions {
    pub fontdb: Arc<Database>,
    pub tape_spec: TapeSpec,
    pub auto_scale: bool,
    pub rotate: bool,
    pub placement: Placement,
    pub debug: bool,
}

pub struct Label {
    element: Box<dyn Element>,
    options: LabelOptions,
}

impl Label {
    /// Create Label from Element (unified API)
    pub fn from_element(element: Box<dyn Element>, options: LabelOptions) -> Self {
        Label { element, options }
    }

    /// Create SVG document
    pub fn to_svg(&self) -> Result<String> {
        create_label_svg_from_element(&*self.element, &self.options)
    }

    /// Create PNG data
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let svg_data = self.to_svg()?;
        let pixmap = render_svg_to_pixmap(&svg_data, &self.options.fontdb)?;
        Ok(pixmap.encode_png()?)
    }

    /// Save SVG file
    pub fn save_svg<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let svg_data = self.to_svg()?;
        let mut file = File::create(path)?;
        file.write_all(svg_data.as_bytes())?;
        Ok(())
    }

    /// Save PNG file
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let png_data = self.to_png()?;
        let mut file = File::create(path)?;
        file.write_all(&png_data)?;
        Ok(())
    }

    /// Access method to Option
    pub fn options(&self) -> &LabelOptions {
        &self.options
    }
}

fn create_label_svg_from_element(element: &dyn Element, options: &LabelOptions) -> Result<String> {
    let tape = &options.tape_spec;

    // Elementからbounding_boxを取得
    let bbox = element.bounding_box()?;

    let vh = tape.width as f32;
    let ch = tape.inner as f32;
    let m = tape.margin as f32;

    // For rotation, we need to consider how the text dimensions map to tape dimensions
    let (effective_width, effective_height) = if options.rotate {
        // When rotated, text width becomes the height on the tape
        (bbox.height, bbox.width)
    } else {
        (bbox.width, bbox.height)
    };

    let mut vw = effective_width + 2.0;
    let mut scale = 1.0;
    let y_offset;

    // Handle auto-scaling
    if options.auto_scale {
        y_offset = m;
        scale = ch / effective_height;
        vw = effective_width * scale + 2.0;
    } else {
        // Handle placement
        y_offset = match options.placement {
            Placement::Top => m,
            Placement::Center => m + (ch - effective_height) / 2.0,
            Placement::Bottom => m + (ch - effective_height),
        };
    }

    let margin_color = if options.debug { "gray" } else { "white" };

    let mut document = Document::new()
        .set("viewBox", (0, 0, vw.round() as u32, vh as u32))
        .set("xmlns", "http://www.w3.org/2000/svg");

    // Add white background for the entire label
    document = document.add(
        svg::node::element::Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", vw.round() as u32)
            .set("height", vh as u32)
            .set("fill", "white"),
    );

    let mut content_group = element.render_at(0.0, 0.0)?;

    // Add debug bounding box (at original bbox position, will be moved by same transform)
    if options.debug {
        let bbox_rect = svg::node::element::Rectangle::new()
            .set("x", bbox.x as i32)
            .set("y", bbox.y as i32)
            .set("width", bbox.width as u32)
            .set("height", bbox.height as u32)
            .set("fill", "none")
            .set("stroke", "red")
            .set("stroke-width", 1);
        content_group = content_group.add(bbox_rect);
    }

    // Add rotation if enabled
    if options.rotate {
        content_group = svg::node::element::Group::new()
            .set("x", 0)
            .set("y", 0)
            .set(
                "transform",
                format!("rotate(-90, 0, 0) translate({}, 0)", -bbox.width),
            )
            .add(content_group);
    }

    // Add scaling if auto-scale is enabled
    if options.auto_scale {
        content_group = svg::node::element::Group::new()
            .set("transform", format!("scale({})", scale))
            .add(content_group);
    }

    // Create main group with translation
    let main_group = svg::node::element::Group::new()
        .set("transform", format!("translate(0, {})", y_offset))
        .add(content_group);
    document = document.add(main_group);

    // Add margin rectangles to mask non-printable areas (after text rendering)
    // Color: gray for debug, white for normal mode
    document = document
        .add(
            svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", vw.round() as u32)
                .set("height", m as u32)
                .set("fill", margin_color),
        )
        .add(
            svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", (ch + m) as u32)
                .set("width", vw.round() as u32)
                .set("height", m as u32)
                .set("fill", margin_color),
        );

    Ok(document.to_string())
}
