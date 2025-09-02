use crate::element::{Column, Element, QrCode, Row, RowOptions, Text, TextOptions};

/// Parse layout script DSL into Element tree
///
/// Syntax (BNF):
/// - {ROW} := {COLUMN} ("+" {COLUMN})*
/// - {COLUMN} := {ELEMENT}+
/// - {ELEMENT} :=  {BAR_ELEMENT} | {IMG_ELEMENT} | {QRC_ELEMENT} | {TXT_ELEMENT}
///
/// - {BAR_ELEMENT} := "bar:"{STRING}
/// - {IMG_ELEMENT} := "img:"{STRING}
/// - {QRC_ELEMENT} := "qrc:"{STRING}
/// - {TXT_ELEMENT} := ("txt:"{STRING} | {STRING})+
///
/// - Prefixes: "txt:", "qrc:", "bar:", "img:" (defaults to "txt:" if no prefix)
/// - "+" separates COLUMN, and layouts columns horizontally (creates ROW)
/// - Continuous text becomes a single text element.
/// - Creating Column or Row only when there are multiple elements to contain
///
/// Examples:
/// ["Happy", "Birthday"]
/// -> Text(Happy,Birthday)
///
/// ["Happy", "+", "Birthday"]
/// -> Row(Text(Happy),Text(Birthday))
///
/// ["Hello", "World", "+", "To", "You"]
/// -> Row(Text(Hello,World),Text(To,You))
///
/// ["Happy", "Birthday", "qrc:example.com", "+", "To", "You"]
/// -> Row(Column(Text(Happy,Birthday),QrCode(example.com)),Text(To,You))
///
pub fn parse_layout_script(
    script: &[String],
    text_options: &TextOptions,
    row_options: &RowOptions,
) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
    if script.is_empty() {
        return Err("Empty layout script".into());
    }

    let tokens: Vec<&str> = script.iter().map(|s| s.as_str()).collect();
    let mut tokenizer = Tokenizer::new(tokens, text_options, row_options);
    parse_row(&mut tokenizer)
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
}

/// Parse ROW := COLUMN ("+" COLUMN)*
fn parse_row(tokenizer: &mut Tokenizer) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
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

/// Parse COLUMN := ELEMENT+
fn parse_column(tokenizer: &mut Tokenizer) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
    let mut elements = Vec::new();

    while let Some(element) = parse_element(tokenizer)? {
        elements.push(element);
    }

    if elements.is_empty() {
        return Err("Empty column".into());
    }

    create_column_element(elements)
}

/// Parse ELEMENT := BAR_ELEMENT | IMG_ELEMENT | QRC_ELEMENT | TXT_ELEMENT
fn parse_element(
    tokenizer: &mut Tokenizer,
) -> Result<Option<Box<dyn Element>>, Box<dyn std::error::Error>> {
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
        } else if token == "+" {
            // Stop parsing elements when we hit "+"
            Ok(None)
        } else {
            // Parse TXT_ELEMENT
            parse_txt_element(tokenizer).map(Some)
        }
    } else {
        Ok(None)
    }
}

/// Parse TXT_ELEMENT := ("txt:" STRING | STRING)+
fn parse_txt_element(
    tokenizer: &mut Tokenizer,
) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
    let mut texts = Vec::new();

    while let Some(token) = tokenizer.peek() {
        // Stop if we hit a non-text element or separator
        if token.starts_with("bar:")
            || token.starts_with("img:")
            || token.starts_with("qrc:")
            || token == "+"
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
        return Err("Empty text element".into());
    }

    Ok(Box::new(Text::new(&texts, tokenizer.text_options.clone())?))
}

/// Create Row element or return single element if columns.len() == 1
fn create_row_element(
    columns: Vec<Box<dyn Element>>,
    row_options: RowOptions,
) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
    let mut columns = columns;
    match columns.len() {
        0 => Err("No columns found".into()),
        1 => Ok(columns.pop().unwrap()),
        _ => Ok(Box::new(Row::new(columns, row_options))),
    }
}

/// Create Column element or return single element if elements.len() == 1
fn create_column_element(
    elements: Vec<Box<dyn Element>>,
) -> Result<Box<dyn Element>, Box<dyn std::error::Error>> {
    let mut elements = elements;
    match elements.len() {
        0 => Err("No elements found".into()),
        1 => Ok(elements.pop().unwrap()),
        _ => Ok(Box::new(Column::new(elements, 5.0))),
    }
}
