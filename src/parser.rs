use crate::ast::{AstNode, BinOp, WhenClause};
use crate::tokens::{Token, TokenType};
use crate::{DsqlexError, Result};
use rust_decimal::prelude::*;
use std::rc::Rc;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline]
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    #[inline]
    fn peek_type(&self) -> Option<&TokenType> {
        self.peek().map(|t| &t.ty)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn expect(&mut self, ty: &TokenType) -> Result<Token> {
        match self.advance() {
            Some(tok) if &tok.ty == ty => Ok(tok),
            Some(tok) => Err(DsqlexError(format!(
                "Expected {:?}, got {:?}",
                ty, tok.ty
            ))),
            None => Err(DsqlexError(format!("Expected {:?}, got end of input", ty))),
        }
    }

    #[inline]
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    // program := [SELECT] logical EOF
    fn parse_program(&mut self) -> Result<AstNode> {
        if self.peek_type() == Some(&TokenType::Select) {
            self.advance();
        }
        let expr = self.parse_logical()?;
        if !self.at_end() {
            return Err(DsqlexError(format!(
                "Unexpected token after expression: {:?}",
                self.peek().unwrap().ty
            )));
        }
        Ok(AstNode::Select(Box::new(expr)))
    }

    // logical := comparison ( (AND|OR) comparison )*
    fn parse_logical(&mut self) -> Result<AstNode> {
        let mut left = self.parse_comparison()?;

        let first_op = match self.peek_type() {
            Some(TokenType::And) => Some(TokenType::And),
            Some(TokenType::Or) => Some(TokenType::Or),
            _ => None,
        };

        if let Some(ref expected) = first_op {
            while self.peek_type() == Some(expected) {
                self.advance();
                let right = self.parse_comparison()?;
                let op = if *expected == TokenType::And {
                    BinOp::And
                } else {
                    BinOp::Or
                };
                left = AstNode::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };

                match self.peek_type() {
                    Some(TokenType::And) if *expected == TokenType::Or => {
                        return Err(DsqlexError(
                            "Ambiguous expression: mix of AND and OR requires parentheses".into(),
                        ));
                    }
                    Some(TokenType::Or) if *expected == TokenType::And => {
                        return Err(DsqlexError(
                            "Ambiguous expression: mix of AND and OR requires parentheses".into(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<AstNode> {
        let left = self.parse_arithmetic()?;

        match self.peek_type() {
            Some(TokenType::Eq)
            | Some(TokenType::Neq)
            | Some(TokenType::Lt)
            | Some(TokenType::Gt)
            | Some(TokenType::Lte)
            | Some(TokenType::Gte) => {
                let op_tok = self.advance().unwrap();
                let right = self.parse_arithmetic()?;
                let op = match op_tok.ty {
                    TokenType::Eq => BinOp::Eq,
                    TokenType::Neq => BinOp::Neq,
                    TokenType::Lt => BinOp::Lt,
                    TokenType::Gt => BinOp::Gt,
                    TokenType::Lte => BinOp::Lte,
                    TokenType::Gte => BinOp::Gte,
                    _ => unreachable!(),
                };
                match self.peek_type() {
                    Some(TokenType::Eq)
                    | Some(TokenType::Neq)
                    | Some(TokenType::Lt)
                    | Some(TokenType::Gt)
                    | Some(TokenType::Lte)
                    | Some(TokenType::Gte) => {
                        return Err(DsqlexError(
                            "Cannot chain comparison operators".into(),
                        ));
                    }
                    _ => {}
                }
                Ok(AstNode::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(TokenType::Is) => {
                self.advance();
                let negated = if self.peek_type() == Some(&TokenType::Not) {
                    self.advance();
                    true
                } else {
                    false
                };
                let right = match self.peek_type() {
                    Some(TokenType::Null) => {
                        self.advance();
                        AstNode::NullLit
                    }
                    Some(TokenType::True) => {
                        self.advance();
                        AstNode::BoolLit(true)
                    }
                    Some(TokenType::False) => {
                        self.advance();
                        AstNode::BoolLit(false)
                    }
                    _ => {
                        return Err(DsqlexError(
                            "Expected NULL, TRUE, or FALSE after IS [NOT]".into(),
                        ));
                    }
                };
                let op = if negated { BinOp::Neq } else { BinOp::Eq };
                Ok(AstNode::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(TokenType::Not) => {
                self.advance();
                match self.peek_type() {
                    Some(TokenType::In) => {
                        self.advance();
                        let items = self.parse_in_list()?;
                        Ok(AstNode::NotInExpr {
                            expr: Box::new(left),
                            items,
                        })
                    }
                    Some(TokenType::Like) => {
                        self.advance();
                        let pattern = self.parse_primary()?;
                        Ok(AstNode::NotLikeExpr {
                            expr: Box::new(left),
                            pattern: Box::new(pattern),
                        })
                    }
                    _ => Err(DsqlexError("Expected IN or LIKE after NOT".into())),
                }
            }
            Some(TokenType::In) => {
                self.advance();
                let items = self.parse_in_list()?;
                Ok(AstNode::InExpr {
                    expr: Box::new(left),
                    items,
                })
            }
            Some(TokenType::Like) => {
                self.advance();
                let pattern = self.parse_primary()?;
                Ok(AstNode::LikeExpr {
                    expr: Box::new(left),
                    pattern: Box::new(pattern),
                })
            }
            _ => Ok(left),
        }
    }

    fn parse_in_list(&mut self) -> Result<Vec<AstNode>> {
        self.expect(&TokenType::LParen)?;
        let mut items = vec![self.parse_logical()?];
        while self.peek_type() == Some(&TokenType::Comma) {
            self.advance();
            items.push(self.parse_logical()?);
        }
        self.expect(&TokenType::RParen)?;
        Ok(items)
    }

    fn parse_arithmetic(&mut self) -> Result<AstNode> {
        let mut left = self.parse_primary()?;

        #[derive(PartialEq, Clone, Copy)]
        enum ArithGroup {
            Additive,
            Multiplicative,
        }

        fn group_of(ty: &TokenType) -> Option<ArithGroup> {
            match ty {
                TokenType::Plus | TokenType::Minus => Some(ArithGroup::Additive),
                TokenType::Multiply | TokenType::Divide => Some(ArithGroup::Multiplicative),
                _ => None,
            }
        }

        let first_group = self.peek_type().and_then(group_of);

        if let Some(expected_group) = first_group {
            while let Some(g) = self.peek_type().and_then(group_of) {
                if g != expected_group {
                    return Err(DsqlexError(
                        "Ambiguous expression: mix of +/- and */÷ requires parentheses".into(),
                    ));
                }
                let op_tok = self.advance().unwrap();
                let right = self.parse_primary()?;
                let op = match op_tok.ty {
                    TokenType::Plus => BinOp::Plus,
                    TokenType::Minus => BinOp::Minus,
                    TokenType::Multiply => BinOp::Multiply,
                    TokenType::Divide => BinOp::Divide,
                    _ => unreachable!(),
                };
                left = AstNode::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<AstNode> {
        match self.peek_type().cloned() {
            Some(TokenType::Number) => {
                let tok = self.advance().unwrap();
                let d = Decimal::from_str(&tok.text)
                    .map_err(|e| DsqlexError(format!("Invalid number '{}': {}", tok.text, e)))?;
                Ok(AstNode::NumberLit(d))
            }
            Some(TokenType::String) => {
                let tok = self.advance().unwrap();
                Ok(AstNode::StringLit(tok.text.into()))
            }
            Some(TokenType::True) => {
                self.advance();
                Ok(AstNode::BoolLit(true))
            }
            Some(TokenType::False) => {
                self.advance();
                Ok(AstNode::BoolLit(false))
            }
            Some(TokenType::Null) => {
                self.advance();
                Ok(AstNode::NullLit)
            }
            Some(TokenType::Identifier) => {
                let tok = self.advance().unwrap();
                Ok(AstNode::Identifier(tok.text.into()))
            }
            Some(TokenType::LParen) => {
                self.advance();
                let expr = self.parse_logical()?;
                self.expect(&TokenType::RParen)?;
                Ok(expr)
            }
            Some(TokenType::Case) => self.parse_case(),
            Some(
                TokenType::FnUpper
                | TokenType::FnLower
                | TokenType::FnRound
                | TokenType::FnCoalesce
                | TokenType::FnAbs
                | TokenType::FnConcat
                | TokenType::FnEvent,
            ) => self.parse_function_call(),
            Some(ty) => Err(DsqlexError(format!("Unexpected token: {:?}", ty))),
            None => Err(DsqlexError("Unexpected end of input".into())),
        }
    }

    fn parse_case(&mut self) -> Result<AstNode> {
        self.expect(&TokenType::Case)?;
        let mut whens = Vec::new();

        while self.peek_type() == Some(&TokenType::When) {
            self.advance();
            let condition = self.parse_logical()?;
            self.expect(&TokenType::Then)?;
            let result = self.parse_logical()?;
            whens.push(WhenClause {
                condition: Box::new(condition),
                result: Box::new(result),
            });
        }

        if whens.is_empty() {
            return Err(DsqlexError("CASE requires at least one WHEN clause".into()));
        }

        let else_clause = if self.peek_type() == Some(&TokenType::Else) {
            self.advance();
            Some(Box::new(self.parse_logical()?))
        } else {
            None
        };

        self.expect(&TokenType::End)?;

        Ok(AstNode::CaseExpr { whens, else_clause })
    }

    fn parse_function_call(&mut self) -> Result<AstNode> {
        let tok = self.advance().unwrap();
        let name: Rc<str> = match tok.ty {
            TokenType::FnUpper => "UPPER".into(),
            TokenType::FnLower => "LOWER".into(),
            TokenType::FnRound => "ROUND".into(),
            TokenType::FnCoalesce => "COALESCE".into(),
            TokenType::FnAbs => "ABS".into(),
            TokenType::FnConcat => "CONCAT".into(),
            TokenType::FnEvent => "EVENT".into(),
            _ => unreachable!(),
        };

        self.expect(&TokenType::LParen)?;

        let mut args = Vec::new();
        if self.peek_type() != Some(&TokenType::RParen) {
            args.push(self.parse_logical()?);
            while self.peek_type() == Some(&TokenType::Comma) {
                self.advance();
                args.push(self.parse_logical()?);
            }
        }

        self.expect(&TokenType::RParen)?;

        Ok(AstNode::FunctionCall { name, args })
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<AstNode> {
    let mut parser = Parser::new(tokens);
    if parser.at_end() {
        return Err(DsqlexError("Empty expression".into()));
    }
    parser.parse_program()
}
