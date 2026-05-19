use dsqlex::evaluator::{Context, EvalOptions, Value};
use dsqlex::lexer::tokenize;
use dsqlex::tokens::TokenType;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::rc::Rc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// ═══════════════════════════════════════════════════════════════
// LEXER TESTS
// ═══════════════════════════════════════════════════════════════

#[test]
fn lexer_simple_tokens() {
    let tokens = tokenize("+ - * / = != < > <= >=").unwrap();
    let types: Vec<_> = tokens.iter().map(|t| &t.ty).collect();
    assert_eq!(
        types,
        vec![
            &TokenType::Plus,
            &TokenType::Minus,
            &TokenType::Multiply,
            &TokenType::Divide,
            &TokenType::Eq,
            &TokenType::Neq,
            &TokenType::Lt,
            &TokenType::Gt,
            &TokenType::Lte,
            &TokenType::Gte,
        ]
    );
}

#[test]
fn lexer_keywords() {
    let tokens = tokenize("SELECT CASE WHEN THEN ELSE END AND OR NULL TRUE FALSE").unwrap();
    let types: Vec<_> = tokens.iter().map(|t| &t.ty).collect();
    assert_eq!(
        types,
        vec![
            &TokenType::Select,
            &TokenType::Case,
            &TokenType::When,
            &TokenType::Then,
            &TokenType::Else,
            &TokenType::End,
            &TokenType::And,
            &TokenType::Or,
            &TokenType::Null,
            &TokenType::True,
            &TokenType::False,
        ]
    );
}

#[test]
fn lexer_case_insensitive() {
    let tokens = tokenize("select Case WHEN true false null").unwrap();
    assert_eq!(tokens[0].ty, TokenType::Select);
    assert_eq!(tokens[1].ty, TokenType::Case);
    assert_eq!(tokens[2].ty, TokenType::When);
    assert_eq!(tokens[3].ty, TokenType::True);
    assert_eq!(tokens[4].ty, TokenType::False);
    assert_eq!(tokens[5].ty, TokenType::Null);
}

#[test]
fn lexer_numbers() {
    let tokens = tokenize("42 3.14 100.00").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "42");
    assert_eq!(tokens[1].text, "3.14");
    assert_eq!(tokens[2].text, "100.00");
}

#[test]
fn lexer_strings() {
    let tokens = tokenize("'hello' 'world' ''").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "hello");
    assert_eq!(tokens[1].text, "world");
    assert_eq!(tokens[2].text, "");
}

#[test]
fn lexer_identifiers() {
    let tokens = tokenize("amount currency_rate config.pricing.margin").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "amount");
    assert_eq!(tokens[1].text, "currency_rate");
    assert_eq!(tokens[2].text, "config.pricing.margin");
}

