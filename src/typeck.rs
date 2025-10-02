use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::ast;
use crate::ast::{
    BinaryOp as AstBinaryOp, Expression as AstExpression, Statement as AstStatement, TypeExpr,
};
use crate::error::CompileError;
use crate::hir;
use crate::hir::{BinaryOp, LiteralValue, Type, UnaryOp};
use crate::span::Span;

#[derive(Clone)]
struct FunctionSignature {
    params: Vec<Type>,
    return_type: Type,
}

#[derive(Clone)]
struct VariableInfo {
    ty: Type,
    mutable: bool,
}

#[derive(Clone, Copy)]
enum LoopKind {
    Loop,
    While,
}

struct LoopContext {
    kind: LoopKind,
    break_type: Option<Type>,
}

impl LoopContext {
    fn new(kind: LoopKind) -> Self {
        Self {
            kind,
            break_type: None,
        }
    }

    fn record_break(&mut self, ty: Type) -> Result<(), CompileError> {
        match self.kind {
            LoopKind::Loop => {
                if let Some(existing) = self.break_type {
                    if existing != ty {
                        return Err(CompileError::new(format!(
                            "break value type mismatch: expected `{}` but found `{}`",
                            type_name(existing),
                            type_name(ty)
                        )));
                    }
                } else {
                    self.break_type = Some(ty);
                }
                Ok(())
            }
            LoopKind::While => {
                if ty != Type::Unit {
                    Err(CompileError::new(
                        "`break` with a value is only allowed inside `loop` expressions",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::I32 => "i32",
        Type::I64 => "i64",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::Bool => "bool",
        Type::Unit => "()",
    }
}

struct ScopeGuard<'a> {
    checker: &'a mut TypeChecker,
}

impl<'a> ScopeGuard<'a> {
    fn new(checker: &'a mut TypeChecker) -> Self {
        checker.enter_scope();
        Self { checker }
    }
}

impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        self.checker.exit_scope();
    }
}

impl Deref for ScopeGuard<'_> {
    type Target = TypeChecker;
    fn deref(&self) -> &Self::Target {
        self.checker
    }
}

impl DerefMut for ScopeGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.checker
    }
}

pub struct TypeChecker {
    functions: HashMap<String, FunctionSignature>,
    scopes: Vec<HashMap<String, VariableInfo>>,
    current_return_type: Type,
    loop_stack: Vec<LoopContext>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        functions.insert(
            "load_u8".to_string(),
            FunctionSignature {
                params: vec![Type::I32],
                return_type: Type::I32,
            },
        );
        functions.insert(
            "store_u8".to_string(),
            FunctionSignature {
                params: vec![Type::I32, Type::I32],
                return_type: Type::Unit,
            },
        );
        functions.insert(
            "load_i32".to_string(),
            FunctionSignature {
                params: vec![Type::I32],
                return_type: Type::I32,
            },
        );
        functions.insert(
            "store_i32".to_string(),
            FunctionSignature {
                params: vec![Type::I32, Type::I32],
                return_type: Type::Unit,
            },
        );
        functions.insert(
            "load_i64".to_string(),
            FunctionSignature {
                params: vec![Type::I32],
                return_type: Type::I64,
            },
        );
        functions.insert(
            "store_i64".to_string(),
            FunctionSignature {
                params: vec![Type::I32, Type::I64],
                return_type: Type::Unit,
            },
        );
        functions.insert(
            "load_f32".to_string(),
            FunctionSignature {
                params: vec![Type::I32],
                return_type: Type::F32,
            },
        );
        functions.insert(
            "store_f32".to_string(),
            FunctionSignature {
                params: vec![Type::I32, Type::F32],
                return_type: Type::Unit,
            },
        );
        functions.insert(
            "load_f64".to_string(),
            FunctionSignature {
                params: vec![Type::I32],
                return_type: Type::F64,
            },
        );
        functions.insert(
            "store_f64".to_string(),
            FunctionSignature {
                params: vec![Type::I32, Type::F64],
                return_type: Type::Unit,
            },
        );

