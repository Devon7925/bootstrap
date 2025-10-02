use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub tail: Option<Box<Expression>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStatement),
    Assign(AssignStatement),
    Return(ReturnStatement),
    Expr(ExpressionStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
}

#[derive(Debug, Clone)]
pub struct LetStatement {
    pub name: String,
    pub mutable: bool,
    pub ty: Type,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignStatement {
    pub target: String,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expr: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Variable(Variable),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    If(IfExpr),
    Block(Block),
    Loop(LoopExpr),
    While(WhileExpr),
}

impl Expression {
    pub fn ty(&self) -> Type {
        match self {
            Expression::Literal(lit) => lit.ty,
            Expression::Variable(var) => var.ty,
            Expression::Binary(expr) => expr.ty,
            Expression::Unary(expr) => expr.ty,
            Expression::Call(call) => call.ty,
            Expression::If(expr) => expr.ty,
            Expression::Block(block) => block.ty(),
            Expression::Loop(expr) => expr.ty,
            Expression::While(expr) => expr.ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub value: LiteralValue,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expression>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: String,
    pub args: Vec<Expression>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_branch: Box<Block>,
    pub else_branch: Option<Box<Expression>>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LoopExpr {
    pub body: Box<Block>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub condition: Box<Expression>,
    pub body: Box<Block>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Unit,
}

impl Type {
    pub fn is_numeric(self) -> bool {
        matches!(self, Type::I32 | Type::I64 | Type::F32 | Type::F64)
    }

    pub fn is_integer(self) -> bool {
        matches!(self, Type::I32 | Type::I64)
    }
}

impl Block {
    pub fn ty(&self) -> Type {
        self.tail
            .as_ref()
            .map(|expr| expr.ty())
            .unwrap_or(Type::Unit)
    }
}
