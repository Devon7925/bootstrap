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

const AST_COMPILER_FUNCTION_ENTRY_SIZE: usize = 32;
const AST_COMPILER_FUNCTIONS_BASE_OFFSET: usize = 851_968;
const AST_COMPILER_MAX_FUNCTIONS: usize = 512;

fn ast_compiler_output_ptr(compiler: &CompilerInstance) -> i32 {
    let memory_size = compiler.memory_size_bytes();
    let reserved = AST_COMPILER_FUNCTIONS_BASE_OFFSET
        + AST_COMPILER_MAX_FUNCTIONS * AST_COMPILER_FUNCTION_ENTRY_SIZE;
    assert!(
        memory_size > reserved,
        "ast_compiler memory must exceed reserved layout"
    );
    (memory_size - reserved) as i32
}

#[test]
fn ast_compiler_compiler_bootstraps() {
    let (mut ast_compiler, ast_compiler_source) = prepare_ast_compiler_compiler();
    let ast_compiler_output_address = ast_compiler_output_ptr(&ast_compiler);
    assert!(
        ast_compiler_source.len() < ast_compiler_output_address as usize,
        "ast_compiler source must not overlap output buffer",
    );

    // Compile the ast_compiler source with the ast_compiler compiler itself to produce stage2.
    let stage2_wasm = ast_compiler
        .compile_at(0, ast_compiler_output_address, &ast_compiler_source)
        .expect("ast_compiler should compile itself and produce stage2");

    // Ensure stage2 can be instantiated and compile the ast_compiler source again.
    let mut stage2 = CompilerInstance::new(&stage2_wasm);
    let stage2_output_address = ast_compiler_output_ptr(&stage2);
    assert!(
        ast_compiler_source.len() < stage2_output_address as usize,
        "stage2 output buffer must accommodate ast_compiler source",
    );
    let stage3_wasm = stage2
        .compile_at(0, stage2_output_address, &ast_compiler_source)
        .expect("stage2 should compile the ast_compiler source");

    assert_eq!(
        &stage2_wasm, &stage3_wasm,
        "stage2 and stage3 wasm outputs should be identical",
    );

    // Stage3 should be a fully functional compiler capable of compiling user
    // programs. Compile a small program and execute it to verify the output.
    let mut stage3 = CompilerInstance::new(&stage3_wasm);
    let stage3_output_address = ast_compiler_output_ptr(&stage3);
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
    assert!(
        sample_program.len() < stage3_output_address as usize,
        "sample program must fit in stage3 output buffer",
    );
    let sample_wasm = stage3
        .compile_at(0, stage3_output_address, sample_program)
        .expect("stage3 should compile sample program");

    assert_eq!(
        run_wasm_main(stage3.engine(), &sample_wasm),
        10,
        "compiled sample program should execute correctly",
    );
}
