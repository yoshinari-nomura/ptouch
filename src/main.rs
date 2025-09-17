use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use clap_complete::{CompleteEnv, CompletionCandidate, generate};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use ptouch::backend;
use ptouch::element::TextOptions;
use ptouch::element::{RowOptions, VerticalAlign};
use ptouch::label::{Label, LabelOptions, Placement as LabelPlacement};
use ptouch::layout;
use ptouch::printable_image::PrintableImage;
use ptouch::printer::Printer;
use ptouch::tape::{self, Tape, TapeSpec};
use ptouch::{Result, get_font_names, load_fontdb_with_paths, unescape_shell_string};

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
#[clap(rename_all = "lowercase")]
enum Placement {
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

// To avoid bringing Clap into label:: and tape::, implement
// conversion from clap::ValueEnum to label, tape.
impl From<Placement> for LabelPlacement {
    fn from(placement: Placement) -> Self {
        match placement {
            Placement::Top => LabelPlacement::Top,
            Placement::Center => LabelPlacement::Center,
            Placement::Bottom => LabelPlacement::Bottom,
        }
    }
}

impl From<Placement> for VerticalAlign {
    fn from(placement: Placement) -> Self {
        match placement {
            Placement::Top => VerticalAlign::Top,
            Placement::Center => VerticalAlign::Center,
            Placement::Bottom => VerticalAlign::Bottom,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum TapeName {
    #[value(name = "3.5")]
    Tape3_5,
    #[value(name = "6")]
    Tape6,
    #[value(name = "9")]
    Tape9,
    #[value(name = "12")]
    Tape12,
    #[value(name = "18")]
    Tape18,
    #[value(name = "24")]
    Tape24,
    #[value(name = "36")]
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

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum Resolution {
    #[value(name = "180")]
    Dpi180,
    #[value(name = "360")]
    Dpi360,
}

impl Resolution {
    fn to_dpi(self) -> u32 {
        match self {
            Resolution::Dpi180 => 180,
            Resolution::Dpi360 => 360,
        }
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_dpi())
    }
}

impl TapeName {
    fn to_tape(self, resolution: Resolution) -> Result<Tape> {
        match (self, resolution) {
            (TapeName::Tape3_5, Resolution::Dpi360) => Ok(Tape::TZe3H),
            (TapeName::Tape6, Resolution::Dpi360) => Ok(Tape::TZe6H),
            (TapeName::Tape9, Resolution::Dpi360) => Ok(Tape::TZe9H),
            (TapeName::Tape12, Resolution::Dpi360) => Ok(Tape::TZe12H),
            (TapeName::Tape18, Resolution::Dpi360) => Ok(Tape::TZe18H),
            (TapeName::Tape24, Resolution::Dpi360) => Ok(Tape::TZe24H),
            (TapeName::Tape36, Resolution::Dpi360) => Ok(Tape::TZe36H),
            (TapeName::Tape3_5, Resolution::Dpi180) => Ok(Tape::TZe3L),
            (TapeName::Tape6, Resolution::Dpi180) => Ok(Tape::TZe6L),
            (TapeName::Tape9, Resolution::Dpi180) => Ok(Tape::TZe9L),
            (TapeName::Tape12, Resolution::Dpi180) => Ok(Tape::TZe12L),
            (TapeName::Tape18, Resolution::Dpi180) => Ok(Tape::TZe18L),
            (TapeName::Tape24, Resolution::Dpi180) => Ok(Tape::TZe24L),
            (TapeName::Tape36, Resolution::Dpi180) => {
                Err("36mm tape not supported on 180DPI printers".into())
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "ptouch")]
#[command(about = "CLI for Brother P-Touch Label Writers")]
#[command(version = "0.1.0")]
#[command(next_line_help = false)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create label image for Brother P-Touch
    Image(ImageArgs),
    /// Send raster image to P-Touch
    Print(PrintArgs),
    /// Get status information from P-Touch
    Status(StatusArgs),
    /// Generate shell completion scripts
    Completion(CompletionArgs),
}

#[derive(Args)]
struct ImageArgs {
    /// Auto scale contents to the tape width
    #[arg(short = 'a', long = "auto-scale")]
    auto_scale: bool,

    /// Show alignment marks for debug
    #[arg(short = 'd', long = "debug")]
    debug: bool,

    /// Font name
    #[arg(short = 'f', long = "font", default_value = "Noto Sans CJK JP",
          add = ArgValueCompleter::new(font_completer))]
    font: String,

    /// Additional font path
    #[arg(
        short = 'F',
        long = "font-path",
        value_name = "FONT_PATH",
        long_help = "Additional font path: directory or font file (can be specified multiple times)"
    )]
    font_paths: Vec<PathBuf>,

    /// Line height in pixels [default: font-size]
    #[arg(short = 'l', long = "line-height")]
    line_height: Option<u32>,

