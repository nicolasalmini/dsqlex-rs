use dsqlex::evaluator::{Context, EvalOptions, Value};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::rc::Rc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// ── Literals ──

#[test]
fn number_literal() {
    let r = dsqlex::eval_string("42", &Context::new()).unwrap();
    assert_eq!(r, Value::Decimal(dec("42")));
}

#[test]
fn string_literal() {
    let r = dsqlex::eval_string("'hello'", &Context::new()).unwrap();
    assert_eq!(r, Value::String("hello".into()));
}

#[test]
fn bool_literals() {
    assert_eq!(dsqlex::eval_string("TRUE", &Context::new()).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("FALSE", &Context::new()).unwrap(), Value::Bool(false));
}

#[test]
fn null_literal() {
    assert_eq!(dsqlex::eval_string("NULL", &Context::new()).unwrap(), Value::Null);
}

// ── Identifiers ──

#[test]
fn field_lookup() {
    let mut ctx = Context::new();
    ctx.set_decimal("amount", "500.00");
    assert_eq!(dsqlex::eval_string("amount", &ctx).unwrap(), Value::Decimal(dec("500.00")));
}

#[test]
fn unknown_field_is_error() {
    assert!(dsqlex::eval_string("nonexistent", &Context::new()).is_err());
}

#[test]
fn dot_path_single_level() {
    let mut ctx = Context::new();
    let mut nested = Context::new();
    nested.set_decimal("rate", "5.00");
    ctx.set_nested("config", nested);
    assert_eq!(dsqlex::eval_string("config.rate", &ctx).unwrap(), Value::Decimal(dec("5.00")));
}

#[test]
fn dot_path_multi_level() {
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
fn resolver_callback() {
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
    assert_eq!(dsqlex::eval_with_options(&ast, &ctx, &opts).unwrap(), Value::Decimal(dec("150.0")));
}

// ── Arithmetic ──

#[test]
fn addition_subtraction_multiplication() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "10");
    ctx.set_decimal("b", "3");
    assert_eq!(dsqlex::eval_string("a + b", &ctx).unwrap(), Value::Decimal(dec("13")));
    assert_eq!(dsqlex::eval_string("a - b", &ctx).unwrap(), Value::Decimal(dec("7")));
    assert_eq!(dsqlex::eval_string("a * b", &ctx).unwrap(), Value::Decimal(dec("30")));
}

#[test]
fn division_precision() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "10");
    ctx.set_decimal("b", "3");
    if let Value::Decimal(d) = dsqlex::eval_string("a / b", &ctx).unwrap() {
        assert!(d > dec("3.33") && d < dec("3.34"));
    } else {
        panic!("Expected Decimal");
    }
}

// ── Comparison ──

