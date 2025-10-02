pub mod ast;
pub mod codegen;
pub mod error;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod typeck;

use crate::codegen::wat::WatGenerator;
use crate::error::CompileError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::typeck::TypeChecker;

pub struct Compilation {
    wat: String,
}

impl Compilation {
    pub fn wat(&self) -> &str {
        &self.wat
    }

    pub fn into_wat(self) -> String {
        self.wat
    }

    pub fn to_wasm(&self) -> Result<Vec<u8>, CompileError> {
        wat::parse_str(&self.wat).map_err(|err| {
            CompileError::new(format!("failed to encode wasm from generated WAT: {err}"))
        })
    }

    pub fn into_wasm(self) -> Result<Vec<u8>, CompileError> {
        self.to_wasm()
    }
}

pub fn compile(source: &str) -> Result<Compilation, CompileError> {
    let tokens = Lexer::new(source).collect::<Result<Vec<_>, _>>()?;
    let mut parser = Parser::new(&tokens, source);
    let program = parser.parse_program()?;
    let typed_program = TypeChecker::new().check(program)?;
    let wat = WatGenerator::default().emit_program(&typed_program)?;
    Ok(Compilation { wat })
}

pub fn compile_to_wat(source: &str) -> Result<String, CompileError> {
    Ok(compile(source)?.into_wat())
}

pub fn compile_to_wasm(source: &str) -> Result<Vec<u8>, CompileError> {
    compile(source)?.into_wasm()
}
