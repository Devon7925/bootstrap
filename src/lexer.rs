use crate::error::LexerError;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Fn,
    Let,
    Mut,
    Return,
    If,
    Else,
    Loop,
    While,
    Break,
    Continue,
    True,
    False,
    Struct,
    Enum,
    Identifier(String),
    IntLiteral {
        value: i64,
        suffix: Option<IntLiteralSuffix>,
    },
    FloatLiteral {
        value: f64,
        suffix: Option<FloatLiteralSuffix>,
    },
    Colon,
    Semicolon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    DoubleEquals,
    NotEquals,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    ShiftLeft,
    ShiftRight,
    Ampersand,
    Pipe,
    Caret,
    AndAnd,
    OrOr,
    Assign,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntLiteralSuffix {
    I32,
    I64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatLiteralSuffix {
    F32,
    F64,
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    index: usize,
    finished: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars = source.char_indices().collect();
        Self {
            source,
            chars,
            index: 0,
            finished: false,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).map(|&(_, ch)| ch)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).map(|&(_, ch)| ch)
    }

    fn current_pos(&self) -> usize {
        self.chars
            .get(self.index)
            .map(|&(pos, _)| pos)
            .unwrap_or_else(|| self.source.len())
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        if let Some(&(pos, ch)) = self.chars.get(self.index) {
            self.index += 1;
            Some((pos, ch))
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let ch = match self.peek() {
                Some(c) => c,
                None => return,
            };

            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            if ch == '/' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            break;
        }
    }

    fn lex_number(&mut self, start_pos: usize, first_char: char) -> Result<Token, LexerError> {
        let mut buffer = String::new();
        buffer.push(first_char);
        let mut literal_end = start_pos + first_char.len_utf8();
        let mut span_end = literal_end;
        let mut seen_dot = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                literal_end = self.current_pos() + ch.len_utf8();
                span_end = literal_end;
                buffer.push(ch);
                self.advance();
            } else if ch == '.' && !seen_dot {
                seen_dot = true;
                literal_end = self.current_pos() + ch.len_utf8();
                span_end = literal_end;
                buffer.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        #[derive(Clone, Copy)]
        enum NumericSuffix {
            Int(IntLiteralSuffix),
            Float(FloatLiteralSuffix),
        }

        let mut suffix_kind: Option<NumericSuffix> = None;
        let mut suffix_span: Option<Span> = None;
        if let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() {
                let suffix_start = self.current_pos();
                let mut suffix_buf = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_alphanumeric() {
                        span_end = self.current_pos() + ch.len_utf8();
                        suffix_buf.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let parsed_span = Span::new(suffix_start, span_end);
                suffix_span = Some(parsed_span);
                suffix_kind = Some(match suffix_buf.as_str() {
                    "i32" => NumericSuffix::Int(IntLiteralSuffix::I32),
                    "i64" => NumericSuffix::Int(IntLiteralSuffix::I64),
                    "f32" => NumericSuffix::Float(FloatLiteralSuffix::F32),
                    "f64" => NumericSuffix::Float(FloatLiteralSuffix::F64),
                    _ => {
                        return Err(LexerError {
                            message: format!("invalid numeric suffix `{suffix_buf}`"),
                            span: parsed_span,
                        });
                    }
                });
            }
        }

        let total_end = span_end;
        let literal_span = Span::new(start_pos, literal_end);

        if seen_dot {
            if let Some(NumericSuffix::Int(_)) = suffix_kind {
                return Err(LexerError {
                    message: "floating literal cannot use integer suffix".into(),
                    span: suffix_span.expect("suffix span must exist when suffix parsed"),
                });
            }
            let value = buffer.parse::<f64>().map_err(|_| LexerError {
                message: format!("invalid float literal `{buffer}`"),
                span: literal_span,
            })?;
            let suffix = match suffix_kind {
                Some(NumericSuffix::Float(s)) => Some(s),
                Some(NumericSuffix::Int(_)) => unreachable!(),
                None => None,
            };
            Ok(Token {
                kind: TokenKind::FloatLiteral { value, suffix },
                span: Span::new(start_pos, total_end),
            })
        } else {
            match suffix_kind {
                Some(NumericSuffix::Float(s)) => {
                    let value = buffer.parse::<f64>().map_err(|_| LexerError {
                        message: format!("invalid float literal `{buffer}`"),
                        span: literal_span,
                    })?;
                    Ok(Token {
                        kind: TokenKind::FloatLiteral {
                            value,
                            suffix: Some(s),
                        },
                        span: Span::new(start_pos, total_end),
                    })
                }
                Some(NumericSuffix::Int(s)) => {
                    let value = buffer.parse::<i64>().map_err(|_| LexerError {
                        message: format!("invalid integer literal `{buffer}`"),
                        span: literal_span,
                    })?;
                    Ok(Token {
                        kind: TokenKind::IntLiteral {
                            value,
                            suffix: Some(s),
                        },
                        span: Span::new(start_pos, total_end),
                    })
                }
                None => {
                    let value = buffer.parse::<i64>().map_err(|_| LexerError {
                        message: format!("invalid integer literal `{buffer}`"),
                        span: literal_span,
                    })?;
                    Ok(Token {
                        kind: TokenKind::IntLiteral {
                            value,
                            suffix: None,
                        },
                        span: Span::new(start_pos, total_end),
                    })
                }
            }
        }
    }

    fn lex_identifier(&mut self, start_pos: usize, first_char: char) -> Token {
        let mut ident = String::new();
        ident.push(first_char);
        let mut end_pos = start_pos + first_char.len_utf8();

        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                end_pos = self.current_pos() + ch.len_utf8();
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match ident.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "loop" => TokenKind::Loop,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            _ => TokenKind::Identifier(ident),
        };

        Token {
            kind,
            span: Span::new(start_pos, end_pos),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        self.skip_whitespace_and_comments();

        let (start_pos, ch) = match self.advance() {
            Some(pair) => pair,
            None => {
                self.finished = true;
                return Some(Ok(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(self.source.len(), self.source.len()),
                }));
            }
        };

        let token_result = match ch {
            'a'..='z' | 'A'..='Z' | '_' => Ok(self.lex_identifier(start_pos, ch)),
            '0'..='9' => self.lex_number(start_pos, ch),
            '(' => Ok(Token {
                kind: TokenKind::LParen,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            ')' => Ok(Token {
                kind: TokenKind::RParen,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '{' => Ok(Token {
                kind: TokenKind::LBrace,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '}' => Ok(Token {
                kind: TokenKind::RBrace,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            ':' => Ok(Token {
                kind: TokenKind::Colon,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            ';' => Ok(Token {
                kind: TokenKind::Semicolon,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            ',' => Ok(Token {
                kind: TokenKind::Comma,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '+' => Ok(Token {
                kind: TokenKind::Plus,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '-' => {
                if let Some(next) = self.peek() {
                    if next == '>' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::Arrow,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::Minus,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::Minus,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '*' => Ok(Token {
                kind: TokenKind::Star,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '/' => Ok(Token {
                kind: TokenKind::Slash,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '%' => Ok(Token {
                kind: TokenKind::Percent,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            '=' => {
                if let Some(next) = self.peek() {
                    if next == '=' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::DoubleEquals,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::Assign,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::Assign,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '!' => {
                if let Some(next) = self.peek() {
                    if next == '=' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::NotEquals,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::Bang,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::Bang,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '<' => {
                if let Some(next) = self.peek() {
                    if next == '<' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::ShiftLeft,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else if next == '=' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::LessThanEqual,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::LessThan,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::LessThan,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '>' => {
                if let Some(next) = self.peek() {
                    if next == '>' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::ShiftRight,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else if next == '=' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::GreaterThanEqual,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::GreaterThan,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::GreaterThan,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '&' => {
                if let Some(next) = self.peek() {
                    if next == '&' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::AndAnd,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::Ampersand,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::Ampersand,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '|' => {
                if let Some(next) = self.peek() {
                    if next == '|' {
                        let end_pos = self.current_pos() + next.len_utf8();
                        self.advance();
                        Ok(Token {
                            kind: TokenKind::OrOr,
                            span: Span::new(start_pos, end_pos),
                        })
                    } else {
                        Ok(Token {
                            kind: TokenKind::Pipe,
                            span: Span::new(start_pos, start_pos + ch.len_utf8()),
                        })
                    }
                } else {
                    Ok(Token {
                        kind: TokenKind::Pipe,
                        span: Span::new(start_pos, start_pos + ch.len_utf8()),
                    })
                }
            }
            '^' => Ok(Token {
                kind: TokenKind::Caret,
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
            _ => Err(LexerError {
                message: format!("unexpected character '{ch}'"),
                span: Span::new(start_pos, start_pos + ch.len_utf8()),
            }),
        };

        Some(token_result)
    }
}
