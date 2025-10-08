#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::try_compile_with_ast_compiler;

#[test]
fn program_requires_main() {
    let source = r#"
fn helper() -> i32 {
    1
}
"#;

    let error = try_compile_with_ast_compiler(source).expect_err("expected missing main error");
    assert!(error.produced_len <= 0);
}

#[test]
fn main_cannot_accept_parameters() {
    let source = r#"
fn main(value: i32) -> i32 {
    value
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("expected main with parameters to be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
#[ignore]
fn main_must_return_i32() {
    let source = r#"
fn main() -> bool {
    true
}
"#;

    let error = try_compile_with_ast_compiler(source).expect_err("expected main return type error");
    assert!(error.produced_len <= 0);
}

#[test]
fn main_function_name_must_be_unique() {
    let source = r#"
fn main() -> i32 {
    1
}

fn helper() -> i32 {
    2
}

fn main() -> i32 {
    3
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("programs should not allow multiple main functions");
    assert!(error.produced_len <= 0);
}
