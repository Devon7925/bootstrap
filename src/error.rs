use std::error::Error;
use std::fmt;

use crate::span::Span;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Option<Span>,
}

impl CompileError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "error at {}: {}", span, self.message)
        } else {
            write!(f, "error: {}", self.message)
        }
    }
}

impl Error for CompileError {}

#[derive(Debug, Clone)]
pub struct LexerError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lexer error at {}: {}", self.span, self.message)
    }
}

impl Error for LexerError {}

impl From<LexerError> for CompileError {
    fn from(value: LexerError) -> Self {
        CompileError {
            message: value.message,
            span: Some(value.span),
        }
    }
}