        Self {
            functions,
            scopes: Vec::new(),
            current_return_type: Type::Unit,
            loop_stack: Vec::new(),
        }
    }

    pub fn check(mut self, program: ast::Program) -> Result<hir::Program, CompileError> {
        let mut main_found = false;
        for function in &program.functions {
            let param_types = function
                .params
                .iter()
                .map(|param| self.type_from_type_expr(&param.ty))
                .collect::<Result<Vec<_>, _>>()?;
            let return_type = self.type_from_type_expr(&function.return_type)?;
            if function.name == "main" {
                if !param_types.is_empty() {
                    return Err(
                        CompileError::new("`main` cannot take parameters").with_span(function.span)
                    );
                }
                if return_type != Type::I32 {
                    return Err(
                        CompileError::new("`main` must return `i32`").with_span(function.span)
                    );
                }
                main_found = true;
            }
            if self.functions.contains_key(&function.name) {
                return Err(CompileError::new(format!(
                    "function `{}` already defined",
                    function.name
                ))
                .with_span(function.span));
            }
            self.functions.insert(
                function.name.clone(),
                FunctionSignature {
                    params: param_types,
                    return_type,
                },
            );
        }

        if !main_found {
            return Err(CompileError::new("program must define `fn main() -> i32`"));
        }

        let mut functions = Vec::new();
        for function in program.functions {
            functions.push(self.check_function(function)?);
        }

        Ok(hir::Program { functions })
    }

    fn check_function(&mut self, function: ast::Function) -> Result<hir::Function, CompileError> {
        let ast::Function {
            name,
            params: param_asts,
            return_type: _,
            body,
            span,
        } = function;

        let signature = self.functions.get(&name).cloned().ok_or_else(|| {
            CompileError::new(format!("unknown function `{}`", name)).with_span(span)
        })?;

        self.current_return_type = signature.return_type;
        let mut scoped = ScopeGuard::new(self);

        let mut params = Vec::new();
        for (param_ast, param_ty) in param_asts.into_iter().zip(signature.params.iter()) {
            if scoped
                .scopes
                .last_mut()
                .expect("scope exists")
                .insert(
                    param_ast.name.clone(),
                    VariableInfo {
                        ty: *param_ty,
                        mutable: false,
                    },
                )
                .is_some()
            {
                return Err(
                    CompileError::new(format!("duplicate parameter `{}`", param_ast.name))
                        .with_span(param_ast.span),
                );
            }
            params.push(hir::Param {
                name: param_ast.name,
                ty: *param_ty,
                span: param_ast.span,
            });
        }

        let body = scoped.check_block(body)?;
        let block_ty = body.ty();
        if signature.return_type != Type::Unit && block_ty != signature.return_type {
            return Err(CompileError::new(format!(
                "function `{}` must return `{}` but block evaluates to `{}`",
                name,
                scoped.format_type(signature.return_type),
                scoped.format_type(block_ty)
            ))
            .with_span(body.span));
        }

        drop(scoped);

        Ok(hir::Function {
            name,
            params,
            return_type: signature.return_type,
            body,
            span,
        })
    }

    fn check_block(&mut self, block: ast::Block) -> Result<hir::Block, CompileError> {
        let span = block.span;
        let mut scoped = ScopeGuard::new(self);
        let mut statements = Vec::new();
        for statement in block.statements {
            match statement {
                AstStatement::Let(stmt) => {
                    let typed = scoped.check_let(stmt)?;
                    statements.push(hir::Statement::Let(typed));
                }
                AstStatement::Assign(stmt) => {
                    let typed = scoped.check_assign(stmt)?;
                    statements.push(hir::Statement::Assign(typed));
                }
                AstStatement::Return(stmt) => {
                    let typed = scoped.check_return(stmt)?;
                    statements.push(hir::Statement::Return(typed));
                }
                AstStatement::Break(stmt) => {
                    let typed = scoped.check_break(stmt)?;
                    statements.push(hir::Statement::Break(typed));
                }
                AstStatement::Continue(stmt) => {
                    let typed = scoped.check_continue(stmt)?;
                    statements.push(hir::Statement::Continue(typed));
                }
                AstStatement::Expr(stmt) => {
                    let expr = scoped.check_expression(stmt.expr)?;
                    statements.push(hir::Statement::Expr(hir::ExpressionStatement {
                        expr,
                        span: stmt.span,
                    }));
                }
            }
        }

        let tail = if let Some(expr) = block.tail {
            Some(Box::new(scoped.check_expression(*expr)?))
        } else {
            None
        };

        drop(scoped);

        Ok(hir::Block {
            statements,
            tail,
            span,
        })
    }

    fn check_let(&mut self, stmt: ast::LetStatement) -> Result<hir::LetStatement, CompileError> {
        let declared_ty = self.type_from_type_expr(&stmt.ty)?;
        let value = self.check_expression(stmt.value)?;
        if value.ty() != declared_ty {
            return Err(CompileError::new(format!(
                "expected `{}` but expression has type `{}`",
                self.format_type(declared_ty),
                self.format_type(value.ty())
            ))
            .with_span(stmt.span));
        }
        if self
            .scopes
            .last_mut()
            .expect("scope exists")
            .insert(
                stmt.name.clone(),
                VariableInfo {
                    ty: declared_ty,
                    mutable: stmt.mutable,
                },
            )
            .is_some()
        {
            return Err(CompileError::new(format!(
                "variable `{}` already defined in this scope",
                stmt.name
            ))
            .with_span(stmt.span));
        }

        Ok(hir::LetStatement {
            name: stmt.name,
            mutable: stmt.mutable,
            ty: declared_ty,
            value,
            span: stmt.span,
        })
    }

    fn check_assign(
        &mut self,
        stmt: ast::AssignStatement,
    ) -> Result<hir::AssignStatement, CompileError> {
        let info = self.lookup_var(&stmt.target, stmt.span)?;
        if !info.mutable {
            return Err(CompileError::new(format!(
                "cannot assign to immutable binding `{}`",
                stmt.target
            ))
            .with_span(stmt.span));
        }
        let value = self.check_expression(stmt.value)?;
        if value.ty() != info.ty {
            return Err(CompileError::new(format!(
                "expected `{}` but expression has type `{}`",
                self.format_type(info.ty),
                self.format_type(value.ty())
            ))
            .with_span(stmt.span));
        }
        Ok(hir::AssignStatement {
            target: stmt.target,
            value,
            span: stmt.span,
        })
    }

    fn check_return(
        &mut self,
        stmt: ast::ReturnStatement,
    ) -> Result<hir::ReturnStatement, CompileError> {
        if let Some(expr) = stmt.value {
            let value = self.check_expression(expr)?;
            if value.ty() != self.current_return_type {
                return Err(CompileError::new(format!(
                    "expected return type `{}` but found `{}`",
                    self.format_type(self.current_return_type),
                    self.format_type(value.ty())
                ))
                .with_span(stmt.span));
            }
            Ok(hir::ReturnStatement {
                value: Some(value),
                span: stmt.span,
            })
        } else {
            if self.current_return_type != Type::Unit {
                return Err(CompileError::new(format!(
                    "expected value of type `{}`",
                    self.format_type(self.current_return_type)
                ))
                .with_span(stmt.span));
            }
            Ok(hir::ReturnStatement {
                value: None,
                span: stmt.span,
            })
        }
    }

    fn check_break(
        &mut self,
        stmt: ast::BreakStatement,
    ) -> Result<hir::BreakStatement, CompileError> {
        if self.loop_stack.is_empty() {
            return Err(CompileError::new("`break` outside of loop").with_span(stmt.span));
        }
        if let Some(expr) = stmt.value {
            let value = self.check_expression(expr)?;
            let context = self
                .loop_stack
                .last_mut()
                .expect("loop context should exist");
            context
                .record_break(value.ty())
                .map_err(|err| err.with_span(stmt.span))?;
            Ok(hir::BreakStatement {
                value: Some(value),
                span: stmt.span,
            })
        } else {
            let context = self
                .loop_stack
                .last_mut()
                .expect("loop context should exist");
            context
                .record_break(Type::Unit)
                .map_err(|err| err.with_span(stmt.span))?;
            Ok(hir::BreakStatement {
                value: None,
                span: stmt.span,
            })
        }
    }

    fn check_continue(
        &mut self,
        stmt: ast::ContinueStatement,
    ) -> Result<hir::ContinueStatement, CompileError> {
        if self.loop_stack.is_empty() {
            Err(CompileError::new("`continue` outside of loop").with_span(stmt.span))
        } else {
            Ok(hir::ContinueStatement { span: stmt.span })
        }
    }

    fn check_expression(&mut self, expr: ast::Expression) -> Result<hir::Expression, CompileError> {
        match expr {
            AstExpression::Literal(lit) => self.check_literal(lit),
            AstExpression::Variable(var) => self.check_variable(var),
            AstExpression::Binary(bin) => self.check_binary(bin),
            AstExpression::Unary(un) => self.check_unary(un),
            AstExpression::Call(call) => self.check_call(call),
            AstExpression::Group(group) => self.check_expression(*group.expr),
            AstExpression::If(if_expr) => self.check_if(if_expr),
            AstExpression::Block(block) => {
                let block = self.check_block(block)?;
                Ok(hir::Expression::Block(block))
            }
            AstExpression::Loop(loop_expr) => self.check_loop(loop_expr),
            AstExpression::While(while_expr) => self.check_while(while_expr),
        }
    }

    fn check_loop(&mut self, loop_expr: ast::LoopExpr) -> Result<hir::Expression, CompileError> {
        let ast::LoopExpr { body, span } = loop_expr;
        self.loop_stack.push(LoopContext::new(LoopKind::Loop));
        let (body, loop_type) = match self.check_block(body) {
            Ok(body) => {
                let context = self.loop_stack.pop().expect("loop context should exist");
                let ty = context.break_type.unwrap_or(Type::Unit);
                (body, ty)
            }
            Err(err) => {
                self.loop_stack.pop();
                return Err(err);
            }
        };
        Ok(hir::Expression::Loop(hir::LoopExpr {
            body: Box::new(body),
            ty: loop_type,
            span,
        }))
    }

    fn check_while(&mut self, while_expr: ast::WhileExpr) -> Result<hir::Expression, CompileError> {
        let ast::WhileExpr {
            condition,
            body,
            span,
        } = while_expr;
        let condition = self.check_expression(*condition)?;
        if condition.ty() != Type::Bool {
            return Err(CompileError::new("while condition must be boolean").with_span(span));
        }
        self.loop_stack.push(LoopContext::new(LoopKind::While));
        let body = match self.check_block(body) {
            Ok(body) => {
                self.loop_stack.pop();
                body
            }
            Err(err) => {
                self.loop_stack.pop();
                return Err(err);
            }
        };
        Ok(hir::Expression::While(hir::WhileExpr {
            condition: Box::new(condition),
            body: Box::new(body),
            ty: Type::Unit,
            span,
        }))
    }

    fn check_literal(&self, lit: ast::Literal) -> Result<hir::Expression, CompileError> {
        let (value, ty) = match lit.value {
            ast::LiteralValue::Int(int_lit) => {
                let ty = match int_lit.suffix.unwrap_or(ast::IntSuffix::I32) {
                    ast::IntSuffix::I32 => Type::I32,
                    ast::IntSuffix::I64 => Type::I64,
                };
                if ty == Type::I32
                    && (int_lit.value < i32::MIN as i64 || int_lit.value > i32::MAX as i64)
                {
                    return Err(CompileError::new("integer literal out of range for i32")
                        .with_span(lit.span));
                }
                (LiteralValue::Int(int_lit.value), ty)
            }
            ast::LiteralValue::Float(float_lit) => {
                let ty = match float_lit.suffix.unwrap_or(ast::FloatSuffix::F32) {
                    ast::FloatSuffix::F32 => Type::F32,
                    ast::FloatSuffix::F64 => Type::F64,
                };
                (LiteralValue::Float(float_lit.value), ty)
            }
            ast::LiteralValue::Bool(value) => (LiteralValue::Bool(value), Type::Bool),
        };
        Ok(hir::Expression::Literal(hir::Literal {
            value,
            ty,
            span: lit.span,
        }))
    }

    fn check_variable(&self, var: ast::Variable) -> Result<hir::Expression, CompileError> {
        let info = self.lookup_var(&var.name, var.span)?;
        Ok(hir::Expression::Variable(hir::Variable {
            name: var.name,
            ty: info.ty,
            span: var.span,
        }))
    }

    fn check_binary(&mut self, expr: ast::BinaryExpr) -> Result<hir::Expression, CompileError> {
        let op = expr.op.clone();
        let left = self.check_expression(*expr.left)?;
        let right = self.check_expression(*expr.right)?;
        let ty = self.binary_result_type(op.clone(), left.ty(), right.ty(), expr.span)?;
        Ok(hir::Expression::Binary(hir::BinaryExpr {
            op: self.map_binary_op(op),
            left: Box::new(left),
            right: Box::new(right),
            ty,
            span: expr.span,
        }))
    }

    fn check_unary(&mut self, expr: ast::UnaryExpr) -> Result<hir::Expression, CompileError> {
        let operand = self.check_expression(*expr.expr)?;
        let ty = match expr.op {
            ast::UnaryOp::Neg => {
                if operand.ty().is_numeric() {
                    operand.ty()
                } else {
                    return Err(
                        CompileError::new("`-` requires numeric operand").with_span(expr.span)
                    );
                }
            }
            ast::UnaryOp::Not => {
                if operand.ty() == Type::Bool {
                    Type::Bool
                } else {
                    return Err(
                        CompileError::new("`!` requires boolean operand").with_span(expr.span)
                    );
                }
            }
        };
        Ok(hir::Expression::Unary(hir::UnaryExpr {
            op: match expr.op {
                ast::UnaryOp::Neg => UnaryOp::Neg,
                ast::UnaryOp::Not => UnaryOp::Not,
            },
            expr: Box::new(operand),
            ty,
            span: expr.span,
        }))
    }

    fn check_call(&mut self, call: ast::CallExpr) -> Result<hir::Expression, CompileError> {
        let signature = self.functions.get(&call.callee).cloned().ok_or_else(|| {
            CompileError::new(format!("unknown function `{}`", call.callee)).with_span(call.span)
        })?;
        if signature.params.len() != call.args.len() {
            return Err(CompileError::new(format!(
                "function `{}` expects {} arguments but {} given",
                call.callee,
                signature.params.len(),
                call.args.len()
            ))
            .with_span(call.span));
        }
        let mut args = Vec::new();
        for (arg_expr, expected_ty) in call.args.into_iter().zip(signature.params.iter()) {
            let arg = self.check_expression(arg_expr)?;
            if arg.ty() != *expected_ty {
                return Err(CompileError::new(format!(
                    "expected argument of type `{}` but found `{}`",
                    self.format_type(*expected_ty),
                    self.format_type(arg.ty())
                ))
                .with_span(call.span));
            }
            args.push(arg);
        }
        Ok(hir::Expression::Call(hir::CallExpr {
            callee: call.callee,
            args,
            ty: signature.return_type,
            span: call.span,
        }))
    }

    fn check_if(&mut self, if_expr: ast::IfExpr) -> Result<hir::Expression, CompileError> {
        let condition = self.check_expression(*if_expr.condition)?;
        if condition.ty() != Type::Bool {
            return Err(CompileError::new("if condition must be boolean").with_span(if_expr.span));
        }
        let then_block = self.check_block(if_expr.then_branch)?;
        let then_ty = then_block.ty();
        let else_branch = if let Some(else_expr) = if_expr.else_branch {
            Some(Box::new(self.check_expression(*else_expr)?))
        } else {
            None
        };

        let ty = if let Some(ref else_expr) = else_branch {
            let else_ty = else_expr.ty();
            if then_ty != else_ty {
                return Err(CompileError::new(format!(
                    "mismatched branch types `{}` and `{}`",
                    self.format_type(then_ty),
                    self.format_type(else_ty)
                ))
                .with_span(if_expr.span));
            }
            then_ty
        } else {
            Type::Unit
        };

        Ok(hir::Expression::If(hir::IfExpr {
            condition: Box::new(condition),
            then_branch: Box::new(then_block),
            else_branch,
            ty,
            span: if_expr.span,
        }))
    }

    fn binary_result_type(
        &self,
        op: AstBinaryOp,
        left: Type,
        right: Type,
        span: Span,
    ) -> Result<Type, CompileError> {
        match op {
            AstBinaryOp::Add | AstBinaryOp::Sub | AstBinaryOp::Mul | AstBinaryOp::Div => {
                if left == right && left.is_numeric() {
                    Ok(left)
                } else {
                    Err(
                        CompileError::new("arithmetic operands must share numeric type")
                            .with_span(span),
                    )
                }
            }
            AstBinaryOp::Rem => {
                if left != right {
                    return Err(CompileError::new(format!(
                        "remainder operands must share type, found `{}` and `{}`",
                        self.format_type(left),
                        self.format_type(right)
                    ))
                    .with_span(span));
                }
                if !left.is_integer() {
                    return Err(CompileError::new(format!(
                        "remainder is only supported for integer types, found `{}`",
                        self.format_type(left)
                    ))
                    .with_span(span));
                }
                Ok(left)
            }
            AstBinaryOp::BitAnd | AstBinaryOp::BitOr | AstBinaryOp::BitXor => {
                if left != right {
                    return Err(CompileError::new(format!(
                        "bitwise operands must share type, found `{}` and `{}`",
                        self.format_type(left),
                        self.format_type(right)
                    ))
                    .with_span(span));
                }
                if !left.is_integer() {
                    return Err(CompileError::new(format!(
                        "bitwise operations require integer types, found `{}`",
                        self.format_type(left)
                    ))
                    .with_span(span));
                }
                Ok(left)
            }
            AstBinaryOp::Shl | AstBinaryOp::Shr => {
                if left != right {
                    return Err(CompileError::new(format!(
                        "shift operands must share type, found `{}` and `{}`",
                        self.format_type(left),
                        self.format_type(right)
                    ))
                    .with_span(span));
                }
                if !left.is_integer() {
                    return Err(CompileError::new(format!(
                        "shifts are only supported for integer types, found `{}`",
                        self.format_type(left)
                    ))
                    .with_span(span));
                }
                Ok(left)
            }
            AstBinaryOp::Eq | AstBinaryOp::Ne => {
                if left == right {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new("equality operands must share type").with_span(span))
                }
            }
            AstBinaryOp::Lt | AstBinaryOp::Le | AstBinaryOp::Gt | AstBinaryOp::Ge => {
                if left == right && left.is_numeric() {
                    Ok(Type::Bool)
                } else {
                    Err(
                        CompileError::new("comparison operands must share numeric type")
                            .with_span(span),
                    )
                }
            }
            AstBinaryOp::And | AstBinaryOp::Or => {
                if left == Type::Bool && right == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new("logical operands must be boolean").with_span(span))
                }
            }
        }
    }

    fn map_binary_op(&self, op: AstBinaryOp) -> BinaryOp {
        match op {
            AstBinaryOp::Add => BinaryOp::Add,
            AstBinaryOp::Sub => BinaryOp::Sub,
            AstBinaryOp::Mul => BinaryOp::Mul,
            AstBinaryOp::Div => BinaryOp::Div,
            AstBinaryOp::Rem => BinaryOp::Rem,
            AstBinaryOp::Eq => BinaryOp::Eq,
            AstBinaryOp::Ne => BinaryOp::Ne,
            AstBinaryOp::Lt => BinaryOp::Lt,
            AstBinaryOp::Le => BinaryOp::Le,
            AstBinaryOp::Gt => BinaryOp::Gt,
            AstBinaryOp::Ge => BinaryOp::Ge,
            AstBinaryOp::And => BinaryOp::And,
            AstBinaryOp::Or => BinaryOp::Or,
            AstBinaryOp::BitAnd => BinaryOp::BitAnd,
            AstBinaryOp::BitOr => BinaryOp::BitOr,
            AstBinaryOp::BitXor => BinaryOp::BitXor,
            AstBinaryOp::Shl => BinaryOp::Shl,
            AstBinaryOp::Shr => BinaryOp::Shr,
        }
    }

    fn type_from_type_expr(&self, ty: &TypeExpr) -> Result<Type, CompileError> {
        match ty {
            TypeExpr::Named { name, span } => match name.as_str() {
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                "bool" => Ok(Type::Bool),
                _ => Err(CompileError::new(format!("unknown type `{}`", name)).with_span(*span)),
            },
            TypeExpr::Unit { .. } => Ok(Type::Unit),
        }
    }

    fn lookup_var(&self, name: &str, span: Span) -> Result<VariableInfo, CompileError> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Ok(info.clone());
            }
        }
        Err(CompileError::new(format!("unknown identifier `{}`", name)).with_span(span))
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn format_type(&self, ty: Type) -> &'static str {
        match ty {
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::F32 => "f32",
            Type::F64 => "f64",
            Type::Bool => "bool",
            Type::Unit => "()",
        }
    }
}
