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

#[test]
fn test_single_text() {
    let script = vec!["Happy".to_string(), "Birthday".to_string()];
    let options = create_test_options();
    let row_options = create_test_row_options();
    let result = parse_layout_script(&script, &options, &row_options);
    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_horizontal_layout() {
    let script = vec!["Happy".to_string(), "+".to_string(), "Birthday".to_string()];
    let options = create_test_options();
    let row_options = create_test_row_options();
    let result = parse_layout_script(&script, &options, &row_options);
    assert!(result.is_ok());
}

#[test]
fn test_mixed_elements() {
    let script = vec![
        "Hello".to_string(),
        "World".to_string(),
        "+".to_string(),
        "To".to_string(),
        "You".to_string(),
    ];
    let options = create_test_options();
    let row_options = create_test_row_options();
    let result = parse_layout_script(&script, &options, &row_options);
    assert!(result.is_ok());
}

#[test]
fn test_qr_code() {
    let script = vec!["qrc:example.com".to_string()];
    let options = create_test_options();
    let row_options = create_test_row_options();
    let result = parse_layout_script(&script, &options, &row_options);
    assert!(result.is_ok());
}
