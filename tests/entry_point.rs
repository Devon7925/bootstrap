use bootstrap::compile;

#[test]
fn program_requires_main() {
    let source = r#"
fn helper() -> i32 {
    1
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected missing main error"),
        Err(err) => err,
    };
    assert!(
        error
            .message
            .contains("program must define `fn main() -> i32`"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn main_cannot_accept_parameters() {
    let source = r#"
fn main(value: i32) -> i32 {
    value
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected main parameter error"),
        Err(err) => err,
    };
    assert!(
        error.message.contains("`main` cannot take parameters"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn main_must_return_i32() {
    let source = r#"
fn main() -> bool {
    true
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected main return type error"),
        Err(err) => err,
    };
    assert!(
        error.message.contains("`main` must return `i32`"),
        "unexpected error message: {}",
        error.message
    );
}
