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
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: Vec::new(),
            current_return_type: Type::Unit,
        }
    }

    pub fn check(mut self, program: ast::Program) -> Result<hir::Program, CompileError> {
        for function in &program.functions {
            let param_types = function
                .params
                .iter()
                .map(|param| self.type_from_type_expr(&param.ty))
                .collect::<Result<Vec<_>, _>>()?;
            let return_type = self.type_from_type_expr(&function.return_type)?;
            self.functions.insert(
                function.name.clone(),
                FunctionSignature {
                    params: param_types,
                    return_type,
                },
            );
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

    fn check_expression(&mut self, expr: ast::Expression) -> Result<hir::Expression, CompileError> {
        match expr {
            AstExpression::Literal(lit) => self.check_literal(lit),
            AstExpression::Variable(var) => self.check_variable(var),
            AstExpression::Binary(bin) => self.check_binary(bin),
            AstExpression::Unary(un) => self.check_unary(un),
            AstExpression::Call(call) => self.check_call(call),
            AstExpression::If(if_expr) => self.check_if(if_expr),
            AstExpression::Block(block) => {
                let block = self.check_block(block)?;
                Ok(hir::Expression::Block(block))
            }
        }
    }

    fn check_literal(&self, lit: ast::Literal) -> Result<hir::Expression, CompileError> {
        let (value, ty) = match lit.value {
            ast::LiteralValue::Int(value) => (LiteralValue::Int(value), Type::I32),
            ast::LiteralValue::Float(value) => (LiteralValue::Float(value), Type::F32),
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
            AstBinaryOp::Add
            | AstBinaryOp::Sub
            | AstBinaryOp::Mul
            | AstBinaryOp::Div
            | AstBinaryOp::Rem => {
                if left == right && left.is_numeric() {
                    Ok(left)
                } else {
                    Err(
                        CompileError::new("arithmetic operands must share numeric type")
                            .with_span(span),
                    )
                }
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