    /// Output to file [default: stdout]
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Place contents
    #[arg(short = 'p', long = "placement", default_value_t = Placement::Top,
          long_help = "Place contents on the tape. [possible values: top, center, bottom]",
          hide_possible_values = true)]
    placement: Placement,

    /// Printer resolution in DPI
    #[arg(short = 'r', long = "resolution", default_value_t = Resolution::Dpi360,
          long_help = "Printer resolution in DPI. [possible values: 180, 360]",
          hide_possible_values = true)]
    resolution: Resolution,

    /// Rotate image by 90 degrees
    #[arg(short = 'R', long = "rotate")]
    rotate: bool,

    /// Font size in pixels
    #[arg(short = 's', long = "font-size", default_value = "24")]
    font_size: u32,

    /// Tape size in mm
    #[arg(short = 't', long = "tape-name", default_value_t = TapeName::Tape12,
          long_help = "Tape size in mm. [possible values: 3.5, 6, 9, 12, 18, 24, 36]",
          hide_possible_values = true)]
    tape_name: TapeName,

    /// Output SVG source instead of PNG
    #[arg(short = 'S', long = "source")]
    source: bool,

    /// Text lines to print [default: stdin]
    text: Vec<String>,
}

#[derive(Args)]
struct PrintArgs {
    /// Printer host: hostname.local (network) or vid:pid (USB)
    /// Examples: ptouch.local, 192.168.1.100, 04f9:2085
    #[arg(short = 'H', long = "host", required = true)]
    host: String,

    /// Enable continuous printing (no cutting)
    #[arg(short = 'c', long = "continuous")]
    continuous: bool,

    /// PNG file to print [default: stdin]
    png_file: Option<PathBuf>,
}

#[derive(Args)]
struct StatusArgs {
    /// Printer host: hostname.local (network) or vid:pid (USB)
    /// Examples: ptouch.local, 192.168.1.100, 04f9:2085
    #[arg(short = 'H', long = "host", required = true)]
    host: String,

    /// Show verbose information
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

#[derive(Args)]
struct CompletionArgs {
    /// Shell type
    #[arg(value_enum)]
    shell: clap_complete::Shell,
}

fn handle_image_command(args: ImageArgs) -> Result<()> {
    // Get text input
    let texts = if args.text.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        input.lines().map(|s| s.to_string()).collect()
    } else {
        args.text.clone()
    };

    if texts.is_empty() {
        return Err("No text input provided".into());
    }

    // Create fontdb from font paths
    let fontdb = load_fontdb_with_paths(&args.font_paths)?;

    // Create text options for layout parsing
    let text_options = TextOptions {
        font_name: args.font,
        font_size: args.font_size,
        line_height: args.line_height.unwrap_or(args.font_size),
    };

    // Create label options (simplified)
    let tape_spec = TapeSpec::new(args.tape_name.to_tape(args.resolution)?);

    // At 360 DPI, 14.0 is 1mm, 20.0 is 1.4mm
    // Note: This depends on ""quiet zone" of QR code
    let row_padding = tape_spec.mm_to_dots(1.4) as f32;

    let label_options = LabelOptions {
        fontdb: fontdb.clone(),
        tape_spec,
        auto_scale: args.auto_scale,
        rotate: args.rotate,
        placement: args.placement.into(),
        debug: args.debug,
    };

    // Create row options from placement
    let row_options = RowOptions {
        align: args.placement.into(),
        padding: row_padding,
    };

    // Create label using layout script parsing
    let element = layout::parse_layout_script(&texts, &text_options, &row_options, fontdb)?;
    let label = Label::from_element(element, label_options);

    if args.source {
        // Output source (SVG)
        match &args.output {
            Some(path) => {
                label.save_svg(path)?;
            }
            None => {
                print!("{}", label.to_svg()?);
            }
        }
    } else {
        // Output PNG
        match &args.output {
            Some(path) => {
                label.save_png(path)?;
            }
            None => {
                let png_data = label.to_png()?;
                io::stdout().write_all(&png_data)?;
            }
        }
    }

    Ok(())
}

