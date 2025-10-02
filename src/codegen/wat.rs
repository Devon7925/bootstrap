use std::collections::HashMap;

use crate::error::CompileError;
use crate::hir;
use crate::hir::{BinaryOp, Expression, LiteralValue, Statement, Type, UnaryOp};

pub struct WatGenerator;

impl Default for WatGenerator {
    fn default() -> Self {
        Self
    }
}

impl WatGenerator {
    pub fn emit_program(&self, program: &hir::Program) -> Result<String, CompileError> {
        let mut module = String::new();
        module.push_str("(module\n");

        let mut function_names = Vec::new();
        for function in &program.functions {
            let mut emitter = FunctionEmitter::new(function);
            let func_text = emitter.emit_function()?;
            module.push_str(&func_text);
            function_names.push(function.name.clone());
        }

        for name in function_names {
            module.push_str(&format!("  (export \"{}\" (func ${}))\n", name, name));
        }

        module.push_str(")\n");
        Ok(module)
    }
}

struct FunctionEmitter<'a> {
    func: &'a hir::Function,
    params: Vec<Parameter>,
    locals: Vec<Local>,
    scopes: Vec<HashMap<String, String>>,
    instructions: Vec<String>,
    indent: usize,
    temp_counter: usize,
    loop_stack: Vec<LoopLabels>,
}

struct Parameter {
    wasm_name: String,
    wasm_type: &'static str,
}

struct Local {
    wasm_name: String,
    wasm_type: &'static str,
}

struct LoopLabels {
    break_label: String,
    continue_label: String,
}

enum TailMode {
    Preserve,
    Drop,
}

impl<'a> FunctionEmitter<'a> {
    fn new(func: &'a hir::Function) -> Self {
        Self {
            func,
            params: Vec::new(),
            locals: Vec::new(),
            scopes: Vec::new(),
            instructions: Vec::new(),
            indent: 2,
            temp_counter: 0,
            loop_stack: Vec::new(),
        }
    }

    fn emit_function(&mut self) -> Result<String, CompileError> {
        self.push_scope();
        for param in &self.func.params {
            let wasm_type = wasm_value_type(param.ty)?;
            let wasm_name = self.make_symbol(&format!("p_{}", param.name));
            self.params.push(Parameter {
                wasm_name: wasm_name.clone(),
                wasm_type,
            });
            self.insert_binding(&param.name, wasm_name);
        }

        let tail_mode = if self.func.return_type == Type::Unit {
            TailMode::Drop
        } else {
            TailMode::Preserve
        };
        self.emit_block(&self.func.body, tail_mode)?;
        self.pop_scope();

        let mut text = String::new();
        text.push_str(&format!("  (func ${}", self.func.name));
        for param in &self.params {
            text.push_str(&format!(" (param {} {})", param.wasm_name, param.wasm_type));
        }
        if self.func.return_type != Type::Unit {
            text.push_str(&format!(
                " (result {})",
                wasm_value_type(self.func.return_type)?
            ));
        }
        text.push('\n');

        for local in &self.locals {
            text.push_str(&format!(
                "    (local {} {})\n",
                local.wasm_name, local.wasm_type
            ));
        }

        for line in &self.instructions {
            text.push_str(line);
            text.push('\n');
        }

        text.push_str("  )\n");
        Ok(text)
    }

    fn emit_block(&mut self, block: &hir::Block, tail_mode: TailMode) -> Result<(), CompileError> {
        self.push_scope();
        let result = (|| -> Result<(), CompileError> {
            for statement in &block.statements {
                self.emit_statement(statement)?;
            }
            match (block.tail.as_deref(), &tail_mode) {
                (Some(expr), TailMode::Preserve) => {
                    self.emit_expression(expr)?;
                }
                (Some(expr), TailMode::Drop) => {
                    self.emit_expression(expr)?;
                    if expr.ty() != Type::Unit {
                        self.push_line("drop");
                    }
                }
                (None, TailMode::Preserve) => {
                    return Err(CompileError::new("block must evaluate to a value"));
                }
                (None, TailMode::Drop) => {}
            }
            Ok(())
        })();
        self.pop_scope();
        result
    }

