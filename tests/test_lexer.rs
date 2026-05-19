use dsqlex::lexer::tokenize;
use dsqlex::tokens::TokenType;

#[test]
fn operators() {
    let tokens = tokenize("+ - * / = != < > <= >=").unwrap();
    let types: Vec<_> = tokens.iter().map(|t| &t.ty).collect();
    assert_eq!(
        types,
        vec![
            &TokenType::Plus, &TokenType::Minus, &TokenType::Multiply, &TokenType::Divide,
            &TokenType::Eq, &TokenType::Neq, &TokenType::Lt, &TokenType::Gt,
            &TokenType::Lte, &TokenType::Gte,
        ]
    );
}

#[test]
fn keywords() {
    let tokens = tokenize("SELECT CASE WHEN THEN ELSE END AND OR NULL TRUE FALSE").unwrap();
    let types: Vec<_> = tokens.iter().map(|t| &t.ty).collect();
    assert_eq!(
        types,
        vec![
            &TokenType::Select, &TokenType::Case, &TokenType::When, &TokenType::Then,
            &TokenType::Else, &TokenType::End, &TokenType::And, &TokenType::Or,
            &TokenType::Null, &TokenType::True, &TokenType::False,
        ]
    );
}

#[test]
fn case_insensitive_keywords() {
    let tokens = tokenize("select Case WHEN true false null").unwrap();
    assert_eq!(tokens[0].ty, TokenType::Select);
    assert_eq!(tokens[1].ty, TokenType::Case);
    assert_eq!(tokens[2].ty, TokenType::When);
    assert_eq!(tokens[3].ty, TokenType::True);
    assert_eq!(tokens[4].ty, TokenType::False);
    assert_eq!(tokens[5].ty, TokenType::Null);
}

#[test]
fn number_literals() {
    let tokens = tokenize("42 3.14 100.00").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "42");
    assert_eq!(tokens[1].text, "3.14");
    assert_eq!(tokens[2].text, "100.00");
}

#[test]
fn string_literals() {
    let tokens = tokenize("'hello' 'world' ''").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "hello");
    assert_eq!(tokens[1].text, "world");
    assert_eq!(tokens[2].text, "");
}

#[test]
fn identifiers() {
    let tokens = tokenize("amount currency_rate config.pricing.margin").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "amount");
    assert_eq!(tokens[1].text, "currency_rate");
    assert_eq!(tokens[2].text, "config.pricing.margin");
}

#[test]
fn function_keywords() {
    let tokens = tokenize("ROUND COALESCE NVL UPPER LOWER ABS CONCAT EVENT").unwrap();
    assert_eq!(tokens[0].ty, TokenType::FnRound);
    assert_eq!(tokens[1].ty, TokenType::FnCoalesce);
    assert_eq!(tokens[2].ty, TokenType::FnCoalesce); // NVL alias
    assert_eq!(tokens[3].ty, TokenType::FnUpper);
    assert_eq!(tokens[4].ty, TokenType::FnLower);
    assert_eq!(tokens[5].ty, TokenType::FnAbs);
    assert_eq!(tokens[6].ty, TokenType::FnConcat);
    assert_eq!(tokens[7].ty, TokenType::FnEvent);
}

#[test]
fn line_comments() {
    let t = tokenize("amount -- this is a comment\n+ rate").unwrap();
    assert_eq!(t.len(), 3);
}

#[test]
fn hash_comments() {
    let t = tokenize("amount # hash comment\n+ rate").unwrap();
    assert_eq!(t.len(), 3);
}

#[test]
fn block_comments() {
    let t = tokenize("amount /* block comment */ + rate").unwrap();
    assert_eq!(t.len(), 3);
}

#[test]
fn unterminated_string_is_error() {
    assert!(tokenize("'unterminated").is_err());
}

#[test]
fn unterminated_block_comment_is_error() {
    assert!(tokenize("/* unterminated").is_err());
}

#[test]
fn unexpected_character_is_error() {
    assert!(tokenize("amount @ rate").is_err());
}

#[test]
fn structural_tokens() {
    let tokens = tokenize("( ) ,").unwrap();
    assert_eq!(tokens[0].ty, TokenType::LParen);
    assert_eq!(tokens[1].ty, TokenType::RParen);
    assert_eq!(tokens[2].ty, TokenType::Comma);
}

#[test]
fn is_in_like_keywords() {
    let tokens = tokenize("IS IN LIKE NOT").unwrap();
    assert_eq!(tokens[0].ty, TokenType::Is);
    assert_eq!(tokens[1].ty, TokenType::In);
    assert_eq!(tokens[2].ty, TokenType::Like);
    assert_eq!(tokens[3].ty, TokenType::Not);
}
