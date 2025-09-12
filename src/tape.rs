#[derive(Clone, Copy, Debug)]
pub enum Tape {
    TZe3H,
    TZe6H,
    TZe9H,
    TZe12H,
    TZe18H,
    TZe24H,
    TZe36H,
    TZe3L,
    TZe6L,
    TZe9L,
    TZe12L,
    TZe18L,
    TZe24L,
}

impl std::fmt::Display for Tape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tape::TZe3H => write!(f, "3.5mm (360dpi)"),
            Tape::TZe6H => write!(f, "6mm (360dpi)"),
            Tape::TZe9H => write!(f, "9mm (360dpi)"),
            Tape::TZe12H => write!(f, "12mm (360dpi)"),
            Tape::TZe18H => write!(f, "18mm (360dpi)"),
            Tape::TZe24H => write!(f, "24mm (360dpi)"),
            Tape::TZe36H => write!(f, "36mm (360dpi)"),

            Tape::TZe3L => write!(f, "3.5mm (180dpi)"),
            Tape::TZe6L => write!(f, "6mm (180dpi)"),
            Tape::TZe9L => write!(f, "9mm (180dpi)"),
            Tape::TZe12L => write!(f, "12mm (180dpi)"),
            Tape::TZe18L => write!(f, "18mm (180dpi)"),
            Tape::TZe24L => write!(f, "24mm (180dpi)"),
        }
    }
}

#[derive(Clone)]
pub struct TapeSpec {
    pub name: Tape,      // Tape name
    pub width_mm: u8,    // Total tape width in mm
    pub width_dots: u32, // Total tape width in dots
    pub inner_dots: u32, // Printable width in dots
    pub total_pins: u32, // Total printer pins
    pub right_pins: u32, // Margin pin count for raster
    pub dpi: u32,        // Printer's DPI
}

impl TapeSpec {
    #[rustfmt::skip]
    pub fn new(tape_name: Tape) -> Self {
        let (width_mm, width_dots, inner_dots, total_pins, right_pins, dpi) = match tape_name {
            Tape::TZe3H  =>  (4,  48,  48, 560, 264, 360),
            Tape::TZe6H  =>  (6,  84,  64, 560, 256, 360),
            Tape::TZe9H  =>  (9, 128, 106, 560, 235, 360),
            Tape::TZe12H => (12, 170, 150, 560, 213, 360),
            Tape::TZe18H => (18, 256, 234, 560, 171, 360),
            Tape::TZe24H => (24, 340, 320, 560, 128, 360),
            Tape::TZe36H => (36, 512, 454, 560,  61, 360),

            Tape::TZe3L  =>  (4,  24,  24, 128,  52, 180),
            Tape::TZe6L  =>  (6,  42,  32, 128,  48, 180),
            Tape::TZe9L  =>  (9,  64,  50, 128,  39, 180),
            Tape::TZe12L => (12,  84,  70, 128,  29, 180),
            Tape::TZe18L => (18, 128, 112, 128,   8, 180),
            Tape::TZe24L => (24, 170, 128, 128,   0, 180),
        };
        TapeSpec {
            name: tape_name,
            width_mm,
            width_dots,
            inner_dots,
            total_pins,
            right_pins,
            dpi,
        }
    }

    pub fn mm_to_dots(&self, mm: f32) -> u32 {
        ((mm * self.dpi as f32) / 25.4).round() as u32
    }

    pub fn from_width_dots_and_dpi(dots: u32, dpi: u32) -> Option<Self> {
        match (dots, dpi) {
            (48, 360) => Some(Self::new(Tape::TZe3H)),
            (84, 360) => Some(Self::new(Tape::TZe6H)),
            (128, 360) => Some(Self::new(Tape::TZe9H)),
            (170, 360) => Some(Self::new(Tape::TZe12H)),
            (256, 360) => Some(Self::new(Tape::TZe18H)),
            (340, 360) => Some(Self::new(Tape::TZe24H)),
            (512, 360) => Some(Self::new(Tape::TZe36H)),

            (24, 180) => Some(Self::new(Tape::TZe3L)),
            (42, 180) => Some(Self::new(Tape::TZe6L)),
            (64, 180) => Some(Self::new(Tape::TZe9L)),
            (84, 180) => Some(Self::new(Tape::TZe12L)),
            (128, 180) => Some(Self::new(Tape::TZe18L)),
            (170, 180) => Some(Self::new(Tape::TZe24L)),
            _ => None,
        }
    }

    pub fn from_width_mm_and_dpi(mm: u8, dpi: u32) -> Option<Self> {
        match (mm, dpi) {
            (4, 360) => Some(Self::new(Tape::TZe3H)),
            (6, 360) => Some(Self::new(Tape::TZe6H)),
            (9, 360) => Some(Self::new(Tape::TZe9H)),
            (12, 360) => Some(Self::new(Tape::TZe12H)),
            (18, 360) => Some(Self::new(Tape::TZe18H)),
            (24, 360) => Some(Self::new(Tape::TZe24H)),
            (36, 360) => Some(Self::new(Tape::TZe36H)),

            (4, 180) => Some(Self::new(Tape::TZe3L)),
            (6, 180) => Some(Self::new(Tape::TZe6L)),
            (9, 180) => Some(Self::new(Tape::TZe9L)),
            (12, 180) => Some(Self::new(Tape::TZe12L)),
            (18, 180) => Some(Self::new(Tape::TZe18L)),
            (24, 180) => Some(Self::new(Tape::TZe24L)),
            _ => None,
        }
    }
}
