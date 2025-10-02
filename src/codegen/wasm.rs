use std::collections::HashMap;

use crate::error::CompileError;
use crate::hir::{self, BinaryOp, Expression, LiteralValue, Statement, Type, UnaryOp};

pub struct WasmGenerator;

impl Default for WasmGenerator {
    fn default() -> Self {
        Self
    }
}

impl WasmGenerator {
    pub fn emit_program(&self, program: &hir::Program) -> Result<Vec<u8>, CompileError> {
        let mut func_types = Vec::new();
        let mut function_indices = HashMap::new();

        for (index, function) in program.functions.iter().enumerate() {
            let mut params = Vec::new();
            for param in &function.params {
                params.push(wasm_value_type(param.ty)?);
            }
            let result = if function.return_type == Type::Unit {
                None
            } else {
                Some(wasm_value_type(function.return_type)?)
            };
            func_types.push(FuncType { params, result });
            function_indices.insert(function.name.clone(), index as u32);
        }

        let mut bodies = Vec::new();
        for function in &program.functions {
            let mut emitter = FunctionEmitter::new(function, &function_indices);
            bodies.push(emitter.emit_function()?);
        }

        let mut module = Vec::new();
        module.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d]);
        module.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        let mut type_section = Vec::new();
        encode_u32(&mut type_section, func_types.len() as u32);
        for func_type in &func_types {
            type_section.push(0x60);
            encode_u32(&mut type_section, func_type.params.len() as u32);
            type_section.extend_from_slice(&func_type.params);
            match func_type.result {
                Some(ty) => {
                    type_section.push(0x01);
                    type_section.push(ty);
                }
                None => {
                    type_section.push(0x00);
                }
            }
        }
        push_section(&mut module, 1, &type_section);

        let mut function_section = Vec::new();
        encode_u32(&mut function_section, program.functions.len() as u32);
        for index in 0..program.functions.len() {
            encode_u32(&mut function_section, index as u32);
        }
        push_section(&mut module, 3, &function_section);

        let mut memory_section = Vec::new();
        encode_u32(&mut memory_section, 1);
        memory_section.push(0x00);
        encode_u32(&mut memory_section, 1);
        push_section(&mut module, 5, &memory_section);

        let mut export_section = Vec::new();
        encode_u32(&mut export_section, (program.functions.len() + 1) as u32);
        encode_name(&mut export_section, "memory");
        export_section.push(0x02);
        encode_u32(&mut export_section, 0);
        for (index, function) in program.functions.iter().enumerate() {
            encode_name(&mut export_section, &function.name);
            export_section.push(0x00);
            encode_u32(&mut export_section, index as u32);
        }
        push_section(&mut module, 7, &export_section);

        let mut code_section = Vec::new();
        encode_u32(&mut code_section, bodies.len() as u32);
        for body in &bodies {
            let mut body_buf = Vec::new();

            let mut grouped_locals: Vec<(u32, u8)> = Vec::new();
            for ty in &body.locals {
                let valtype = wasm_value_type_for_local(*ty)?;
                if let Some(last) = grouped_locals.last_mut() {
                    if last.1 == valtype {
                        last.0 += 1;
                        continue;
                    }
                }
                grouped_locals.push((1u32, valtype));
            }

            encode_u32(&mut body_buf, grouped_locals.len() as u32);
            for (count, ty) in grouped_locals {
                encode_u32(&mut body_buf, count);
                body_buf.push(ty);
            }

            body_buf.extend_from_slice(&body.code);
            body_buf.push(0x0b);

            encode_u32(&mut code_section, body_buf.len() as u32);
            code_section.extend_from_slice(&body_buf);
        }
        push_section(&mut module, 10, &code_section);

        Ok(module)
    }
}

struct FuncType {
    params: Vec<u8>,
    result: Option<u8>,
}

struct FunctionBody {
    locals: Vec<Type>,
    code: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Binding {
    index: u32,
}

struct LoopContext {
    break_index: usize,
    continue_index: usize,
    result_local: Option<u32>,
}

enum LabelKind {
    Block,
    Loop,
    If,
}

enum TailMode {
    Preserve,
    Drop,
}

struct FunctionEmitter<'a> {
    func: &'a hir::Function,
    function_indices: &'a HashMap<String, u32>,
    locals: Vec<Type>,
    scopes: Vec<HashMap<String, Binding>>,
    instructions: Vec<u8>,
    label_stack: Vec<LabelKind>,
    loop_stack: Vec<LoopContext>,
    param_count: u32,
}

