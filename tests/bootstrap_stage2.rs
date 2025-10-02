use std::fs;

use bootstrap::compile;
use bootstrap::lexer::Lexer;
use bootstrap::parser::Parser;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompileFailure, CompilerInstance};

#[test]
fn stage1_compiler_identifies_forward_reference_blocker() {
    let stage1_source =
        fs::read_to_string("examples/stage1_minimal.bp").expect("failed to load stage1 source");

    let stage1_compilation = compile(&stage1_source).expect("failed to compile stage1 source");
    let stage1_wasm = stage1_compilation
        .to_wasm()
        .expect("failed to encode stage1 wasm");

    let mut stage1 = CompilerInstance::new(stage1_wasm.as_slice());

    // Compile the stage1 source with the stage1 compiler itself to produce stage2.
    let result = stage1.compile_at(0, 131072, &stage1_source);
    match result {
        Ok(_) => panic!("stage1 unexpectedly compiled itself without resolving forward references"),
        Err(CompileFailure {
            produced_len,
            functions,
            instr_offset,
        }) => {
            assert_eq!(produced_len, -1);
            assert_eq!(instr_offset, 0);

            let tokens = Lexer::new(&stage1_source)
                .collect::<Result<Vec<_>, _>>()
                .expect("lex stage1 source");
            let mut parser = Parser::new(&tokens, &stage1_source);
            let program = parser.parse_program().expect("parse stage1 source");
            let total_functions = program.functions.len() as i32;

            assert!(functions > 0, "expected to register at least one function");
            assert!(
                functions < total_functions,
                "expected failure before all functions were processed"
            );
        }
    }
}