#[test]
fn lexer_functions() {
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
fn lexer_comments() {
    // Line comment
    let t1 = tokenize("amount -- this is a comment\n+ rate").unwrap();
    assert_eq!(t1.len(), 3); // amount + rate
    // Hash comment
    let t2 = tokenize("amount # hash comment\n+ rate").unwrap();
    assert_eq!(t2.len(), 3);
    // Block comment
    let t3 = tokenize("amount /* block comment */ + rate").unwrap();
    assert_eq!(t3.len(), 3);
}

#[test]
fn lexer_unterminated_string() {
    assert!(tokenize("'unterminated").is_err());
}

#[test]
fn lexer_unterminated_block_comment() {
    assert!(tokenize("/* unterminated").is_err());
}

// ═══════════════════════════════════════════════════════════════
// PARSER TESTS
// ═══════════════════════════════════════════════════════════════

#[test]
fn parser_simple_field() {
    let ast = dsqlex::parse("field1").unwrap();
    // Should be Select(Identifier("field1"))
    match ast {
        dsqlex::ast::AstNode::Select(inner) => match *inner {
            dsqlex::ast::AstNode::Identifier(ref name) => assert_eq!(name.as_ref(), "field1"),
            _ => panic!("Expected Identifier"),
        },
        _ => panic!("Expected Select"),
    }
}

#[test]
fn parser_select_optional() {
    // Both should parse successfully
    let _a = dsqlex::parse("SELECT amount").unwrap();
    let _b = dsqlex::parse("amount").unwrap();
}

#[test]
fn parser_arithmetic() {
    let ast = dsqlex::parse("a + b").unwrap();
    match ast {
        dsqlex::ast::AstNode::Select(inner) => match *inner {
            dsqlex::ast::AstNode::BinaryOp { ref op, .. } => {
                assert_eq!(*op, dsqlex::ast::BinOp::Plus);
            }
            _ => panic!("Expected BinaryOp"),
        },
        _ => panic!("Expected Select"),
    }
}

#[test]
fn parser_arithmetic_chaining() {
    // a + b + c should be left-associative: (a + b) + c
    let _ast = dsqlex::parse("a + b + c").unwrap();
}

#[test]
fn parser_mixed_arithmetic_rejected() {
    assert!(dsqlex::parse("a + b * c").is_err());
    assert!(dsqlex::parse("a * b + c").is_err());
}

#[test]
fn parser_parenthesized_mixed() {
    let _ast = dsqlex::parse("(a + b) * c").unwrap();
}

#[test]
fn parser_case() {
    let _ast = dsqlex::parse("CASE WHEN x = 1 THEN 'one' ELSE 'other' END").unwrap();
}

#[test]
fn parser_function() {
    let _ast = dsqlex::parse("ROUND(amount, 2)").unwrap();
}

#[test]
fn parser_in_expr() {
    let _ast = dsqlex::parse("status IN ('active', 'pending')").unwrap();
}

#[test]
fn parser_not_in_expr() {
    let _ast = dsqlex::parse("status NOT IN ('deleted')").unwrap();
}

#[test]
fn parser_like() {
    let _ast = dsqlex::parse("name LIKE '%test%'").unwrap();
}

#[test]
fn parser_not_like() {
    let _ast = dsqlex::parse("name NOT LIKE '%test%'").unwrap();
}

#[test]
fn parser_is_null() {
    let _ast = dsqlex::parse("x IS NULL").unwrap();
}

#[test]
fn parser_is_not_null() {
    let _ast = dsqlex::parse("x IS NOT NULL").unwrap();
}

#[test]
fn parser_mixed_logical_rejected() {
    assert!(dsqlex::parse("a = 1 AND b = 2 OR c = 3").is_err());
}

#[test]
fn parser_same_logical_ok() {
    let _ast = dsqlex::parse("a = 1 AND b = 2 AND c = 3").unwrap();
}

#[test]
fn parser_comparison_no_chain() {
    assert!(dsqlex::parse("a = 1 = 2").is_err());
}

// ═══════════════════════════════════════════════════════════════
// EVALUATOR TESTS
// ═══════════════════════════════════════════════════════════════

#[test]
fn eval_number_literal() {
    let result = dsqlex::eval_string("42", &Context::new()).unwrap();
    assert_eq!(result, Value::Decimal(dec("42")));
}

#[test]
fn eval_string_literal() {
    let result = dsqlex::eval_string("'hello'", &Context::new()).unwrap();
    assert_eq!(result, Value::String("hello".into()));
}

#[test]
fn eval_bool_literals() {
    assert_eq!(
        dsqlex::eval_string("TRUE", &Context::new()).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("FALSE", &Context::new()).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_null_literal() {
    assert_eq!(
        dsqlex::eval_string("NULL", &Context::new()).unwrap(),
        Value::Null
    );
}

#[test]
fn eval_field_lookup() {
    let mut ctx = Context::new();
    ctx.set_decimal("amount", "500.00");
    let result = dsqlex::eval_string("amount", &ctx).unwrap();
    assert_eq!(result, Value::Decimal(dec("500.00")));
}

#[test]
fn eval_arithmetic() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "10");
    ctx.set_decimal("b", "3");

    assert_eq!(
        dsqlex::eval_string("a + b", &ctx).unwrap(),
        Value::Decimal(dec("13"))
    );
    assert_eq!(
        dsqlex::eval_string("a - b", &ctx).unwrap(),
        Value::Decimal(dec("7"))
    );
    assert_eq!(
        dsqlex::eval_string("a * b", &ctx).unwrap(),
        Value::Decimal(dec("30"))
    );
}

#[test]
fn eval_division() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "10");
    ctx.set_decimal("b", "3");
    let result = dsqlex::eval_string("a / b", &ctx).unwrap();
    // Should be high-precision decimal ~3.333...
    if let Value::Decimal(d) = result {
        assert!(d > dec("3.33") && d < dec("3.34"));
    } else {
        panic!("Expected Decimal");
    }
}