impl<'a> FunctionEmitter<'a> {
    fn new(func: &'a hir::Function, function_indices: &'a HashMap<String, u32>) -> Self {
        Self {
            func,
            function_indices,
            locals: Vec::new(),
            scopes: Vec::new(),
            instructions: Vec::new(),
            label_stack: Vec::new(),
            loop_stack: Vec::new(),
            param_count: func.params.len() as u32,
        }
    }

    fn emit_function(&mut self) -> Result<FunctionBody, CompileError> {
        self.push_scope();
        for (index, param) in self.func.params.iter().enumerate() {
            self.insert_binding(
                &param.name,
                Binding {
                    index: index as u32,
                },
            );
        }

        let tail_mode = if self.func.return_type == Type::Unit {
            TailMode::Drop
        } else {
            TailMode::Preserve
        };
        self.emit_block(&self.func.body, tail_mode)?;
        self.pop_scope();

        Ok(FunctionBody {
            locals: self.locals.clone(),
            code: self.instructions.clone(),
        })
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
                        self.instructions.push(0x1a);
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
                let index = self.allocate_local(stmt.ty);
                self.insert_binding(&stmt.name, Binding { index });
                self.emit_expression(&stmt.value)?;
                if stmt.value.ty() == Type::Unit {
                    return Err(CompileError::new("let initializer cannot be unit"));
                }
                self.emit_local_set(index);
                Ok(())
            }
            Statement::Assign(stmt) => {
                let binding = self.lookup_binding(&stmt.target)?;
                self.emit_expression(&stmt.value)?;
                if stmt.value.ty() == Type::Unit {
                    return Err(CompileError::new("assignment value cannot be unit"));
                }
                self.emit_local_set(binding.index);
                Ok(())
            }
            Statement::Return(stmt) => {
                if let Some(expr) = &stmt.value {
                    self.emit_expression(expr)?;
                }
                self.instructions.push(0x0f);
                Ok(())
            }
            Statement::Expr(stmt) => {
                self.emit_expression(&stmt.expr)?;
                if stmt.expr.ty() != Type::Unit {
                    self.instructions.push(0x1a);
                }
                Ok(())
            }
            Statement::Break(stmt) => {
                let (break_index, result_local) = self
                    .loop_stack
                    .last()
                    .map(|labels| (labels.break_index, labels.result_local))
                    .ok_or_else(|| CompileError::new("`break` outside of loop"))?;
                if let Some(value) = &stmt.value {
                    self.emit_expression(value)?;
                    if let Some(local) = result_local {
                        self.emit_local_set(local);
                    }
                } else if result_local.is_some() {
                    return Err(
                        CompileError::new("`break` in this loop must provide a value")
                            .with_span(stmt.span),
                    );
                }
                self.emit_br(break_index);
                Ok(())
            }
            Statement::Continue(_) => {
                let labels = self
                    .loop_stack
                    .last()
                    .ok_or_else(|| CompileError::new("`continue` outside of loop"))?;
                self.emit_br(labels.continue_index);
                Ok(())
            }
        }
    }

    fn emit_expression(&mut self, expr: &Expression) -> Result<(), CompileError> {
        match expr {
            Expression::Literal(lit) => self.emit_literal(lit),
            Expression::Variable(var) => self.emit_variable(var),
            Expression::Binary(expr) => self.emit_binary(expr),
            Expression::Unary(expr) => self.emit_unary(expr),
            Expression::Call(call) => self.emit_call(call),
            Expression::If(if_expr) => self.emit_if(if_expr),
            Expression::Block(block) => self.emit_block(block, TailMode::Preserve),
            Expression::Loop(loop_expr) => self.emit_loop(loop_expr),
            Expression::While(while_expr) => self.emit_while(while_expr),
        }
    }

    fn emit_literal(&mut self, lit: &hir::Literal) -> Result<(), CompileError> {
        match (lit.ty, &lit.value) {
            (Type::I32, LiteralValue::Int(value)) => {
                self.instructions.push(0x41);
                encode_i32(&mut self.instructions, *value as i32);
                Ok(())
            }
            (Type::I64, LiteralValue::Int(value)) => {
                self.instructions.push(0x42);
                encode_i64(&mut self.instructions, *value);
                Ok(())
            }
            (Type::F32, LiteralValue::Float(value)) => {
                self.instructions.push(0x43);
                self.instructions
                    .extend_from_slice(&(*value as f32).to_bits().to_le_bytes());
                Ok(())
            }
            (Type::F64, LiteralValue::Float(value)) => {
                self.instructions.push(0x44);
                self.instructions
                    .extend_from_slice(&value.to_bits().to_le_bytes());
                Ok(())
            }
            (Type::Bool, LiteralValue::Bool(value)) => {
                self.instructions.push(0x41);
                encode_i32(&mut self.instructions, if *value { 1 } else { 0 });
                Ok(())
            }
            _ => Err(CompileError::new("invalid literal representation")),
        }
    }

    fn emit_variable(&mut self, var: &hir::Variable) -> Result<(), CompileError> {
        let binding = self.lookup_binding(&var.name)?;
        self.emit_local_get(binding.index);
        Ok(())
    }

    fn emit_binary(&mut self, expr: &hir::BinaryExpr) -> Result<(), CompileError> {
        use BinaryOp::*;
        if matches!(expr.op, And | Or) {
            return self.emit_logical(expr);
        }
        let operand_ty = expr.left.ty();
        self.emit_expression(&expr.left)?;
        self.emit_expression(&expr.right)?;
        let opcode = match expr.op {
            Add => match operand_ty {
                Type::I32 => 0x6a,
                Type::I64 => 0x7c,
                Type::F32 => 0x92,
                Type::F64 => 0xa0,
                _ => return Err(CompileError::new("unsupported add operand type")),
            },
            Sub => match operand_ty {
                Type::I32 => 0x6b,
                Type::I64 => 0x7d,
                Type::F32 => 0x93,
                Type::F64 => 0xa1,
                _ => return Err(CompileError::new("unsupported sub operand type")),
            },
            Mul => match operand_ty {
                Type::I32 => 0x6c,
                Type::I64 => 0x7e,
                Type::F32 => 0x94,
                Type::F64 => 0xa2,
                _ => return Err(CompileError::new("unsupported mul operand type")),
            },
            Div => match operand_ty {
                Type::I32 => 0x6d,
                Type::I64 => 0x7f,
                Type::F32 => 0x95,
                Type::F64 => 0xa3,
                _ => return Err(CompileError::new("unsupported div operand type")),
            },
            Rem => match operand_ty {
                Type::I32 => 0x6f,
                Type::I64 => 0x81,
                _ => return Err(CompileError::new("remainder not supported for this type")),
            },
            Eq => match operand_ty {
                Type::I32 | Type::Bool => 0x46,
                Type::I64 => 0x51,
                Type::F32 => 0x5b,
                Type::F64 => 0x61,
                Type::Unit => return Err(CompileError::new("cannot compare unit values")),
            },
            Ne => match operand_ty {
                Type::I32 | Type::Bool => 0x47,
                Type::I64 => 0x52,
                Type::F32 => 0x5c,
                Type::F64 => 0x62,
                Type::Unit => return Err(CompileError::new("cannot compare unit values")),
            },
            Lt => match operand_ty {
                Type::I32 => 0x48,
                Type::I64 => 0x53,
                Type::F32 => 0x5d,
                Type::F64 => 0x63,
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Le => match operand_ty {
                Type::I32 => 0x4c,
                Type::I64 => 0x57,
                Type::F32 => 0x5f,
                Type::F64 => 0x65,
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Gt => match operand_ty {
                Type::I32 => 0x4a,
                Type::I64 => 0x55,
                Type::F32 => 0x5e,
                Type::F64 => 0x64,
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            Ge => match operand_ty {
                Type::I32 => 0x4e,
                Type::I64 => 0x59,
                Type::F32 => 0x60,
                Type::F64 => 0x66,
                _ => return Err(CompileError::new("unsupported comparison operand type")),
            },
            BitAnd => match operand_ty {
                Type::I32 => 0x71,
                Type::I64 => 0x83,
                _ => return Err(CompileError::new("`&` only supports integer operands")),
            },
            BitOr => match operand_ty {
                Type::I32 => 0x72,
                Type::I64 => 0x84,
                _ => return Err(CompileError::new("`|` only supports integer operands")),
            },
            BitXor => match operand_ty {
                Type::I32 => 0x73,
                Type::I64 => 0x85,
                _ => return Err(CompileError::new("`^` only supports integer operands")),
            },
            Shl => match operand_ty {
                Type::I32 => 0x74,
                Type::I64 => 0x86,
                _ => return Err(CompileError::new("`<<` only supports integer operands")),
            },
            Shr => match operand_ty {
                Type::I32 => 0x75,
                Type::I64 => 0x87,
                _ => return Err(CompileError::new("`>>` only supports integer operands")),
            },
            And | Or => unreachable!(),
        };
        self.instructions.push(opcode);
        Ok(())
    }

    fn emit_logical(&mut self, expr: &hir::BinaryExpr) -> Result<(), CompileError> {
        match expr.op {
            BinaryOp::And => {
                self.emit_expression(&expr.left)?;
                self.start_if(Some(0x7f));
                self.emit_expression(&expr.right)?;
                self.start_else();
                self.instructions.push(0x41);
                encode_i32(&mut self.instructions, 0);
                self.end_block();
                Ok(())
            }
            BinaryOp::Or => {
                self.emit_expression(&expr.left)?;
                self.start_if(Some(0x7f));
                self.instructions.push(0x41);
                encode_i32(&mut self.instructions, 1);
                self.start_else();
                self.emit_expression(&expr.right)?;
                self.end_block();
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn emit_unary(&mut self, expr: &hir::UnaryExpr) -> Result<(), CompileError> {
        self.emit_expression(&expr.expr)?;
        match (expr.op, expr.ty) {
            (UnaryOp::Neg, Type::I32) => {
                self.instructions.push(0x41);
                encode_i32(&mut self.instructions, -1);
                self.instructions.push(0x6c);
                Ok(())
            }
            (UnaryOp::Neg, Type::I64) => {
                self.instructions.push(0x42);
                encode_i64(&mut self.instructions, -1);
                self.instructions.push(0x7e);
                Ok(())
            }
            (UnaryOp::Neg, Type::F32) => {
                self.instructions.push(0x8c);
                Ok(())
            }
            (UnaryOp::Neg, Type::F64) => {
                self.instructions.push(0x9a);
                Ok(())
            }
            (UnaryOp::Neg, _) => Err(CompileError::new(
                "`-` operator only valid for numeric types",
            )),
            (UnaryOp::Not, Type::Bool) => {
                self.instructions.push(0x45);
                Ok(())
            }
            (UnaryOp::Not, _) => Err(CompileError::new("`!` operator only valid for bool")),
        }
    }

    fn emit_call(&mut self, call: &hir::CallExpr) -> Result<(), CompileError> {
        for arg in &call.args {
            self.emit_expression(arg)?;
        }
        if self.emit_intrinsic_call(call)? {
            return Ok(());
        }
        let index = self
            .function_indices
            .get(&call.callee)
            .copied()
            .ok_or_else(|| CompileError::new(format!("unknown function `{}`", call.callee)))?;
        self.instructions.push(0x10);
        encode_u32(&mut self.instructions, index);
        Ok(())
    }

    fn emit_intrinsic_call(&mut self, call: &hir::CallExpr) -> Result<bool, CompileError> {
        match call.callee.as_str() {
            "load_u8" => {
                if call.args.len() != 1 {
                    return Err(CompileError::new(
                        "`load_u8` intrinsic expects a single pointer argument",
                    ));
                }
                self.instructions.push(0x2d);
                encode_u32(&mut self.instructions, 0);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "store_u8" => {
                if call.args.len() != 2 {
                    return Err(CompileError::new(
                        "`store_u8` intrinsic expects a pointer and value",
                    ));
                }
                self.instructions.push(0x3a);
                encode_u32(&mut self.instructions, 0);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "load_i32" => {
                if call.args.len() != 1 {
                    return Err(CompileError::new(
                        "`load_i32` intrinsic expects a single pointer argument",
                    ));
                }
                self.instructions.push(0x28);
                encode_u32(&mut self.instructions, 2);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "store_i32" => {
                if call.args.len() != 2 {
                    return Err(CompileError::new(
                        "`store_i32` intrinsic expects a pointer and value",
                    ));
                }
                self.instructions.push(0x36);
                encode_u32(&mut self.instructions, 2);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "load_i64" => {
                if call.args.len() != 1 {
                    return Err(CompileError::new(
                        "`load_i64` intrinsic expects a single pointer argument",
                    ));
                }
                self.instructions.push(0x29);
                encode_u32(&mut self.instructions, 3);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "store_i64" => {
                if call.args.len() != 2 {
                    return Err(CompileError::new(
                        "`store_i64` intrinsic expects a pointer and value",
                    ));
                }
                self.instructions.push(0x37);
                encode_u32(&mut self.instructions, 3);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "load_f32" => {
                if call.args.len() != 1 {
                    return Err(CompileError::new(
                        "`load_f32` intrinsic expects a single pointer argument",
                    ));
                }
                self.instructions.push(0x2a);
                encode_u32(&mut self.instructions, 2);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "store_f32" => {
                if call.args.len() != 2 {
                    return Err(CompileError::new(
                        "`store_f32` intrinsic expects a pointer and value",
                    ));
                }
                self.instructions.push(0x38);
                encode_u32(&mut self.instructions, 2);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "load_f64" => {
                if call.args.len() != 1 {
                    return Err(CompileError::new(
                        "`load_f64` intrinsic expects a single pointer argument",
                    ));
                }
                self.instructions.push(0x2b);
                encode_u32(&mut self.instructions, 3);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            "store_f64" => {
                if call.args.len() != 2 {
                    return Err(CompileError::new(
                        "`store_f64` intrinsic expects a pointer and value",
                    ));
                }
                self.instructions.push(0x39);
                encode_u32(&mut self.instructions, 3);
                encode_u32(&mut self.instructions, 0);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn emit_if(&mut self, if_expr: &hir::IfExpr) -> Result<(), CompileError> {
        self.emit_expression(&if_expr.condition)?;
        let result_type = if if_expr.ty == Type::Unit {
            None
        } else {
            Some(wasm_value_type(if_expr.ty)?)
        };
        self.start_if(result_type);
        self.emit_block(
            &if_expr.then_branch,
            if result_type.is_some() {
                TailMode::Preserve
            } else {
                TailMode::Drop
            },
        )?;
        if let Some(else_expr) = &if_expr.else_branch {
            self.start_else();
            if result_type.is_some() {
                self.emit_expression(else_expr)?;
            } else if let Expression::Block(block) = else_expr.as_ref() {
                self.emit_block(block, TailMode::Drop)?;
            } else {
                self.emit_expression(else_expr)?;
                if else_expr.ty() != Type::Unit {
                    self.instructions.push(0x1a);
                }
            }
        } else if result_type.is_some() {
            return Err(CompileError::new("if expression missing else branch"));
        }
        self.end_block();
        Ok(())
    }

    fn emit_loop(&mut self, loop_expr: &hir::LoopExpr) -> Result<(), CompileError> {
        let result_local = if loop_expr.ty == Type::Unit {
            None
        } else {
            Some(self.allocate_local(loop_expr.ty))
        };
        let block_index = self.start_block(None);
        let loop_index = self.start_loop();
        self.loop_stack.push(LoopContext {
            break_index: block_index,
            continue_index: loop_index,
            result_local,
        });
        let body_result = self.emit_block(loop_expr.body.as_ref(), TailMode::Drop);
        let context = self.loop_stack.pop().expect("loop context should exist");
        body_result?;
        self.emit_br(loop_index);
        self.end_block();
        self.end_block();
        if let Some(local) = context.result_local {
            self.emit_local_get(local);
        }
        Ok(())
    }

    fn emit_while(&mut self, while_expr: &hir::WhileExpr) -> Result<(), CompileError> {
        let block_index = self.start_block(None);
        let loop_index = self.start_loop();
        self.loop_stack.push(LoopContext {
            break_index: block_index,
            continue_index: loop_index,
            result_local: None,
        });

        self.emit_expression(&while_expr.condition)?;
        self.instructions.push(0x45);
        self.emit_br_if(block_index);

        self.emit_block(while_expr.body.as_ref(), TailMode::Drop)?;
        self.emit_br(loop_index);
        self.loop_stack.pop();
        self.end_block();
        self.end_block();
        Ok(())
    }

    fn emit_local_get(&mut self, index: u32) {
        self.instructions.push(0x20);
        encode_u32(&mut self.instructions, index);
    }

    fn emit_local_set(&mut self, index: u32) {
        self.instructions.push(0x21);
        encode_u32(&mut self.instructions, index);
    }

    fn emit_br(&mut self, target_index: usize) {
        self.instructions.push(0x0c);
        let depth = self.label_depth(target_index);
        encode_u32(&mut self.instructions, depth);
    }

    fn emit_br_if(&mut self, target_index: usize) {
        self.instructions.push(0x0d);
        let depth = self.label_depth(target_index);
        encode_u32(&mut self.instructions, depth);
    }

    fn start_block(&mut self, result: Option<u8>) -> usize {
        self.instructions.push(0x02);
        if let Some(result) = result {
            self.instructions.push(result);
        } else {
            self.instructions.push(0x40);
        }
        self.push_label(LabelKind::Block)
    }

    fn start_loop(&mut self) -> usize {
        self.instructions.push(0x03);
        self.instructions.push(0x40);
        self.push_label(LabelKind::Loop)
    }

    fn start_if(&mut self, result: Option<u8>) {
        self.instructions.push(0x04);
        if let Some(result) = result {
            self.instructions.push(result);
        } else {
            self.instructions.push(0x40);
        }
        self.push_label(LabelKind::If);
    }

    fn start_else(&mut self) {
        self.instructions.push(0x05);
    }

    fn end_block(&mut self) {
        self.instructions.push(0x0b);
        self.pop_label();
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_binding(&mut self, name: &str, binding: Binding) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), binding);
        }
    }

    fn lookup_binding(&self, name: &str) -> Result<Binding, CompileError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Ok(binding.clone());
            }
        }
        Err(CompileError::new(format!("unknown identifier `{}`", name)))
    }

    fn allocate_local(&mut self, ty: Type) -> u32 {
        let index = self.param_count + self.locals.len() as u32;
        self.locals.push(ty);
        index
    }

    fn push_label(&mut self, kind: LabelKind) -> usize {
        self.label_stack.push(kind);
        self.label_stack.len() - 1
    }

    fn pop_label(&mut self) {
        self.label_stack.pop();
    }

    fn label_depth(&self, target_index: usize) -> u32 {
        let current = self
            .label_stack
            .len()
            .checked_sub(1)
            .expect("label stack underflow");
        (current - target_index) as u32
    }
}

fn wasm_value_type(ty: Type) -> Result<u8, CompileError> {
    match ty {
        Type::I32 | Type::Bool => Ok(0x7f),
        Type::I64 => Ok(0x7e),
        Type::F32 => Ok(0x7d),
        Type::F64 => Ok(0x7c),
        Type::Unit => Err(CompileError::new("unit type has no wasm representation")),
    }
}

fn wasm_value_type_for_local(ty: Type) -> Result<u8, CompileError> {
    wasm_value_type(match ty {
        Type::Bool => Type::I32,
        other => other,
    })
}

fn push_section(module: &mut Vec<u8>, id: u8, contents: &[u8]) {
    module.push(id);
    encode_u32(module, contents.len() as u32);
    module.extend_from_slice(contents);
}

fn encode_name(buf: &mut Vec<u8>, name: &str) {
    encode_u32(buf, name.len() as u32);
    buf.extend_from_slice(name.as_bytes());
}

fn encode_u32(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn encode_i32(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        let done = (value == 0 && !sign_bit) || (value == -1 && sign_bit);
        if done {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

fn encode_i64(buf: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        let done = (value == 0 && !sign_bit) || (value == -1 && sign_bit);
        if done {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}
