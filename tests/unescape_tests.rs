use ptouch::unescape_shell_string;

#[test]
fn test_unescape_shell_string_no_escaping() {
    assert_eq!(unescape_shell_string("hello"), "hello");
    assert_eq!(unescape_shell_string("hello world"), "hello world");
    assert_eq!(unescape_shell_string("123"), "123");
    assert_eq!(unescape_shell_string(""), "");
}

#[test]
fn test_unescape_shell_string_backslash_escapes() {
    assert_eq!(unescape_shell_string(r"hello\ world"), "hello world");
    assert_eq!(unescape_shell_string(r"hello\tworld"), r"hello\tworld");
    assert_eq!(unescape_shell_string(r"hello\nworld"), r"hello\nworld");
    assert_eq!(unescape_shell_string(r"hello\\world"), r"hello\world");
    assert_eq!(unescape_shell_string(r"hello\'world"), "hello'world");
    assert_eq!(unescape_shell_string(r#"hello\"world"#), r#"hello"world"#);
}

#[test]
fn test_unescape_shell_string_trailing_backslash() {
    assert_eq!(unescape_shell_string(r"hello\"), r"hello\");
    assert_eq!(unescape_shell_string(r"\"), r"\");
}

#[test]
fn test_unescape_shell_string_unknown_escapes() {
    assert_eq!(unescape_shell_string(r"hello\x"), r"hello\x");
    assert_eq!(unescape_shell_string(r"\a\b\c"), r"\a\b\c");
}

#[test]
fn test_unescape_shell_string_single_quotes() {
    assert_eq!(unescape_shell_string("'hello world'"), "hello world");
    assert_eq!(unescape_shell_string("'hello'"), "hello");
    assert_eq!(unescape_shell_string("''"), "");
    assert_eq!(
        unescape_shell_string("'hello world' text"),
        "hello world text"
    );
    assert_eq!(
        unescape_shell_string("before'hello'after"),
        "beforehelloafter"
    );
}

#[test]
fn test_unescape_shell_string_single_quotes_literal() {
    assert_eq!(unescape_shell_string(r"'hello\world'"), r"hello\world");
    assert_eq!(unescape_shell_string("'hello\"world'"), "hello\"world");
    assert_eq!(unescape_shell_string("'$VAR'"), "$VAR");
}

#[test]
fn test_unescape_shell_string_unclosed_single_quotes() {
    assert_eq!(unescape_shell_string("'hello"), "hello");
    assert_eq!(unescape_shell_string("'hello world"), "hello world");
}

#[test]
fn test_unescape_shell_string_double_quotes() {
    assert_eq!(unescape_shell_string("\"hello world\""), "hello world");
    assert_eq!(unescape_shell_string("\"hello\""), "hello");
    assert_eq!(unescape_shell_string("\"\""), "");
    assert_eq!(
        unescape_shell_string("\"hello world\" text"),
        "hello world text"
    );
    assert_eq!(
        unescape_shell_string("before\"hello\"after"),
        "beforehelloafter"
    );
}

#[test]
fn test_unescape_shell_string_double_quotes_escapes() {
    assert_eq!(unescape_shell_string(r#""hello\"world""#), "hello\"world");
    assert_eq!(unescape_shell_string(r#""hello\\world""#), r"hello\world");
    assert_eq!(unescape_shell_string("\"hello$world\""), "hello$world");
    assert_eq!(unescape_shell_string("\"hello`world\""), "hello`world");
    assert_eq!(unescape_shell_string("\"hello\nworld\""), "hello\nworld");
}

#[test]
fn test_unescape_shell_string_double_quotes_unknown_escapes() {
    assert_eq!(unescape_shell_string(r#""hello\x""#), r"hello\x");
    assert_eq!(unescape_shell_string(r#""hello\t""#), r"hello\t");
}

#[test]
fn test_unescape_shell_string_double_quotes_trailing_backslash() {
    // This input is "hello\" - the backslash escapes the quote, so it continues
    let input = "\"hello\\\"";
    let result = unescape_shell_string(input);
    assert_eq!(result, "hello\"");
}

#[test]
fn test_unescape_shell_string_unclosed_double_quotes() {
    assert_eq!(unescape_shell_string("\"hello"), "hello");
    assert_eq!(unescape_shell_string("\"hello world"), "hello world");
    assert_eq!(unescape_shell_string(r#""hello\"#), r"hello\");
}

#[test]
fn test_unescape_shell_string_mixed_quotes() {
    assert_eq!(unescape_shell_string("'hello' \"world\""), "hello world");
    assert_eq!(unescape_shell_string("\"hello 'world'\""), "hello 'world'");
    assert_eq!(
        unescape_shell_string("'hello \"world\"'"),
        "hello \"world\""
    );
}

#[test]
fn test_unescape_shell_string_complex_cases() {
    assert_eq!(
        unescape_shell_string(r#"file\ name.txt 'quoted text' "escaped\"quote""#),
        "file name.txt quoted text escaped\"quote"
    );
    assert_eq!(
        unescape_shell_string(r#"prefix\ 'single' "double" suffix"#),
        "prefix single double suffix"
    );
}

#[test]
fn test_unescape_shell_string_edge_cases() {
    assert_eq!(unescape_shell_string("'"), "");
    assert_eq!(unescape_shell_string("\""), "");
    assert_eq!(unescape_shell_string("'\""), "\"");
    assert_eq!(unescape_shell_string("\"'"), "'");
}