#[test]
fn eval_comparison() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "10");
    ctx.set_decimal("b", "20");

    assert_eq!(dsqlex::eval_string("a = a", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("a != b", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("a < b", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("a > b", &ctx).unwrap(), Value::Bool(false));
    assert_eq!(dsqlex::eval_string("a <= a", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("a >= b", &ctx).unwrap(), Value::Bool(false));
}

#[test]
fn eval_string_comparison() {
    let mut ctx = Context::new();
    ctx.set_string("s", "hello");
    assert_eq!(
        dsqlex::eval_string("s = 'hello'", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("s != 'world'", &ctx).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn eval_null_comparison() {
    let mut ctx = Context::new();
    ctx.set_null("x");
    assert_eq!(
        dsqlex::eval_string("x IS NULL", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("x IS NOT NULL", &ctx).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_logical_and() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("TRUE AND TRUE", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("TRUE AND FALSE", &ctx).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        dsqlex::eval_string("FALSE AND TRUE", &ctx).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_logical_or() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("TRUE OR FALSE", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("FALSE OR FALSE", &ctx).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_case_simple() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    ctx.set_decimal("amount", "100");
    let result = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN amount ELSE 0 END",
        &ctx,
    )
    .unwrap();
    assert_eq!(result, Value::Decimal(dec("100")));
}

#[test]
fn eval_case_no_match_returns_null() {
    let mut ctx = Context::new();
    ctx.set_string("status", "unknown");
    let result = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN 1 WHEN status = 'pending' THEN 2 END",
        &ctx,
    )
    .unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn eval_round() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("ROUND(3.14159, 2)", &ctx).unwrap(),
        Value::Decimal(dec("3.14"))
    );
    assert_eq!(
        dsqlex::eval_string("ROUND(2.555, 2)", &ctx).unwrap(),
        Value::Decimal(dec("2.56"))
    );
}

#[test]
fn eval_coalesce() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("COALESCE(NULL, NULL, 42)", &ctx).unwrap(),
        Value::Decimal(dec("42"))
    );
    assert_eq!(
        dsqlex::eval_string("COALESCE(NULL, 'hello')", &ctx).unwrap(),
        Value::String("hello".into())
    );
    assert_eq!(
        dsqlex::eval_string("COALESCE(NULL, NULL)", &ctx).unwrap(),
        Value::Null
    );
}

#[test]
fn eval_upper_lower() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("UPPER('hello')", &ctx).unwrap(),
        Value::String("HELLO".into())
    );
    assert_eq!(
        dsqlex::eval_string("LOWER('HELLO')", &ctx).unwrap(),
        Value::String("hello".into())
    );
}

#[test]
fn eval_abs() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "-42.5");
    assert_eq!(
        dsqlex::eval_string("ABS(x)", &ctx).unwrap(),
        Value::Decimal(dec("42.5"))
    );
}

#[test]
fn eval_concat() {
    let mut ctx = Context::new();
    ctx.set_string("first", "Hello");
    ctx.set_string("last", "World");
    assert_eq!(
        dsqlex::eval_string("CONCAT(first, ' ', last)", &ctx).unwrap(),
        Value::String("Hello World".into())
    );
}

#[test]
fn eval_in() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    assert_eq!(
        dsqlex::eval_string("status IN ('active', 'pending')", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("status IN ('deleted', 'archived')", &ctx).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_not_in() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    assert_eq!(
        dsqlex::eval_string("status NOT IN ('deleted')", &ctx).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn eval_like() {
    let mut ctx = Context::new();
    ctx.set_string("name", "Hello World");
    assert_eq!(
        dsqlex::eval_string("name LIKE '%world%'", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("name LIKE 'hello%'", &ctx).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        dsqlex::eval_string("name LIKE '%xyz%'", &ctx).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn eval_not_like() {
    let mut ctx = Context::new();
    ctx.set_string("name", "Hello");
    assert_eq!(
        dsqlex::eval_string("name NOT LIKE '%xyz%'", &ctx).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn eval_dot_path() {
    let mut ctx = Context::new();
    let mut nested = Context::new();
    nested.set_decimal("rate", "5.00");
    ctx.set_nested("config", nested);
    assert_eq!(
        dsqlex::eval_string("config.rate", &ctx).unwrap(),
        Value::Decimal(dec("5.00"))
    );
}

#[test]
fn eval_nested_dot_path() {
    let mut ctx = Context::new();
    let mut pricing = Context::new();
    pricing.set_decimal("margin", "0.15");
    let mut config = Context::new();
    config.set_nested("pricing", pricing);
    ctx.set_nested("config", config);
    assert_eq!(
        dsqlex::eval_string("config.pricing.margin", &ctx).unwrap(),
        Value::Decimal(dec("0.15"))
    );
}

#[test]
fn eval_unknown_field() {
    assert!(dsqlex::eval_string("nonexistent", &Context::new()).is_err());
}

#[test]
fn eval_complex_expression() {
    let mut ctx = Context::new();
    ctx.set_decimal("price", "100.00");
    ctx.set_decimal("quantity", "5");
    ctx.set_decimal("discount_rate", "0.1");
    let result =
        dsqlex::eval_string("(price * quantity) + (price * quantity * discount_rate)", &ctx)
            .unwrap();
    assert_eq!(result, Value::Decimal(dec("550.000")));
}

#[test]
fn eval_real_world_case() {
    let mut ctx = Context::new();
    ctx.set_string("currency", "BRL");
    ctx.set_decimal("amount_local", "500.00");
    ctx.set_decimal("amount_usd", "100.00");
    let result = dsqlex::eval_string(
        "CASE WHEN currency = 'USD' THEN amount_usd WHEN currency = 'BRL' THEN amount_local ELSE NULL END",
        &ctx,
    )
    .unwrap();
    assert_eq!(result, Value::Decimal(dec("500.00")));
}

#[test]
fn eval_coalesce_with_field() {
    let mut ctx = Context::new();
    ctx.set_null("base_amount");
    ctx.set_decimal("rate", "1.5");
    let result = dsqlex::eval_string(
        "ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1), 4)",
        &ctx,
    )
    .unwrap();
    assert_eq!(result, Value::Decimal(dec("0.0000")));
}

#[test]
fn eval_resolver() {
    let mut ctx = Context::new();
    ctx.set_decimal("amount", "100");
    let opts = EvalOptions {
        resolver: Some(Box::new(|name: &str, _visited: &HashSet<Rc<str>>| {
            if name == "external_rate" {
                Ok(Value::Decimal(dec("1.5")))
            } else {
                Err(dsqlex::DsqlexError(format!("Unknown: {}", name)))
            }
        })),
        event_resolver: None,
        visited: HashSet::new(),
    };
    let ast = dsqlex::parse("amount * external_rate").unwrap();
    let result = dsqlex::eval_with_options(&ast, &ctx, &opts).unwrap();
    assert_eq!(result, Value::Decimal(dec("150.0")));
}

// ═══════════════════════════════════════════════════════════════
// BENCHMARK EXPRESSIONS (all 6 tiers must parse + eval)
// ═══════════════════════════════════════════════════════════════

fn bench_context() -> Context {
    let mut ctx = Context::new();
    ctx.set_string("field1", "revenue");
    ctx.set_decimal("price", "100.50");
    ctx.set_decimal("quantity", "10");
    ctx.set_decimal("discount_rate", "0.15");
    ctx.set_decimal("base_amount", "5000.00");
    ctx.set_decimal("rate", "1.25");
    ctx.set_decimal("amount", "750.00");
    ctx.set_string("status", "active");
    ctx.set_decimal("score", "850");
    ctx.set_string("region", "NA");
    ctx
}

#[test]
fn bench_tier1_trivial() {
    let ctx = bench_context();
    let result = dsqlex::eval_string("field1", &ctx).unwrap();
    assert_eq!(result, Value::String("revenue".into()));
}

#[test]
fn bench_tier2_arithmetic() {
    let ctx = bench_context();
    let _result = dsqlex::eval_string(
        "(price * quantity) + (price * quantity * discount_rate)",
        &ctx,
    )
    .unwrap();
}

#[test]
fn bench_tier3_functions() {
    let ctx = bench_context();
    let _result = dsqlex::eval_string(
        "ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1), 4)",
        &ctx,
    )
    .unwrap();
}

#[test]
fn bench_tier4_simple_case() {
    let ctx = bench_context();
    let _result = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN ROUND(amount * 1.1, 2) ELSE ROUND(amount * 0.9, 2) END",
        &ctx,
    )
    .unwrap();
}

#[test]
fn bench_tier5_multi_case() {
    let ctx = bench_context();
    let _result = dsqlex::eval_string(
        "CASE WHEN score > 900 THEN ROUND(amount * 0.02, 4) \
         WHEN score > 700 THEN ROUND(amount * 0.035, 4) \
         WHEN score > 500 THEN ROUND(amount * 0.05, 4) \
         WHEN score > 300 THEN ROUND(amount * 0.075, 4) \
         ELSE ROUND(amount * 0.10, 4) END",
        &ctx,
    )
    .unwrap();
}

#[test]
fn bench_tier6_monster() {
    let ctx = bench_context();
    let _result = dsqlex::eval_string(
        "CASE WHEN (region = 'NA' AND score > 800 AND status = 'active') \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.92, 4) \
         WHEN (region = 'EU' AND score > 800 AND status = 'active') \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.94, 4) \
         WHEN (region = 'NA' AND score > 600 AND status = 'active') \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.96, 4) \
         WHEN (region = 'EU' AND score > 600 AND status = 'active') \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.97, 4) \
         WHEN (region = 'NA' AND score > 400) \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.98, 4) \
         WHEN (region = 'EU' AND score > 400) \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * 0.99, 4) \
         WHEN status = 'suspended' THEN ROUND(COALESCE(base_amount, 0) * 0.50, 4) \
         WHEN status = 'pending' \
         THEN ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1) * COALESCE(discount_rate, 1), 4) \
         WHEN (score < 200 AND status != 'active') THEN 0 \
         ELSE ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1), 4) END",
        &ctx,
    )
    .unwrap();
}
