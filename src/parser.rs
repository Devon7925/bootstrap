use crate::ast::*;
use crate::error::CompileError;
use crate::lexer::{
    FloatLiteralSuffix as LexerFloatSuffix, IntLiteralSuffix as LexerIntSuffix, Token, TokenKind,
};
use crate::span::Span;

pub struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    #[allow(dead_code)]
    source: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            tokens,
            index: 0,
            source,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, CompileError> {
        let mut functions = Vec::new();
        while !self.is_eof() {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, CompileError> {
        let fn_token = self.expect(|k| matches!(k, TokenKind::Fn), "expected `fn`")?;
        let name_token = self.expect_identifier("expected function name")?;
        let name = if let TokenKind::Identifier(value) = name_token.kind.clone() {
            value
        } else {
            unreachable!()
        };

        self.expect(
            |k| matches!(k, TokenKind::LParen),
            "expected '(' after function name",
        )?;
        let params = self.parse_params()?;
        self.expect(
            |k| matches!(k, TokenKind::RParen),
            "expected ')' after parameters",
        )?;

        let return_type = if self.current_is(|k| matches!(k, TokenKind::Arrow)) {
            self.advance();
            self.parse_type_expr()?
        } else {
            let span = Span::new(name_token.span.end, name_token.span.end);
            TypeExpr::Unit { span }
        };

        let body = self.parse_block()?;
        let span = Span::new(fn_token.span.start, body.span.end);

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, CompileError> {
        let mut params = Vec::new();
        while !self.current_is(|k| matches!(k, TokenKind::RParen)) {
            let name_token = self.expect_identifier("expected parameter name")?;
            let name = if let TokenKind::Identifier(value) = name_token.kind.clone() {
                value
            } else {
                unreachable!()
            };
            self.expect(
                |k| matches!(k, TokenKind::Colon),
                "expected ':' after parameter name",
            )?;
            let ty = self.parse_type_expr()?;
            let span = Span::new(name_token.span.start, ty.span().end);
            params.push(Param { name, ty, span });

            if self.current_is(|k| matches!(k, TokenKind::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, CompileError> {
        if self.current_is(|k| matches!(k, TokenKind::Identifier(_))) {
            let token = self.expect_identifier("expected type name")?;
            if let TokenKind::Identifier(name) = token.kind {
                Ok(TypeExpr::Named {
                    name,
                    span: token.span,
                })
            } else {
                unreachable!()
            }
        } else if self.current_is(|k| matches!(k, TokenKind::LParen))
            && self.lookahead_is(|k| matches!(k, TokenKind::RParen))
        {
            let start = self.advance().span.start;
            let end = self.advance().span.end;
            Ok(TypeExpr::Unit {
                span: Span::new(start, end),
            })
        } else {
            Err(self.error_here("expected type"))
        }
    }

    fn parse_block(&mut self) -> Result<Block, CompileError> {
        let lbrace = self.expect(|k| matches!(k, TokenKind::LBrace), "expected '{'")?;
        let mut statements = Vec::new();
        let mut tail = None;

        while !self.current_is(|k| matches!(k, TokenKind::RBrace)) && !self.is_eof() {
            if self.current_is(|k| matches!(k, TokenKind::Let)) {
                let stmt = self.parse_let_statement()?;
                statements.push(Statement::Let(stmt));
                self.expect(
                    |k| matches!(k, TokenKind::Semicolon),
                    "expected ';' after let statement",
                )?;
            } else if self.current_is(|k| matches!(k, TokenKind::Return)) {
                let stmt = self.parse_return_statement()?;
                statements.push(Statement::Return(stmt));
                self.expect(
                    |k| matches!(k, TokenKind::Semicolon),
                    "expected ';' after return",
                )?;
            } else if self.is_assignment_start() {
                let stmt = self.parse_assignment()?;
                statements.push(Statement::Assign(stmt));
                self.expect(
                    |k| matches!(k, TokenKind::Semicolon),
                    "expected ';' after assignment",
                )?;
            } else if self.current_is(|k| matches!(k, TokenKind::Break)) {
                let stmt = self.parse_break_statement()?;
                statements.push(Statement::Break(stmt));
                self.expect(
                    |k| matches!(k, TokenKind::Semicolon),
                    "expected ';' after break",
                )?;
            } else if self.current_is(|k| matches!(k, TokenKind::Continue)) {
                let stmt = self.parse_continue_statement()?;
                statements.push(Statement::Continue(stmt));
                self.expect(
                    |k| matches!(k, TokenKind::Semicolon),
                    "expected ';' after continue",
                )?;
            } else {
                let expr = self.parse_expression()?;
                if self.current_is(|k| matches!(k, TokenKind::Semicolon)) {
                    let semi = self.advance();
                    let span = Span::new(expr.span().start, semi.span.end);
                    statements.push(Statement::Expr(ExpressionStatement { expr, span }));
                } else if matches!(expr, Expression::Loop(_) | Expression::While(_)) {
                    let span = expr.span();
                    statements.push(Statement::Expr(ExpressionStatement { expr, span }));
                } else {
                    tail = Some(Box::new(expr));
                    break;
                }
            }
        }

        let rbrace = self.expect(
            |k| matches!(k, TokenKind::RBrace),
            "expected '}' to close block",
        )?;
        let span = Span::new(lbrace.span.start, rbrace.span.end);
        Ok(Block {
            statements,
            tail,
            span,
        })
    }

    fn parse_let_statement(&mut self) -> Result<LetStatement, CompileError> {
        let let_token = self.expect(|k| matches!(k, TokenKind::Let), "expected 'let'")?;
        let mutable = if self.current_is(|k| matches!(k, TokenKind::Mut)) {
            self.advance();
            true
        } else {
            false
        };
        let name_token = self.expect_identifier("expected binding name")?;
        let name = if let TokenKind::Identifier(value) = name_token.kind.clone() {
            value
        } else {
            unreachable!()
        };
        self.expect(
            |k| matches!(k, TokenKind::Colon),
            "expected ':' after binding name",
        )?;
        let ty = self.parse_type_expr()?;
        self.expect(
            |k| matches!(k, TokenKind::Assign),
            "expected '=' in let binding",
        )?;
        let value = self.parse_expression()?;
        let span = Span::new(let_token.span.start, value.span().end);
        Ok(LetStatement {
            name,
            mutable,
            ty,
            value,
            span,
        })
    }

    fn parse_assignment(&mut self) -> Result<AssignStatement, CompileError> {
        let ident_token = self.expect_identifier("expected identifier for assignment")?;
        let name = if let TokenKind::Identifier(value) = ident_token.kind.clone() {
            value
        } else {
            unreachable!()
        };
        self.expect(
            |k| matches!(k, TokenKind::Assign),
            "expected '=' in assignment",
        )?;
        let value = self.parse_expression()?;
        let span = Span::new(ident_token.span.start, value.span().end);
        Ok(AssignStatement {
            target: name,
            value,
            span,
        })
    }

    fn parse_return_statement(&mut self) -> Result<ReturnStatement, CompileError> {
        let return_token = self.expect(|k| matches!(k, TokenKind::Return), "expected 'return'")?;
        if self.current_is(|k| matches!(k, TokenKind::Semicolon)) {
            Ok(ReturnStatement {
                value: None,
                span: return_token.span,
            })
        } else {
            let expr = self.parse_expression()?;
            let span = Span::new(return_token.span.start, expr.span().end);
            Ok(ReturnStatement {
                value: Some(expr),
                span,
            })
        }
    }

    fn parse_break_statement(&mut self) -> Result<BreakStatement, CompileError> {
        let token = self.expect(|k| matches!(k, TokenKind::Break), "expected 'break'")?;
        Ok(BreakStatement { span: token.span })
    }

    fn parse_continue_statement(&mut self) -> Result<ContinueStatement, CompileError> {
        let token = self.expect(|k| matches!(k, TokenKind::Continue), "expected 'continue'")?;
        Ok(ContinueStatement { span: token.span })
    }

    fn parse_expression(&mut self) -> Result<Expression, CompileError> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, min_prec: u8) -> Result<Expression, CompileError> {
        let mut left = self.parse_unary()?;
        loop {
            let op_info = match self.current_binary_op() {
                Some(info) => info,
                None => break,
            };
            let (op, precedence, right_assoc) = op_info;
            if precedence < min_prec {
                break;
            }
            self.advance();
            let next_min_prec = if right_assoc {
                precedence
            } else {
                precedence + 1
            };
            let right = self.parse_binary_expression(next_min_prec)?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expression::Binary(BinaryExpr {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, CompileError> {
        if self.current_is(|k| matches!(k, TokenKind::Minus)) {
            let token = self.advance();
            let expr = self.parse_unary()?;
            let span = Span::new(token.span.start, expr.span().end);
            Ok(Expression::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            }))
        } else if self.current_is(|k| matches!(k, TokenKind::Bang)) {
            let token = self.advance();
            let expr = self.parse_unary()?;
            let span = Span::new(token.span.start, expr.span().end);
            Ok(Expression::Unary(UnaryExpr {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            }))
        } else {
            self.parse_call()
        }
    }

    fn parse_call(&mut self) -> Result<Expression, CompileError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.current_is(|k| matches!(k, TokenKind::LParen)) {
                let start = expr.span().start;
                self.advance();
                let mut args = Vec::new();
                if !self.current_is(|k| matches!(k, TokenKind::RParen)) {
                    loop {
                        let arg = self.parse_expression()?;
                        args.push(arg);
                        if self.current_is(|k| matches!(k, TokenKind::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                let rparen = self.expect(
                    |k| matches!(k, TokenKind::RParen),
                    "expected ')' after arguments",
                )?;
                let span = Span::new(start, rparen.span.end);
                let callee_name = match expr {
                    Expression::Variable(Variable { name, .. }) => name,
                    _ => return Err(self.error_at(span, "only identifier calls supported in MVP")),
                };
                expr = Expression::Call(CallExpr {
                    callee: callee_name,
                    args,
                    span,
                });
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, CompileError> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::IntLiteral { value, suffix } => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    value: LiteralValue::Int(IntLiteral {
                        value: *value,
                        suffix: suffix.map(|s| match s {
                            LexerIntSuffix::I32 => IntSuffix::I32,
                            LexerIntSuffix::I64 => IntSuffix::I64,
                        }),
                    }),
                    span: token.span,
                }))
            }
            TokenKind::FloatLiteral { value, suffix } => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    value: LiteralValue::Float(FloatLiteral {
                        value: *value,
                        suffix: suffix.map(|s| match s {
                            LexerFloatSuffix::F32 => FloatSuffix::F32,
                            LexerFloatSuffix::F64 => FloatSuffix::F64,
                        }),
                    }),
                    span: token.span,
                }))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    value: LiteralValue::Bool(true),
                    span: token.span,
                }))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Literal {
                    value: LiteralValue::Bool(false),
                    span: token.span,
                }))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Expression::Variable(Variable {
                    name: name.clone(),
                    span: token.span,
                }))
            }
            TokenKind::If => self.parse_if_expression(),
            TokenKind::Loop => self.parse_loop_expression(),
            TokenKind::While => self.parse_while_expression(),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Expression::Block(block))
            }
            _ => Err(self.error_at(token.span, "unexpected token in expression")),
        }
    }

    fn parse_if_expression(&mut self) -> Result<Expression, CompileError> {
        let if_token = self.expect(|k| matches!(k, TokenKind::If), "expected 'if'")?;
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.current_is(|k| matches!(k, TokenKind::Else)) {
            self.advance();
            if self.current_is(|k| matches!(k, TokenKind::If)) {
                Some(Box::new(self.parse_if_expression()?))
            } else {
                let block_expr = self.parse_block()?;
                Some(Box::new(Expression::Block(block_expr)))
            }
        } else {
            None
        };
        let end_span = else_branch
            .as_ref()
            .map(|expr| expr.span().end)
            .unwrap_or_else(|| then_branch.span.end);
        let span = Span::new(if_token.span.start, end_span);
        Ok(Expression::If(IfExpr {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span,
        }))
    }

    fn parse_loop_expression(&mut self) -> Result<Expression, CompileError> {
        let loop_token = self.expect(|k| matches!(k, TokenKind::Loop), "expected 'loop'")?;
        let body = self.parse_block()?;
        let span = Span::new(loop_token.span.start, body.span.end);
        Ok(Expression::Loop(LoopExpr { body, span }))
    }

    fn parse_while_expression(&mut self) -> Result<Expression, CompileError> {
        let while_token = self.expect(|k| matches!(k, TokenKind::While), "expected 'while'")?;
        self.expect(
            |k| matches!(k, TokenKind::LParen),
            "expected '(' after while",
        )?;
        let condition = self.parse_expression()?;
        self.expect(
            |k| matches!(k, TokenKind::RParen),
            "expected ')' after while condition",
        )?;
        let body = self.parse_block()?;
        let span = Span::new(while_token.span.start, body.span.end);
        Ok(Expression::While(WhileExpr {
            condition: Box::new(condition),
            body,
            span,
        }))
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, bool)> {
        match self.current().kind {
            TokenKind::Plus => Some((BinaryOp::Add, 10, false)),
            TokenKind::Minus => Some((BinaryOp::Sub, 10, false)),
            TokenKind::Star => Some((BinaryOp::Mul, 20, false)),
            TokenKind::Slash => Some((BinaryOp::Div, 20, false)),
            TokenKind::Percent => Some((BinaryOp::Rem, 20, false)),
            TokenKind::DoubleEquals => Some((BinaryOp::Eq, 5, false)),
            TokenKind::NotEquals => Some((BinaryOp::Ne, 5, false)),
            TokenKind::LessThan => Some((BinaryOp::Lt, 5, false)),
            TokenKind::LessThanEqual => Some((BinaryOp::Le, 5, false)),
            TokenKind::GreaterThan => Some((BinaryOp::Gt, 5, false)),
            TokenKind::GreaterThanEqual => Some((BinaryOp::Ge, 5, false)),
            TokenKind::AndAnd => Some((BinaryOp::And, 3, false)),
            TokenKind::OrOr => Some((BinaryOp::Or, 2, false)),
            _ => None,
        }
    }

    fn is_assignment_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
            && self.lookahead_is(|k| matches!(k, TokenKind::Assign))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn current_is<F>(&self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        predicate(&self.current().kind)
    }

    fn lookahead_is<F>(&self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        self.tokens
            .get(self.index + 1)
            .map(|token| predicate(&token.kind))
            .unwrap_or(false)
    }

    fn expect<F>(&mut self, predicate: F, message: &str) -> Result<Token, CompileError>
    where
        F: Fn(&TokenKind) -> bool,
    {
        if predicate(&self.current().kind) {
            let token = self.current().clone();
            self.index += 1;
            Ok(token)
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<Token, CompileError> {
        self.expect(|k| matches!(k, TokenKind::Identifier(_)), message)
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        self.index += 1;
        token
    }

    fn is_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn error_here(&self, message: &str) -> CompileError {
        self.error_at(self.current().span, message)
    }

    fn error_at(&self, span: Span, message: &str) -> CompileError {
        CompileError {
            message: message.into(),
            span: Some(span),
        }
    }
}
