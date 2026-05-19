use crate::tokens::{Token, TokenType};
use crate::{DsqlexError, Result};

fn keyword_lookup(word: &str) -> Option<TokenType> {
    match word.to_ascii_uppercase().as_str() {
        "SELECT" => Some(TokenType::Select),
        "CASE" => Some(TokenType::Case),
        "WHEN" => Some(TokenType::When),
        "THEN" => Some(TokenType::Then),
        "ELSE" => Some(TokenType::Else),
        "END" => Some(TokenType::End),
        "AND" => Some(TokenType::And),
        "OR" => Some(TokenType::Or),
        "NOT" => Some(TokenType::Not),
        "NULL" => Some(TokenType::Null),
        "TRUE" => Some(TokenType::True),
        "FALSE" => Some(TokenType::False),
        "IS" => Some(TokenType::Is),
        "IN" => Some(TokenType::In),
        "LIKE" => Some(TokenType::Like),
        "UPPER" => Some(TokenType::FnUpper),
        "LOWER" => Some(TokenType::FnLower),
        "ROUND" => Some(TokenType::FnRound),
        "COALESCE" | "NVL" => Some(TokenType::FnCoalesce),
        "ABS" => Some(TokenType::FnAbs),
        "CONCAT" => Some(TokenType::FnConcat),
        "EVENT" => Some(TokenType::FnEvent),
        _ => None,
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Skip whitespace
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comments: -- or #
        if c == '#' || (c == '-' && i + 1 < len && chars[i + 1] == '-') {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Block comments: /* ... */
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            loop {
                if i + 1 >= len {
                    return Err(DsqlexError("Unterminated block comment".into()));
                }
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Two-char operators
        if i + 1 < len {
            let two = &input[i..i + 2];
            match two {
                "!=" => { tokens.push(Token::simple(TokenType::Neq)); i += 2; continue; }
                "<=" => { tokens.push(Token::simple(TokenType::Lte)); i += 2; continue; }
                ">=" => { tokens.push(Token::simple(TokenType::Gte)); i += 2; continue; }
                _ => {}
            }
        }

        // Single-char operators/structural
        match c {
            '+' => { tokens.push(Token::simple(TokenType::Plus)); i += 1; continue; }
            '-' => { tokens.push(Token::simple(TokenType::Minus)); i += 1; continue; }
            '*' => { tokens.push(Token::simple(TokenType::Multiply)); i += 1; continue; }
            '/' => { tokens.push(Token::simple(TokenType::Divide)); i += 1; continue; }
            '=' => { tokens.push(Token::simple(TokenType::Eq)); i += 1; continue; }
            '<' => { tokens.push(Token::simple(TokenType::Lt)); i += 1; continue; }
            '>' => { tokens.push(Token::simple(TokenType::Gt)); i += 1; continue; }
            '(' => { tokens.push(Token::simple(TokenType::LParen)); i += 1; continue; }
            ')' => { tokens.push(Token::simple(TokenType::RParen)); i += 1; continue; }
            ',' => { tokens.push(Token::simple(TokenType::Comma)); i += 1; continue; }
            _ => {}
        }

        // String literal
        if c == '\'' {
            i += 1;
            let start = i;
            while i < len && chars[i] != '\'' {
                i += 1;
            }
            if i >= len {
                return Err(DsqlexError("Unterminated string literal".into()));
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(Token::new(TokenType::String, text));
            i += 1; // skip closing quote
            continue;
        }

        // Number literal
        if c.is_ascii_digit() {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(Token::new(TokenType::Number, text));
            continue;
        }

        // Identifier or keyword
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if let Some(kw) = keyword_lookup(&text) {
                tokens.push(Token::new(kw, text));
            } else {
                tokens.push(Token::new(TokenType::Identifier, text));
            }
            continue;
        }

        return Err(DsqlexError(format!("Unexpected character: '{}'", c)));
    }

    tokens
        .into_iter()
        .collect::<Vec<_>>()
        .pipe(Ok)
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R where F: FnOnce(Self) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}
