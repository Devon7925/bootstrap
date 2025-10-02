use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named { name: String, span: Span },
    Unit { span: Span },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named { span, .. } => *span,
            TypeExpr::Unit { span } => *span,
        }
    }
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
pub struct ExpressionStatement {
    pub expr: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LetStatement {
    pub name: String,
    pub mutable: bool,
    pub ty: TypeExpr,
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
pub enum Expression {
    Literal(Literal),
    Variable(Variable),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    Group(GroupExpr),
    If(IfExpr),
    Block(Block),
    Loop(LoopExpr),
    While(WhileExpr),
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub value: LiteralValue,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(IntLiteral),
    Float(FloatLiteral),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub value: i64,
    pub suffix: Option<IntSuffix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    I32,
    I64,
}

#[derive(Debug, Clone)]
pub struct FloatLiteral {
    pub value: f64,
    pub suffix: Option<FloatSuffix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F32,
    F64,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
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
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GroupExpr {
    pub expr: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_branch: Block,
    pub else_branch: Option<Box<Expression>>, // else branch as expression for `else if`
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LoopExpr {
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub condition: Box<Expression>,
    pub body: Block,
    pub span: Span,
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(lit) => lit.span,
            Expression::Variable(var) => var.span,
            Expression::Binary(expr) => expr.span,
            Expression::Unary(expr) => expr.span,
            Expression::Call(call) => call.span,
            Expression::Group(group) => group.span,
            Expression::If(if_expr) => if_expr.span,
            Expression::Block(block) => block.span,
            Expression::Loop(loop_expr) => loop_expr.span,
            Expression::While(while_expr) => while_expr.span,
        }
    }
}
