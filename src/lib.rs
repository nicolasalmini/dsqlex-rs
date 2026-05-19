pub mod tokens;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod evaluator;

use evaluator::{Context, EvalOptions, Value};

#[derive(Debug, Clone)]
pub struct DsqlexError(pub String);

impl std::fmt::Display for DsqlexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DsqlexError {}

pub type Result<T> = std::result::Result<T, DsqlexError>;

/// Parse an expression string into an AST.
pub fn parse(expression: &str) -> Result<ast::AstNode> {
    let tokens = lexer::tokenize(expression)?;
    parser::parse(tokens)
}

/// Evaluate a pre-parsed AST with the given context.
pub fn eval(ast: &ast::AstNode, ctx: &Context) -> Result<Value> {
    evaluator::evaluate(ast, ctx, &EvalOptions::default())
}

/// Evaluate a pre-parsed AST with options (resolver, event_resolver).
pub fn eval_with_options(ast: &ast::AstNode, ctx: &Context, opts: &EvalOptions) -> Result<Value> {
    evaluator::evaluate(ast, ctx, opts)
}

/// Parse and evaluate in one call.
pub fn eval_string(expression: &str, ctx: &Context) -> Result<Value> {
    let ast = parse(expression)?;
    eval(&ast, ctx)
}

/// Parse and evaluate in one call, with options.
pub fn eval_string_with_options(
    expression: &str,
    ctx: &Context,
    opts: &EvalOptions,
) -> Result<Value> {
    let ast = parse(expression)?;
    eval_with_options(&ast, ctx, opts)
}