#[test]
fn decimal_comparisons() {
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
fn string_equality() {
    let mut ctx = Context::new();
    ctx.set_string("s", "hello");
    assert_eq!(dsqlex::eval_string("s = 'hello'", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("s != 'world'", &ctx).unwrap(), Value::Bool(true));
}

#[test]
fn null_is_null() {
    let mut ctx = Context::new();
    ctx.set_null("x");
    assert_eq!(dsqlex::eval_string("x IS NULL", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("x IS NOT NULL", &ctx).unwrap(), Value::Bool(false));
}

// ── Logical ──

#[test]
fn and_operator() {
    let ctx = Context::new();
    assert_eq!(dsqlex::eval_string("TRUE AND TRUE", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("TRUE AND FALSE", &ctx).unwrap(), Value::Bool(false));
    assert_eq!(dsqlex::eval_string("FALSE AND TRUE", &ctx).unwrap(), Value::Bool(false));
}

#[test]
fn or_operator() {
    let ctx = Context::new();
    assert_eq!(dsqlex::eval_string("TRUE OR FALSE", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("FALSE OR FALSE", &ctx).unwrap(), Value::Bool(false));
}

// ── CASE/WHEN ──

#[test]
fn case_matching_branch() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    ctx.set_decimal("amount", "100");
    let r = dsqlex::eval_string("CASE WHEN status = 'active' THEN amount ELSE 0 END", &ctx).unwrap();
    assert_eq!(r, Value::Decimal(dec("100")));
}

#[test]
fn case_no_match_returns_null() {
    let mut ctx = Context::new();
    ctx.set_string("status", "unknown");
    let r = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN 1 WHEN status = 'pending' THEN 2 END",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Null);
}

// ── Functions ──

#[test]
fn round_function() {
    let ctx = Context::new();
    assert_eq!(dsqlex::eval_string("ROUND(3.14159, 2)", &ctx).unwrap(), Value::Decimal(dec("3.14")));
    assert_eq!(dsqlex::eval_string("ROUND(2.555, 2)", &ctx).unwrap(), Value::Decimal(dec("2.56")));
}

#[test]
fn round_null_returns_null() {
    assert_eq!(dsqlex::eval_string("ROUND(NULL, 2)", &Context::new()).unwrap(), Value::Null);
}

#[test]
fn coalesce_function() {
    let ctx = Context::new();
    assert_eq!(dsqlex::eval_string("COALESCE(NULL, NULL, 42)", &ctx).unwrap(), Value::Decimal(dec("42")));
    assert_eq!(dsqlex::eval_string("COALESCE(NULL, 'hello')", &ctx).unwrap(), Value::String("hello".into()));
    assert_eq!(dsqlex::eval_string("COALESCE(NULL, NULL)", &ctx).unwrap(), Value::Null);
}

#[test]
fn upper_lower_functions() {
    let ctx = Context::new();
    assert_eq!(dsqlex::eval_string("UPPER('hello')", &ctx).unwrap(), Value::String("HELLO".into()));
    assert_eq!(dsqlex::eval_string("LOWER('HELLO')", &ctx).unwrap(), Value::String("hello".into()));
}

#[test]
fn upper_null_returns_null() {
    assert_eq!(dsqlex::eval_string("UPPER(NULL)", &Context::new()).unwrap(), Value::Null);
}

#[test]
fn abs_function() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "-42.5");
    assert_eq!(dsqlex::eval_string("ABS(x)", &ctx).unwrap(), Value::Decimal(dec("42.5")));
}

#[test]
fn concat_function() {
    let mut ctx = Context::new();
    ctx.set_string("first", "Hello");
    ctx.set_string("last", "World");
    assert_eq!(
        dsqlex::eval_string("CONCAT(first, ' ', last)", &ctx).unwrap(),
        Value::String("Hello World".into())
    );
}

// ── IN / NOT IN ──

#[test]
fn in_operator() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    assert_eq!(dsqlex::eval_string("status IN ('active', 'pending')", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("status IN ('deleted', 'archived')", &ctx).unwrap(), Value::Bool(false));
}

#[test]
fn not_in_operator() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    assert_eq!(dsqlex::eval_string("status NOT IN ('deleted')", &ctx).unwrap(), Value::Bool(true));
}

// ── LIKE / NOT LIKE ──

#[test]
fn like_operator() {
    let mut ctx = Context::new();
    ctx.set_string("name", "Hello World");
    assert_eq!(dsqlex::eval_string("name LIKE '%world%'", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("name LIKE 'hello%'", &ctx).unwrap(), Value::Bool(true));
    assert_eq!(dsqlex::eval_string("name LIKE '%xyz%'", &ctx).unwrap(), Value::Bool(false));
}

#[test]
fn not_like_operator() {
    let mut ctx = Context::new();
    ctx.set_string("name", "Hello");
    assert_eq!(dsqlex::eval_string("name NOT LIKE '%xyz%'", &ctx).unwrap(), Value::Bool(true));
}

#[test]
fn like_null_returns_null() {
    let mut ctx = Context::new();
    ctx.set_null("name");
    assert_eq!(dsqlex::eval_string("name LIKE '%test%'", &ctx).unwrap(), Value::Null);
}
