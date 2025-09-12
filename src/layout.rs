use crate::Result;
use crate::element::{Column, Element, Gap, Overlay, QrCode, Row, RowOptions, Text, TextOptions};

/// Parse layout script DSL into Element tree
///
/// Syntax (BNF):
/// - {OVERLAY} := {LAYER} ("/" {LAYER})*
/// - {LAYER}   := {ROW}                     // pseudo (identity transformation)
/// - {ROW}     := {COLUMN} ("+" {COLUMN})*
/// - {COLUMN}  := {FACTOR}+
/// - {FACTOR}  := {ELEMENT} | "[" {ROW} "]"
/// - {ELEMENT} := {BAR} | {IMG} | {QRC} | {GAP} | {BOX} | {TXT}
///
/// Note: LAYER is omitted in implementation and ROW is directly reduced to OVERLAY.
///
/// - {BAR} := "bar:"{STRING}
/// - {IMG} := "img:"{STRING}
/// - {QRC} := "qrc:"{STRING}
/// - {GAP} := "gap:"{SPEC}
/// - {BOX} := "box:"{SPEC}
/// - {TXT} := ("txt:"{STRING} | {STRING})+
///
/// - Prefixes: "txt:", "qrc:", "bar:", "img:" (defaults to "txt:" if no prefix)
/// - "+" separates COLUMN, and layouts columns horizontally (creates ROW)
/// - Continuous text becomes a single text element.
/// - Creating Column or Row only when there are multiple elements to contain
///
/// Examples:
/// Happy Birthday
/// -> Text(Happy,Birthday)
///
/// Happy + Birthday
/// -> Row(Text(Happy),Text(Birthday))
///
/// Hello World + To You
/// -> Row(Text(Hello,World),Text(To,You))
///
/// Happy Birthday qrc:example.com + To You
/// -> Row(Column(Text(Happy,Birthday),QrCode(example.com)),Text(To,You))
///
/// Long-Title-On-Top [ qrc:http://example.com + nom@example.com ]
/// -> Column(Text(Long-Title-On-Top),Row(QrCode(http://example.com),Text(nom@example.com)))
///
pub fn parse_layout_script(
    script: &[String],
    text_options: &TextOptions,
    row_options: &RowOptions,
) -> Result<Box<dyn Element>> {
    if script.is_empty() {
        return Err("Empty layout script".into());
    }

    let tokens: Vec<&str> = script.iter().map(|s| s.as_str()).collect();
    let mut tokenizer = Tokenizer::new(tokens, text_options, row_options);
    let overlay = parse_overlay(&mut tokenizer)?;

    // Check for unconsumed tokens (like unmatched ']')
    if !tokenizer.is_empty() {
        return Err(format!("Syntax error at {}", tokenizer.position_info()).into());
    }

    Ok(overlay)
}

/// Tokenizer for layout script DSL
struct Tokenizer<'a> {
    tokens: Vec<&'a str>,
    position: usize,
    text_options: &'a TextOptions,
    row_options: &'a RowOptions,
}

impl<'a> Tokenizer<'a> {
    fn new(
        tokens: Vec<&'a str>,
        text_options: &'a TextOptions,
        row_options: &'a RowOptions,
    ) -> Self {
        Self {
            tokens,
            position: 0,
            text_options,
            row_options,
        }
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.position).copied()
    }

    fn consume(&mut self) -> Option<&str> {
        if self.position < self.tokens.len() {
            let token = self.tokens[self.position];
            self.position += 1;
            Some(token)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &str) -> bool {
        if self.peek() == Some(expected) {
            self.consume();
            true
        } else {
            false
        }
    }

    fn is_empty(&self) -> bool {
        self.position >= self.tokens.len()
    }

    fn position_info(&self) -> String {
        if self.is_empty() {
            "End of input".to_string()
        } else {
            format!(
                "Position {}, Token: {}",
                self.position + 1,
                self.peek().unwrap()
            )
        }
    }
}

/// Parse OVERLAY := ROW ("/" ROW)*
fn parse_overlay(tokenizer: &mut Tokenizer) -> Result<Box<dyn Element>> {
    let mut rows = Vec::new();

    // Parse first row
    let row = parse_row(tokenizer)?;
    rows.push(row);

    // Parse additional rows separated by "/"
    while tokenizer.expect("/") {
        let row = parse_row(tokenizer)?;
        rows.push(row);
    }

    create_overlay_element(rows)
}

/// Parse ROW := COLUMN ("+" COLUMN)*
fn parse_row(tokenizer: &mut Tokenizer) -> Result<Box<dyn Element>> {
    let mut columns = Vec::new();

    // Parse first column
    let column = parse_column(tokenizer)?;
    columns.push(column);

    // Parse additional columns separated by "+"
    while tokenizer.expect("+") {
        let column = parse_column(tokenizer)?;
        columns.push(column);
    }

    create_row_element(columns, tokenizer.row_options.clone())
}