fn handle_print_command(args: PrintArgs) -> Result<()> {
    // Read PNG data
    let png_data = match &args.png_file {
        Some(path) => std::fs::read(path)?,
        None => {
            let mut buffer = Vec::new();
            io::stdin().read_to_end(&mut buffer)?;
            buffer
        }
    };

    // Get PNG dimensions
    let decoder = png::Decoder::new(png_data.as_slice());
    let reader = decoder.read_info()?;
    let png_info = reader.info();
    let png_height = png_info.height;

    // Check printer status to get DPI and tape width
    println!("Checking printer status...");

    let backend = backend::from_host(&args.host)?;
    let mut printer = Printer::new(backend);
    let status = printer.get_status()?;

    // Check for errors first
    if status.has_errors() {
        println!("Printer error detected:");
        status.print_status_info(false);
        return Err("Cannot print due to printer errors".into());
    }

    // Get printer DPI and tape width
    let printer_dpi = status.printer_dpi();
    let actual_tape_width = status.media_width_mm();

    // Get tape spec from PNG dimensions using printer's DPI
    let png_tape_spec = tape::TapeSpec::from_width_dots_and_dpi(png_height, printer_dpi)
        .ok_or_else(|| {
            format!(
                "Unsupported PNG height: {} pixels at {}DPI",
                png_height, printer_dpi
            )
        })?;

    // Get printer tape spec using the same DPI
    let printer_tape_spec = tape::TapeSpec::from_width_mm_and_dpi(actual_tape_width, printer_dpi)
        .ok_or_else(|| {
        format!(
            "Unsupported tape width: {} mm at {}DPI",
            actual_tape_width, printer_dpi
        )
    })?;

    // Verify PNG tape spec matches printer tape spec
    if png_tape_spec.width_dots != printer_tape_spec.width_dots {
        return Err(format!(
            "Tape specification mismatch: PNG expects {}mm tape ({}px width), but printer has {}mm tape ({}px width)",
            png_tape_spec.width_mm, png_tape_spec.width_dots,
            printer_tape_spec.width_mm, printer_tape_spec.width_dots
        ).into());
    }

    println!("Verified tape compatibility: {} mm", actual_tape_width);
    println!("Starting print...");

    // Create PrintableImage and print
    let printable = PrintableImage::from_png_data(png_data, printer_tape_spec)?;
    printer.print(&printable, args.continuous)?;

    Ok(())
}

fn handle_status_command(args: StatusArgs) -> Result<()> {
    let backend = backend::from_host(&args.host)?;
    let mut printer = Printer::new(backend);

    match printer.get_status() {
        Ok(status) => {
            status.print_status_info(args.verbose);
        }
        Err(e) => {
            println!("Error getting printer status: {}", e);
        }
    }

    Ok(())
}

fn handle_completion_command(args: CompletionArgs) -> Result<()> {
    match args.shell {
        clap_complete::Shell::Zsh => {
            // Generate dynamic completion script for zsh using CompleteEnv
            unsafe {
                std::env::set_var("COMPLETE", "zsh");
            }
            CompleteEnv::with_factory(Cli::command).complete();
        }
        _ => {
            // Generate static completion for other shells
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "ptouch", &mut io::stdout());
        }
    }
    Ok(())
}

/// Generate font completion candidates for shell completion
///
/// This function scans the system fonts and additional font paths to create completion
/// candidates for the `--font` option. Each candidate includes:
/// - Font family name (e.g., "Arial", "Noto Sans CJK JP")
/// - Help text with type info (Monospace/Proportional) and style count
/// - Unique serial number to prevent shell completion grouping
///
/// # Arguments
/// * `font_paths` - Additional font directories/files to scan beyond system fonts
///
/// Get font completion candidates with help text for shell completion
///
/// # Arguments
/// * `font_paths` - Vector of paths to search for fonts
///
/// # Returns
/// * Vector of completion candidates sorted alphabetically by font name
fn get_font_completions(font_paths: &[PathBuf]) -> Vec<CompletionCandidate> {
    let font_names = get_font_names(font_paths);

    font_names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let help_text = format!("Font #{}", index + 1);
            CompletionCandidate::new(name).help(Some(help_text.into()))
        })
        .collect()
}

fn font_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut completions = vec![];
    let Some(current_str) = current.to_str() else {
        return completions;
    };

    // Check if this completer should even run (to avoid duplicate options)
    // This is handled automatically by clap for non-repeatable arguments
    // For --font specifically, it should only appear once per command

    // Note: clap_complete may not call this for partially completed space-containing arguments
    // This appears to be a limitation of the current implementation

    // Debug to file since stderr might be captured by shell
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/ptouch_debug.log")
    {
        use std::io::Write;
        let _ = writeln!(
            file,
            "DEBUG: font completion called for '{}' at {}",
            current_str,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }

    // Get all available fonts
    let font_paths = vec![];
    let all_completions = get_font_completions(&font_paths);

    // Filter based on current input
    let unescaped_current = unescape_shell_string(current_str);

    for candidate in all_completions {
        let font_name = candidate.get_value().to_string_lossy();
        if current_str.is_empty() || font_name.starts_with(&unescaped_current) {
            completions.push(candidate);
        }
    }

    completions
}

fn main() -> Result<()> {
    // Check for dynamic completion first
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    match cli.command {
        Commands::Image(args) => handle_image_command(args)?,
        Commands::Print(args) => handle_print_command(args)?,
        Commands::Status(args) => handle_status_command(args)?,
        Commands::Completion(args) => handle_completion_command(args)?,
    }

    Ok(())
}