    fn emit_statement(&mut self, statement: &Statement) -> Result<(), CompileError> {
        match statement {
            Statement::Let(stmt) => {
                let wasm_type = wasm_value_type(stmt.ty)?;
                let wasm_name = self.make_symbol(&format!("l_{}", stmt.name));
                self.locals.push(Local {
                    wasm_name: wasm_name.clone(),
                    wasm_type,
                });
                self.insert_binding(&stmt.name, wasm_name.clone());
                self.emit_expression(&stmt.value)?;
                if stmt.value.ty() == Type::Unit {
                    return Err(CompileError::new("let initializer cannot be unit"));
                }
                self.push_line(&format!("local.set {}", wasm_name));
                Ok(())
            }
            Statement::Assign(stmt) => {
                let wasm_name = self.lookup_binding(&stmt.target)?;
                self.emit_expression(&stmt.value)?;
                if stmt.value.ty() == Type::Unit {
                    return Err(CompileError::new("assignment value cannot be unit"));
                }
                self.push_line(&format!("local.set {}", wasm_name));
                Ok(())
            }
            Statement::Return(stmt) => {
                if let Some(expr) = &stmt.value {
                    self.emit_expression(expr)?;
                    self.push_line("return");
                } else {
                    self.push_line("return");
                }
                Ok(())
            }
            Statement::Expr(stmt) => {
                self.emit_expression(&stmt.expr)?;
                if stmt.expr.ty() != Type::Unit {
                    self.push_line("drop");
                }
                Ok(())
            }
            Statement::Break(_) => {
                let labels = self
                    .current_loop_labels()
                    .ok_or_else(|| CompileError::new("`break` outside of loop"))?;
                self.push_line(&format!("br {}", labels.break_label));
                Ok(())
            }
            Statement::Continue(_) => {
                let labels = self
                    .current_loop_labels()
                    .ok_or_else(|| CompileError::new("`continue` outside of loop"))?;
                self.push_line(&format!("br {}", labels.continue_label));
                Ok(())
            }
        }
    }

    fn emit_expression(&mut self, expr: &Expression) -> Result<(), CompileError> {
        match expr {
            Expression::Literal(lit) => {
                match &lit.value {
                    LiteralValue::Int(value) => {
                        self.push_line(&format!("i32.const {}", value));
                    }
                    LiteralValue::Float(value) => {
                        self.push_line(&format!("f32.const {}", *value as f32));
                    }
                    LiteralValue::Bool(value) => {
                        self.push_line(&format!("i32.const {}", if *value { 1 } else { 0 }));
                    }
                }
                Ok(())
            }
            Expression::Variable(var) => {
                let wasm_name = self.lookup_binding(&var.name)?;
                self.push_line(&format!("local.get {}", wasm_name));
                Ok(())
            }
            Expression::Binary(expr) => self.emit_binary(expr),
            Expression::Unary(expr) => self.emit_unary(expr),
            Expression::Call(call) => {
                for arg in &call.args {
                    self.emit_expression(arg)?;
                }
                self.push_line(&format!("call ${}", call.callee));
                Ok(())
            }
            Expression::If(if_expr) => self.emit_if(if_expr),
            Expression::Block(block) => {
                self.emit_block(block, TailMode::Preserve)?;
                Ok(())
            }
            Expression::Loop(loop_expr) => self.emit_loop(loop_expr),
            Expression::While(while_expr) => self.emit_while(while_expr),
        }
    }