/// Parse COLUMN := FACTOR+
fn parse_column(tokenizer: &mut Tokenizer) -> Result<Box<dyn Element>> {
    let mut factors = Vec::new();

    while let Some(factor) = parse_factor(tokenizer)? {
        factors.push(factor);
    }

    if factors.is_empty() {
        return Err(format!("No COLUMN at {}", tokenizer.position_info()).into());
    }

    create_column_element(factors)
}

/// Parse FACTOR := ELEMENT | "[" ROW "]"
fn parse_factor(tokenizer: &mut Tokenizer) -> Result<Option<Box<dyn Element>>> {
    if let Some(token) = tokenizer.peek() {
        if token == "[" {
            tokenizer.consume(); // consume "["
            let row = parse_row(tokenizer)?;
            if !tokenizer.expect("]") {
                return Err(format!("Expected ']' at {}", tokenizer.position_info()).into());
            }
            Ok(Some(row))
        } else {
            parse_element(tokenizer)
        }
    } else {
        Ok(None)
    }
}

/// Parse ELEMENT := BAR_ELEMENT | IMG_ELEMENT | QRC_ELEMENT | GAP_ELEMENT | BOX_ELEMENT | TXT_ELEMENT
fn parse_element(tokenizer: &mut Tokenizer) -> Result<Option<Box<dyn Element>>> {
    if let Some(token) = tokenizer.peek() {
        if let Some(content) = token.strip_prefix("bar:") {
            let content = content.to_string();
            tokenizer.consume();
            Err(format!("Barcode not yet implemented: {}", content).into())
        } else if let Some(content) = token.strip_prefix("img:") {
            let content = content.to_string();
            tokenizer.consume();
            Err(format!("Image not yet implemented: {}", content).into())
        } else if let Some(content) = token.strip_prefix("qrc:") {
            let content = content.to_string();
            tokenizer.consume();
            let qr_code = QrCode::new(content)?;
            Ok(Some(Box::new(qr_code)))
        } else if let Some(content) = token.strip_prefix("gap:") {
            let content = content.to_string();
            tokenizer.consume();
            let gap = Gap::parse(&content, false)?;
            Ok(Some(Box::new(gap)))
        } else if let Some(content) = token.strip_prefix("box:") {
            let content = content.to_string();
            tokenizer.consume();
            let box_element = Gap::parse(&content, true)?;
            Ok(Some(Box::new(box_element)))
        } else {
            // Parse TXT_ELEMENT (handles stopping conditions internally)
            parse_txt_element(tokenizer)
        }
    } else {
        Ok(None)
    }
}

/// Parse TXT_ELEMENT := ("txt:" STRING | STRING)+
fn parse_txt_element(tokenizer: &mut Tokenizer) -> Result<Option<Box<dyn Element>>> {
    let mut texts = Vec::new();

    while let Some(token) = tokenizer.peek() {
        // Stop if we hit a non-text element or separator or brackets
        if token.starts_with("bar:")
            || token.starts_with("img:")
            || token.starts_with("qrc:")
            || token.starts_with("gap:")
            || token.starts_with("box:")
            || token == "+"
            || token == "/"
            || token == "["
            || token == "]"
        {
            break;
        }

        let txt = tokenizer.consume().unwrap();
        if let Some(content) = txt.strip_prefix("txt:") {
            texts.push(content.to_string());
        } else {
            texts.push(txt.to_string());
        }
    }

    if texts.is_empty() {
        return Ok(None);
    }

    Ok(Some(Box::new(Text::new(
        &texts,
        tokenizer.text_options.clone(),
    )?)))
}

/// Create Row element or return single element if columns.len() == 1
fn create_row_element(
    columns: Vec<Box<dyn Element>>,
    row_options: RowOptions,
) -> Result<Box<dyn Element>> {
    let mut columns = columns;
    match columns.len() {
        0 => Err("No columns found".into()),
        1 => Ok(columns.pop().unwrap()),
        _ => Ok(Box::new(Row::new(columns, row_options))),
    }
}

/// Create Column element or return single element if elements.len() == 1
fn create_column_element(elements: Vec<Box<dyn Element>>) -> Result<Box<dyn Element>> {
    let mut elements = elements;
    match elements.len() {
        0 => Err("No elements found".into()),
        1 => Ok(elements.pop().unwrap()),
        _ => Ok(Box::new(Column::new(elements, 20.0))), // FIXME: 360DPI
    }
}

/// Create Overlay element or return single element if elements.len() == 1
fn create_overlay_element(elements: Vec<Box<dyn Element>>) -> Result<Box<dyn Element>> {
    let mut elements = elements;
    match elements.len() {
        0 => Err("No rows found".into()),
        1 => Ok(elements.pop().unwrap()),
        _ => Ok(Box::new(Overlay::new(elements))),
    }
}
