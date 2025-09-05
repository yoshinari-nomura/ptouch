pub mod backend;
pub mod element;
pub mod label;
pub mod layout;
pub mod printable_image;
pub mod printer;
pub mod raster_command;
pub mod status;
pub mod tape;

use fontdb::Database;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Remove shell escaping from a string
///
/// Handles common escape sequences: `\<space>`, `\<tab>`, `\<newline>`, `\\`, `\'`, `\"`
/// Also handles quoted strings: `'text'` and `"text"`
///
/// # Examples
///
/// ```
/// use ptouch::unescape_shell_string;
///
/// // Basic escaping
/// assert_eq!(unescape_shell_string(r"hello\ world"), "hello world");
///
/// // Single quotes
/// assert_eq!(unescape_shell_string("'hello world'"), "hello world");
///
/// // Double quotes with escaping
/// assert_eq!(unescape_shell_string(r#""hello\"world""#), "hello\"world");
///
/// // Complex case
/// assert_eq!(
///     unescape_shell_string(r#"file\ name.txt 'quoted text' "escaped\"quote""#),
///     "file name.txt quoted text escaped\"quote"
/// );
/// ```
pub fn unescape_shell_string(s: &str) -> String {
    type CharIter<'a> = std::iter::Peekable<std::str::Chars<'a>>;

    fn backslash(chars: &mut CharIter, result: &mut String, escapes: &str) {
        if let Some(&next_ch) = chars.peek()
            && escapes.contains(next_ch)
        {
            result.push(chars.next().unwrap());
        } else {
            result.push('\\');
        }
    }

    fn single_str(chars: &mut CharIter, result: &mut String) {
        #[allow(clippy::while_let_on_iterator)]
        while let Some(ch) = chars.next() {
            if ch == '\'' {
                break;
            }
            result.push(ch);
        }
    }

    fn double_str(chars: &mut CharIter, result: &mut String) {
        while let Some(ch) = chars.next() {
            if ch == '"' {
                break;
            }
            if ch == '\\' {
                backslash(chars, result, "\"\\$`\n");
            } else {
                result.push(ch);
            }
        }
    }

    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => backslash(&mut chars, &mut result, " \t\n\\'\""),
            '\'' => single_str(&mut chars, &mut result),
            '"' => double_str(&mut chars, &mut result),
            _ => result.push(ch),
        }
    }
    result
}

/// Load fontdb with system fonts and additional font paths
///
/// # Arguments
/// * `font_paths` - Additional font directories/files to load
///
/// # Returns
/// * Result containing Arc<Database> or error
pub fn load_fontdb_with_paths(font_paths: &[PathBuf]) -> Result<std::sync::Arc<Database>> {
    let mut fontdb = Database::new();
    fontdb.load_system_fonts();

    // Load fonts from additional paths
    for path in font_paths {
        if path.is_dir() {
            fontdb.load_fonts_dir(path);
        } else if path.is_file() {
            fontdb.load_font_file(path)?;
        } else {
            eprintln!("Warning: Font path does not exist: {}", path.display());
        }
    }

    Ok(std::sync::Arc::new(fontdb))
}

/// Get available font names from font paths
///
/// # Arguments
/// * `font_paths` - Vector of paths to search for fonts
///
/// # Returns
/// * Vector of font names sorted alphabetically
pub fn get_font_names(font_paths: &[PathBuf]) -> Vec<String> {
    let fontdb = match load_fontdb_with_paths(font_paths) {
        Ok(db) => db,
        Err(_) => return vec![],
    };

    let mut font_names = std::collections::HashSet::new();

    // Collect unique font family names
    for face in fontdb.faces() {
        for (family_name, _) in &face.families {
            font_names.insert(family_name.clone());
        }
    }

    let mut names: Vec<String> = font_names.into_iter().collect();
    names.sort();
    names
}