    fn emit_binary(&mut self, expr: &hir::BinaryExpr) -> Result<(), CompileError> {
        use BinaryOp::*;
        if matches!(expr.op, And | Or) {
            return self.emit_logical(expr);
        }
        let operand_ty = expr.left.ty();
        self.emit_expression(&expr.left)?;
        self.emit_expression(&expr.right)?;
        let op = match expr.op {
            Add => match operand_ty {
                Type::I32 => "i32.add",
                Type::I64 => "i64.add",
                Type::F32 => "f32.add",
                Type::F64 => "f64.add",
                _ => return Err(CompileError::new("unsupported add operand type")),
            },
            Sub => match operand_ty {
                Type::I32 => "i32.sub",
                Type::I64 => "i64.sub",
                Type::F32 => "f32.sub",
                Type::F64 => "f64.sub",
                _ => return Err(CompileError::new("unsupported sub operand type")),
            },
            Mul => match operand_ty {
                Type::I32 => "i32.mul",
                Type::I64 => "i64.mul",
                Type::F32 => "f32.mul",
                Type::F64 => "f64.mul",
                _ => return Err(CompileError::new("unsupported mul operand type")),
            },
            Div => match operand_ty {
                Type::I32 => "i32.div_s",
                Type::I64 => "i64.div_s",
                Type::F32 => "f32.div",
                Type::F64 => "f64.div",
                _ => return Err(CompileError::new("unsupported div operand type")),
            },
            Rem => match operand_ty {
                Type::I32 => "i32.rem_s",
                Type::I64 => "i64.rem_s",
                _ => return Err(CompileError::new("remainder not supported for this type")),
            },
            Eq => match operand_ty {
                Type::I32 | Type::Bool => "i32.eq",
                Type::I64 => "i64.eq",
                Type::F32 => "f32.eq",
                Type::F64 => "f64.eq",
                Type::Unit => return Err(CompileError::new("cannot compare unit values")),
            },
            Ne => match operand_ty {
                Type::I32 | Type::Bool => "i32.ne",
                Type::I64 => "i64.ne",
                Type::F32 => "f32.ne",
                Type::F64 => "f64.ne",
                Type::Unit => return Err(CompileError::new("cannot compare unit values")),
            },
            Lt => match operand_ty {
                Type::I32 => "i32.lt_s",
                Type::I64 => "i64.lt_s",
                Type::F32 => "f32.lt",
                Type::F64 => "f64.lt",
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Le => match operand_ty {
                Type::I32 => "i32.le_s",
                Type::I64 => "i64.le_s",
                Type::F32 => "f32.le",
                Type::F64 => "f64.le",
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Gt => match operand_ty {
                Type::I32 => "i32.gt_s",
                Type::I64 => "i64.gt_s",
                Type::F32 => "f32.gt",
                Type::F64 => "f64.gt",
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Ge => match operand_ty {
                Type::I32 => "i32.ge_s",
                Type::I64 => "i64.ge_s",
                Type::F32 => "f32.ge",
                Type::F64 => "f64.ge",
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            And | Or => unreachable!(),
        };
        self.push_line(op);
        Ok(())
    }

    fn emit_logical(&mut self, expr: &hir::BinaryExpr) -> Result<(), CompileError> {
        match expr.op {
            BinaryOp::And => {
                self.emit_expression(&expr.left)?;
                self.push_line("(if (result i32)");
                self.indent += 1;
                self.push_line("(then");
                self.indent += 1;
                self.emit_expression(&expr.right)?;
                self.indent -= 1;
                self.push_line(")");
                self.push_line("(else");
                self.indent += 1;
                self.push_line("i32.const 0");
                self.indent -= 1;
                self.push_line(")");
                self.indent -= 1;
                self.push_line(")");
                Ok(())
            }
            BinaryOp::Or => {
                self.emit_expression(&expr.left)?;
                self.push_line("(if (result i32)");
                self.indent += 1;
                self.push_line("(then");
                self.indent += 1;
                self.push_line("i32.const 1");
                self.indent -= 1;
                self.push_line(")");
                self.push_line("(else");
                self.indent += 1;
                self.emit_expression(&expr.right)?;
                self.indent -= 1;
                self.push_line(")");
                self.indent -= 1;
                self.push_line(")");
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn emit_unary(&mut self, expr: &hir::UnaryExpr) -> Result<(), CompileError> {
        self.emit_expression(&expr.expr)?;
        match (expr.op, expr.ty) {
            (UnaryOp::Neg, Type::I32) => {
                self.push_line("i32.const -1");
                self.push_line("i32.mul");
                Ok(())
            }
            (UnaryOp::Neg, Type::I64) => {
                self.push_line("i64.const -1");
                self.push_line("i64.mul");
                Ok(())
            }
            (UnaryOp::Neg, Type::F32) => {
                self.push_line("f32.neg");
                Ok(())
            }
            (UnaryOp::Neg, Type::F64) => {
                self.push_line("f64.neg");
                Ok(())
            }
            (UnaryOp::Neg, _) => Err(CompileError::new(
                "`-` operator only valid for numeric types",
            )),
            (UnaryOp::Not, Type::Bool) => {
                self.push_line("i32.eqz");
                Ok(())
            }
            (UnaryOp::Not, _) => Err(CompileError::new("`!` operator only valid for bool")),
        }
    }

    fn emit_if(&mut self, if_expr: &hir::IfExpr) -> Result<(), CompileError> {
        self.emit_expression(&if_expr.condition)?;
        let has_result = if_expr.ty != Type::Unit;
        if has_result {
            self.push_line(&format!("(if (result {})", wasm_value_type(if_expr.ty)?));
        } else {
            self.push_line("(if");
        }
        self.indent += 1;
        self.push_line("(then");
        self.indent += 1;
        self.emit_block(
            &if_expr.then_branch,
            if has_result {
                TailMode::Preserve
            } else {
                TailMode::Drop
            },
        )?;
        self.indent -= 1;
        self.push_line(")");
        self.push_line("(else");
        self.indent += 1;
        if let Some(else_expr) = &if_expr.else_branch {
            self.emit_expression(else_expr)?;
            if !has_result && else_expr.ty() != Type::Unit {
                self.push_line("drop");
            }
        } else if has_result {
            return Err(CompileError::new("if expression missing else branch"));
        }
        self.indent -= 1;
        self.push_line(")");
        self.indent -= 1;
        self.push_line(")");
        Ok(())
    }

    fn emit_loop(&mut self, loop_expr: &hir::LoopExpr) -> Result<(), CompileError> {
        let break_label = self.make_symbol("loop_break");
        let continue_label = self.make_symbol("loop_body");
        self.push_line(&format!("(block {}", break_label));
        self.indent += 1;
        self.push_line(&format!("(loop {}", continue_label));
        self.indent += 1;

        self.loop_stack.push(LoopLabels {
            break_label: break_label.clone(),
            continue_label: continue_label.clone(),
        });
        let body_result = self.emit_block(loop_expr.body.as_ref(), TailMode::Drop);
        self.loop_stack.pop();
        if let Err(err) = body_result {
            self.indent -= 1;
            self.indent -= 1;
            return Err(err);
        }

        self.push_line(&format!("br {}", continue_label));
        self.indent -= 1;
        self.push_line(")");
        self.indent -= 1;
        self.push_line(")");
        Ok(())
    }

    fn emit_while(&mut self, while_expr: &hir::WhileExpr) -> Result<(), CompileError> {
        let break_label = self.make_symbol("while_break");
        let continue_label = self.make_symbol("while_loop");
        self.push_line(&format!("(block {}", break_label));
        self.indent += 1;
        self.push_line(&format!("(loop {}", continue_label));
        self.indent += 1;

        self.emit_expression(&while_expr.condition)?;
        self.push_line("i32.eqz");
        self.push_line(&format!("br_if {}", break_label));

        self.loop_stack.push(LoopLabels {
            break_label: break_label.clone(),
            continue_label: continue_label.clone(),
        });
        let body_result = self.emit_block(while_expr.body.as_ref(), TailMode::Drop);
        self.loop_stack.pop();
        if let Err(err) = body_result {
            self.indent -= 1;
            self.indent -= 1;
            return Err(err);
        }

        self.push_line(&format!("br {}", continue_label));
        self.indent -= 1;
        self.push_line(")");
        self.indent -= 1;
        self.push_line(")");
        Ok(())
    }

    fn make_symbol(&mut self, prefix: &str) -> String {
        let sanitized = sanitize(prefix);
        let name = format!("${}_{:03}", sanitized, self.temp_counter);
        self.temp_counter += 1;
        name
    }

    fn insert_binding(&mut self, name: &str, wasm_name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), wasm_name);
        }
    }

    fn lookup_binding(&self, name: &str) -> Result<String, CompileError> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.clone());
            }
        }
        Err(CompileError::new(format!("unknown identifier `{}`", name)))
    }

    fn current_loop_labels(&self) -> Option<&LoopLabels> {
        self.loop_stack.last()
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn push_line(&mut self, line: &str) {
        let indent = "  ".repeat(self.indent);
        self.instructions.push(format!("{}{}", indent, line));
    }
}

fn wasm_value_type(ty: Type) -> Result<&'static str, CompileError> {
    match ty {
        Type::I32 | Type::Bool => Ok("i32"),
        Type::I64 => Ok("i64"),
        Type::F32 => Ok("f32"),
        Type::F64 => Ok("f64"),
        Type::Unit => Err(CompileError::new("unit type has no wasm representation")),
    }
}

fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect()
}
