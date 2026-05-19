#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    // Comparison
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    // Keywords
    Select,
    Case,
    When,
    Then,
    Else,
    End,
    And,
    Or,
    Not,
    Null,
    True,
    False,
    Is,
    In,
    Like,
    // Functions
    FnUpper,
    FnLower,
    FnRound,
    FnCoalesce,
    FnAbs,
    FnConcat,
    FnEvent,
    // Literals & identifiers
    Number,
    String,
    Identifier,
    // Structural
    LParen,
    RParen,
    Comma,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub ty: TokenType,
    pub text: std::string::String,
}

impl Token {
    pub fn new(ty: TokenType, text: impl Into<std::string::String>) -> Self {
        Self { ty, text: text.into() }
    }

    pub fn simple(ty: TokenType) -> Self {
        Self { ty, text: std::string::String::new() }
    }
}
