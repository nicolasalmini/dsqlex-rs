use rust_decimal::Decimal;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct WhenClause {
    pub condition: Box<AstNode>,
    pub result: Box<AstNode>,
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Select(Box<AstNode>),
    NumberLit(Decimal),    // Pre-parsed at parse time — no conversion at eval time
    StringLit(Rc<str>),    // Rc<str> — clone is refcount bump, not heap alloc
    BoolLit(bool),
    NullLit,
    Identifier(Rc<str>),   // Rc<str> for the identifier name
    BinaryOp {
        op: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    CaseExpr {
        whens: Vec<WhenClause>,
        else_clause: Option<Box<AstNode>>,
    },
    FunctionCall {
        name: Rc<str>,
        args: Vec<AstNode>,
    },
    InExpr {
        expr: Box<AstNode>,
        items: Vec<AstNode>,
    },
    NotInExpr {
        expr: Box<AstNode>,
        items: Vec<AstNode>,
    },
    LikeExpr {
        expr: Box<AstNode>,
        pattern: Box<AstNode>,
    },
    NotLikeExpr {
        expr: Box<AstNode>,
        pattern: Box<AstNode>,
    },
}
