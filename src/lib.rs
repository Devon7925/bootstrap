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

pub fn compile_to_wat(source: &str) -> Result<String, CompileError> {
    let tokens = Lexer::new(source).collect::<Result<Vec<_>, _>>()?;
    let mut parser = Parser::new(&tokens, source);
    let program = parser.parse_program()?;
    let typed_program = TypeChecker::new().check(program)?;
    let wat = WatGenerator::default().emit_program(&typed_program)?;
    Ok(wat)
}
