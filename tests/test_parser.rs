use dsqlex::ast::{AstNode, BinOp};

#[test]
fn simple_identifier() {
    let ast = dsqlex::parse("field1").unwrap();
    match ast {
        AstNode::Select(inner) => match *inner {
            AstNode::Identifier(ref name) => assert_eq!(name.as_ref(), "field1"),
            _ => panic!("Expected Identifier"),
        },
        _ => panic!("Expected Select"),
    }
}

#[test]
fn select_keyword_is_optional() {
    let _a = dsqlex::parse("SELECT amount").unwrap();
    let _b = dsqlex::parse("amount").unwrap();
}

#[test]
fn binary_arithmetic() {
    let ast = dsqlex::parse("a + b").unwrap();
    match ast {
        AstNode::Select(inner) => match *inner {
            AstNode::BinaryOp { ref op, .. } => assert_eq!(*op, BinOp::Plus),
            _ => panic!("Expected BinaryOp"),
        },
        _ => panic!("Expected Select"),
    }
}

#[test]
fn arithmetic_chaining_is_left_associative() {
    let _ast = dsqlex::parse("a + b + c").unwrap();
    let _ast = dsqlex::parse("a * b * c").unwrap();
}

#[test]
fn mixed_arithmetic_rejected() {
    assert!(dsqlex::parse("a + b * c").is_err());
    assert!(dsqlex::parse("a * b + c").is_err());
}

#[test]
fn parentheses_allow_mixed_arithmetic() {
    let _ast = dsqlex::parse("(a + b) * c").unwrap();
    let _ast = dsqlex::parse("a * (b + c)").unwrap();
}

#[test]
fn case_when_else_end() {
    let _ast = dsqlex::parse("CASE WHEN x = 1 THEN 'one' ELSE 'other' END").unwrap();
}

#[test]
fn case_multiple_when_no_else() {
    let _ast =
        dsqlex::parse("CASE WHEN x = 1 THEN 'a' WHEN x = 2 THEN 'b' END").unwrap();
}

#[test]
fn function_call() {
    let _ast = dsqlex::parse("ROUND(amount, 2)").unwrap();
}

#[test]
fn nested_function_call() {
    let _ast = dsqlex::parse("ROUND(COALESCE(x, 0), 2)").unwrap();
}

#[test]
fn in_expression() {
    let _ast = dsqlex::parse("status IN ('active', 'pending')").unwrap();
}

#[test]
fn not_in_expression() {
    let _ast = dsqlex::parse("status NOT IN ('deleted')").unwrap();
}

#[test]
fn like_expression() {
    let _ast = dsqlex::parse("name LIKE '%test%'").unwrap();
}

#[test]
fn not_like_expression() {
    let _ast = dsqlex::parse("name NOT LIKE '%test%'").unwrap();
}

#[test]
fn is_null() {
    let _ast = dsqlex::parse("x IS NULL").unwrap();
}

#[test]
fn is_not_null() {
    let _ast = dsqlex::parse("x IS NOT NULL").unwrap();
}

#[test]
fn is_true_false() {
    let _ast = dsqlex::parse("x IS TRUE").unwrap();
    let _ast = dsqlex::parse("x IS FALSE").unwrap();
    let _ast = dsqlex::parse("x IS NOT TRUE").unwrap();
}

#[test]
fn mixed_logical_operators_rejected() {
    assert!(dsqlex::parse("a = 1 AND b = 2 OR c = 3").is_err());
}

#[test]
fn same_logical_operator_chaining() {
    let _ast = dsqlex::parse("a = 1 AND b = 2 AND c = 3").unwrap();
    let _ast = dsqlex::parse("a = 1 OR b = 2 OR c = 3").unwrap();
}

#[test]
fn comparison_chaining_rejected() {
    assert!(dsqlex::parse("a = 1 = 2").is_err());
    assert!(dsqlex::parse("a < b < c").is_err());
}

#[test]
fn empty_expression_is_error() {
    assert!(dsqlex::parse("").is_err());
}

#[test]
fn trailing_tokens_is_error() {
    assert!(dsqlex::parse("a b").is_err());
}
