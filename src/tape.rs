#[derive(Clone, Copy, Debug)]
pub enum TapeName {
    Tape3_5,
    Tape6,
    Tape9,
    Tape12,
    Tape18,
    Tape24,
    Tape36,
}

impl std::fmt::Display for TapeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TapeName::Tape3_5 => write!(f, "3.5"),
            TapeName::Tape6 => write!(f, "6"),
            TapeName::Tape9 => write!(f, "9"),
            TapeName::Tape12 => write!(f, "12"),
            TapeName::Tape18 => write!(f, "18"),
            TapeName::Tape24 => write!(f, "24"),
            TapeName::Tape36 => write!(f, "36"),
        }
    }
}

#[derive(Clone)]
pub struct TapeSpec {
    pub name: TapeName, // Tape name
    pub width: u32,     // Total tape width in dots
    pub inner: u32,     // Printable width in dots
    pub margin: u32,    // Margin on each side in dots
    pub width_mm: u8,   // Total tape width in dots
    pub right_pin: u32, // Right margin pin count for raster
    #[allow(dead_code)]
    pub left_pin: u32, // Left margin pin count for raster (for future use)
}

impl TapeSpec {
    pub fn new(tape_name: TapeName) -> Self {
        let (width, inner, margin, width_mm, left_pin, right_pin) = match tape_name {
            TapeName::Tape3_5 => (48, 48, 0, 4, 248, 264),
            TapeName::Tape6 => (84, 64, 10, 6, 240, 256),
            TapeName::Tape9 => (128, 106, 11, 9, 219, 235),
            TapeName::Tape12 => (170, 150, 10, 12, 197, 213),
            TapeName::Tape18 => (256, 234, 11, 18, 155, 171),
            TapeName::Tape24 => (340, 320, 10, 24, 112, 128),
            TapeName::Tape36 => (512, 454, 29, 36, 45, 61),
        };
        TapeSpec {
            name: tape_name,
            width,
            inner,
            margin,
            width_mm,
            left_pin,
            right_pin,
        }
    }

    pub fn from_width(dots: u32) -> Option<Self> {
        match dots {
            48 => Some(Self::new(TapeName::Tape3_5)),
            84 => Some(Self::new(TapeName::Tape6)),
            128 => Some(Self::new(TapeName::Tape9)),
            170 => Some(Self::new(TapeName::Tape12)),
            256 => Some(Self::new(TapeName::Tape18)),
            340 => Some(Self::new(TapeName::Tape24)),
            512 => Some(Self::new(TapeName::Tape36)),
            _ => None,
        }
    }

    pub fn from_width_mm(mm: u8) -> Option<Self> {
        match mm {
            4 => Some(Self::new(TapeName::Tape3_5)), // 3.5mm reported as 4
            6 => Some(Self::new(TapeName::Tape6)),
            9 => Some(Self::new(TapeName::Tape9)),
            12 => Some(Self::new(TapeName::Tape12)),
            18 => Some(Self::new(TapeName::Tape18)),
            24 => Some(Self::new(TapeName::Tape24)),
            36 => Some(Self::new(TapeName::Tape36)),
            _ => None,
        }
    }
}
