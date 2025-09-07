use fontdb::Database;
use ptouch::element::{RowOptions, TextOptions, VerticalAlign};
use ptouch::layout::parse_layout_script;
use std::sync::Arc;

fn create_test_options() -> TextOptions {
    let mut fontdb = Database::new();
    fontdb.load_fonts_dir("attic/fonts");
    fontdb.load_system_fonts();

    TextOptions {
        font_name: "Noto Sans JP".to_string(),
        font_size: 24,
        line_height: 30,
        fontdb: Arc::new(fontdb),
    }
}

fn create_test_row_options() -> RowOptions {
    RowOptions {
        align: VerticalAlign::default(),
        padding: 5.0,
    }
}

fn script_from_str(input: &str) -> Vec<String> {
    input.split_whitespace().map(|s| s.to_string()).collect()
}

fn parse_test_script(input: &str) -> ptouch::Result<Box<dyn ptouch::element::Element>> {
    let script = script_from_str(input);
    let options = create_test_options();
    let row_options = create_test_row_options();
    parse_layout_script(&script, &options, &row_options)
}

fn assert_parse_result(input: &str, expected: &str) {
    let result = parse_test_script(input);
    assert!(
        result.is_ok(),
        "Failed to parse '{}': {}",
        input,
        result.err().unwrap()
    );
    let element = result.unwrap();
    assert_eq!(
        format!("{}", element),
        expected,
        "Parsing '{}' produced wrong structure",
        input
    );
}

#[test]
fn test_single_text() {
    assert_parse_result("Happy Birthday", "Text(Happy,Birthday)");
}

#[test]
fn test_horizontal_layout() {
    assert_parse_result("Happy + Birthday", "Row(Text(Happy),Text(Birthday))");
}

#[test]
fn test_mixed_elements() {
    assert_parse_result(
        "Hello World + To You",
        "Row(Text(Hello,World),Text(To,You))",
    );
}

#[test]
fn test_qr_code() {
    assert_parse_result("qrc:example.com", "QrCode(example.com)");
}

// Tests for nested bracket syntax
#[test]
fn test_simple_nested_layout() {
    // [A + B] C -> Column(Row(A,B),C)
    assert_parse_result("[ A + B ] C", "Column(Row(Text(A),Text(B)),Text(C))");
}

#[test]
fn test_multiple_nested_layout() {
    // [A + B] [C + D] -> Column(Row(A,B),Row(C,D))
    assert_parse_result(
        "[ A + B ] [ C + D ]",
        "Column(Row(Text(A),Text(B)),Row(Text(C),Text(D)))",
    );
}

#[test]
fn test_nested_with_horizontal_layout() {
    // [A + B] + [C + D] -> Row(Row(A,B),Row(C,D))
    assert_parse_result(
        "[ A + B ] + [ C + D ]",
        "Row(Row(Text(A),Text(B)),Row(Text(C),Text(D)))",
    );
}

#[test]
fn test_nested_with_qr_code() {
    // Title [qrc:example.com + contact@example.com] -> Column(Text(Title),Row(QrCode,Text))
    assert_parse_result(
        "Title [ qrc:example.com + contact@example.com ]",
        "Column(Text(Title),Row(QrCode(example.com),Text(contact@example.com)))",
    );
}

// Tests for error cases
#[test]
fn test_unmatched_closing_bracket() {
    // A ] -> Should error
    let result = parse_test_script("A ]");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("Syntax error at Position 2, Token: ]"));
}

#[test]
fn test_unmatched_opening_bracket() {
    // [ A -> Should error (missing ])
    let result = parse_test_script("[ A");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("Expected ']' at End of input"));
}

#[test]
fn test_empty_brackets() {
    // [ ] -> Should error (empty column)
    let result = parse_test_script("[ ]");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("No COLUMN"));
}

#[test]
fn test_nested_bracket_mismatch() {
    // [ A [ B ] -> Should error (missing ] for first [)
    let result = parse_test_script("[ A [ B ]");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("Expected ']' at End of input"));
}

#[test]
fn test_extra_closing_bracket() {
    // [ A + B ] ] -> Should error (extra ])
    let result = parse_test_script("[ A + B ] ]");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("Syntax error at Position 6, Token: ]"));
}

#[test]
fn test_empty_script() {
    // Empty script -> Should error
    let result = parse_test_script("");
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("Empty layout script"));
}
