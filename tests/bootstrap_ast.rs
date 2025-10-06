#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompilerInstance, run_wasm_main};

#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

use ast_compiler_helpers::{ast_compiler_source, ast_compiler_wasm};

fn prepare_ast_compiler_compiler() -> (CompilerInstance, &'static str) {
    (
        CompilerInstance::new(ast_compiler_wasm()),
        ast_compiler_source(),
    )
}

#[test]
fn ast_compiler_compiler_bootstraps() {
    let (mut ast_compiler, ast_compiler_source) = prepare_ast_compiler_compiler();

    // Compile the ast_compiler source with the ast_compiler compiler itself to produce stage2.
    let stage2_wasm = ast_compiler
        .compile_at(0, ast_compiler_source.len() as i32, &ast_compiler_source)
        .expect("ast_compiler should compile itself and produce stage2");

    // Ensure stage2 can be instantiated and compile the ast_compiler source again.
    let mut stage2 = CompilerInstance::new(&stage2_wasm);
    let stage3_wasm = stage2
        .compile_at(0, ast_compiler_source.len() as i32, &ast_compiler_source)
        .expect("stage2 should compile the ast_compiler source");

    assert_eq!(
        &stage2_wasm, &stage3_wasm,
        "stage2 and stage3 wasm outputs should be identical",
    );

    // Stage3 should be a fully functional compiler capable of compiling user
    // programs. Compile a small program and execute it to verify the output.
    let mut stage3 = CompilerInstance::new(&stage3_wasm);
    let sample_program = r#"
fn main() -> i32 {
    let mut total: i32 = 0;
    let mut idx: i32 = 0;
    loop {
        if idx >= 5 {
            break;
        };
        total = total + idx;
        idx = idx + 1;
    };
    total
}
"#;
    let sample_wasm = stage3
        .compile_at(0, sample_program.len() as i32, sample_program)
        .expect("stage3 should compile sample program");

    assert_eq!(
        run_wasm_main(stage3.engine(), &sample_wasm),
        10,
        "compiled sample program should execute correctly",
    );
}
