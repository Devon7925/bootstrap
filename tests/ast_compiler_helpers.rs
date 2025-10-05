#![allow(dead_code)]

use std::fs;
use std::sync::OnceLock;

use bootstrap::{Target, compile};

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompileFailure, CompilerInstance};

const AST_COMPILER_SOURCE_PATH: &str = "compiler/ast_compiler.bp";

static AST_COMPILER_SOURCE: OnceLock<String> = OnceLock::new();
static AST_COMPILER_WASM: OnceLock<Vec<u8>> = OnceLock::new();

pub fn ast_compiler_source() -> &'static str {
    AST_COMPILER_SOURCE
        .get_or_init(|| {
            fs::read_to_string(AST_COMPILER_SOURCE_PATH)
                .unwrap_or_else(|err| panic!("failed to load ast compiler source: {err}"))
        })
        .as_str()
}

pub fn ast_compiler_wasm() -> &'static [u8] {
    AST_COMPILER_WASM
        .get_or_init(|| {
            compile(ast_compiler_source(), Target::Wasm)
                .and_then(|compilation| compilation.into_wasm())
                .unwrap_or_else(|err| panic!("failed to compile ast compiler source: {err}"))
        })
        .as_slice()
}

pub fn try_compile_with_ast_compiler(source: &str) -> Result<Vec<u8>, CompileFailure> {
    let mut compiler = CompilerInstance::new(ast_compiler_wasm());
    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;
    compiler.compile_with_layout(&mut input_cursor, &mut output_cursor, source)
}

pub fn compile_with_ast_compiler(source: &str) -> Vec<u8> {
    try_compile_with_ast_compiler(source)
        .unwrap_or_else(|err| panic!("ast compiler failed to compile source: {err:?}"))
}
