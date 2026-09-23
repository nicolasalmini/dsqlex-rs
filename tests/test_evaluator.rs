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

#[test]
fn unary_minus() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "100.00");
    ctx.set_null("nullable_field");

    assert_eq!(
        dsqlex::eval_string("SELECT -5", &ctx).unwrap(),
        Value::Decimal(dec("-5"))
    );
    assert_eq!(
        dsqlex::eval_string("SELECT -x", &ctx).unwrap(),
        Value::Decimal(dec("-100.00"))
    );
    assert_eq!(
        dsqlex::eval_string("SELECT -(1 + 2)", &ctx).unwrap(),
        Value::Decimal(dec("-3"))
    );
    assert_eq!(
        dsqlex::eval_string("SELECT - -5", &ctx).unwrap(),
        Value::Decimal(dec("5"))
    );
    assert_eq!(
        dsqlex::eval_string("SELECT -nullable_field", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(
        dsqlex::eval_string("SELECT -NULL", &ctx).unwrap(),
        Value::Null
    );
    assert!(dsqlex::eval_string("SELECT -'abc'", &ctx).is_err());
}

#[test]
fn arithmetic_null_propagation() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "10");
    ctx.set_null("n");
    for expr in ["n + 1", "1 + n", "n - 1", "n * 2", "n / 2", "x * n"] {
        assert_eq!(
            dsqlex::eval_string(expr, &ctx).unwrap(),
            Value::Null,
            "{}",
            expr
        );
    }
}

#[test]
fn round_abs_null() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "3.14159");
    ctx.set_null("n");
    assert_eq!(
        dsqlex::eval_string("ROUND(n, 2)", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(
        dsqlex::eval_string("ROUND(x, n)", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(dsqlex::eval_string("ABS(n)", &ctx).unwrap(), Value::Null);
}

#[test]
fn least_greatest() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "100.00");
    ctx.set_decimal("y", "20.00");
    ctx.set_null("n");

    assert_eq!(
        dsqlex::eval_string("LEAST(3, 1, 2)", &ctx).unwrap(),
        Value::Decimal(dec("1"))
    );
    assert_eq!(
        dsqlex::eval_string("GREATEST(3, 1, 2)", &ctx).unwrap(),
        Value::Decimal(dec("3"))
    );
    assert_eq!(
        dsqlex::eval_string("LEAST(x, y)", &ctx).unwrap(),
        Value::Decimal(dec("20.00"))
    );
    assert_eq!(
        dsqlex::eval_string("LEAST(7)", &ctx).unwrap(),
        Value::Decimal(dec("7"))
    );
    assert_eq!(
        dsqlex::eval_string("LEAST(x, n)", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(
        dsqlex::eval_string("GREATEST(1, n)", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(
        dsqlex::eval_string("LEAST('banana', 'apple', 'cherry')", &ctx).unwrap(),
        Value::String("apple".into())
    );
    assert_eq!(
        dsqlex::eval_string("GREATEST('banana', 'apple', 'cherry')", &ctx).unwrap(),
        Value::String("cherry".into())
    );
    let r = dsqlex::eval_string("LEAST(1, 1.0)", &ctx).unwrap();
    assert_eq!(r, Value::Decimal(dec("1")));
    if let Value::Decimal(d) = r {
        assert_eq!(d.to_string(), "1");
    }
    assert!(dsqlex::eval_string("LEAST()", &ctx).is_err());
    assert!(dsqlex::eval_string("GREATEST()", &ctx).is_err());
}

#[test]
fn least_greatest_numeric_strings() {
    let ctx = Context::new();
    assert_eq!(
        dsqlex::eval_string("LEAST('2', '10')", &ctx).unwrap(),
        Value::String("10".into())
    );
    assert_eq!(
        dsqlex::eval_string("GREATEST('2', '10')", &ctx).unwrap(),
        Value::String("2".into())
    );
}

#[test]
fn least_greatest_dates() {
    use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
    let mut ctx = Context::new();
    ctx.set_date("d1", NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    ctx.set_date("d2", NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());
    ctx.set_datetime(
        "dt1",
        Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap(),
    );
    ctx.set_datetime(
        "dt2",
        Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap(),
    );
    ctx.set_time("t1", NaiveTime::from_hms_opt(9, 30, 0).unwrap());
    ctx.set_time("t2", NaiveTime::from_hms_opt(18, 45, 0).unwrap());

    assert_eq!(
        dsqlex::eval_string("LEAST(d1, d2)", &ctx).unwrap(),
        Value::Date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
    );
    assert_eq!(
        dsqlex::eval_string("GREATEST(dt1, dt2)", &ctx).unwrap(),
        Value::DateTime(Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap())
    );
    assert_eq!(
        dsqlex::eval_string("LEAST(t1, t2)", &ctx).unwrap(),
        Value::Time(NaiveTime::from_hms_opt(9, 30, 0).unwrap())
    );
}

#[test]
fn unknown_dotted_field_errors() {
    let ctx = Context::new();
    assert!(dsqlex::eval_string("missing", &ctx).is_err());
    assert!(dsqlex::eval_string("missing.field", &ctx).is_err());
}

#[test]
fn resolver_fallback_and_precedence() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "100.00");
    let opts = EvalOptions {
        resolver: Some(Box::new(|name: &str, _visited: &HashSet<Rc<str>>| {
            if name == "external" {
                Ok(Value::Decimal(dec("1.5")))
            } else {
                Err(dsqlex::DsqlexError(format!("Unknown field: {}", name)))
            }
        })),
        ..Default::default()
    };
    assert_eq!(
        dsqlex::eval_string_with_options("external", &ctx, &opts).unwrap(),
        Value::Decimal(dec("1.5"))
    );
    assert!(dsqlex::eval_string_with_options("nope", &ctx, &opts).is_err());

    let opts2 = EvalOptions {
        resolver: Some(Box::new(|_: &str, _: &HashSet<Rc<str>>| {
            Ok(Value::Decimal(dec("999")))
        })),
        ..Default::default()
    };
    assert_eq!(
        dsqlex::eval_string_with_options("x", &ctx, &opts2).unwrap(),
        Value::Decimal(dec("100.00"))
    );
}

#[test]
fn resolver_circular() {
    let ctx = Context::new();
    let opts = EvalOptions {
        resolver: Some(Box::new(|name: &str, visited: &HashSet<Rc<str>>| {
            let mut inner_visited = visited.clone();
            inner_visited.insert(name.into());
            let inner = EvalOptions {
                resolver: Some(Box::new(|_: &str, _: &HashSet<Rc<str>>| Ok(Value::Null))),
                visited: inner_visited,
                ..Default::default()
            };
            dsqlex::eval_string_with_options(name, &Context::new(), &inner)
        })),
        ..Default::default()
    };
    assert!(dsqlex::eval_string_with_options("loop", &ctx, &opts).is_err());
}

#[test]
fn dot_path_list_numeric() {
    let mut ctx = Context::new();
    let mut d1 = Context::new();
    d1.set_decimal("amt", "3");
    let mut d2 = Context::new();
    d2.set_decimal("amt", "4");
    ctx.set_list("items", vec![d1, d2]);
    assert_eq!(
        dsqlex::eval_string("items.amt", &ctx).unwrap(),
        Value::Decimal(dec("7"))
    );
}

#[test]
fn dot_path_list_nonnumeric() {
    let mut ctx = Context::new();
    let mut s1 = Context::new();
    s1.set_string("name", "x");
    let mut s2 = Context::new();
    s2.set_string("name", "y");
    ctx.set_list("items", vec![s1, s2]);
    match dsqlex::eval_string("items.name", &ctx).unwrap() {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::String("x".into()));
            assert_eq!(items[1], Value::String("y".into()));
        }
        other => panic!("expected list value, got {:?}", other),
    }
}

#[test]
fn round_null_with_missing_precision_errors() {
    let mut ctx = Context::new();
    ctx.set_null("n");
    assert_eq!(
        dsqlex::eval_string("ROUND(n, 2)", &ctx).unwrap(),
        Value::Null
    );
    assert_eq!(
        dsqlex::eval_string("ROUND(1.5, n)", &ctx).unwrap(),
        Value::Null
    );
    assert!(dsqlex::eval_string("ROUND(n, missing_prec)", &ctx).is_err());
}

#[test]
fn nested_map_returns_map_value() {
    let mut ctx = Context::new();
    let mut order = Context::new();
    order.set_decimal("base", "7");
    ctx.set_nested("order", order.clone());
    match dsqlex::eval_string("order", &ctx).unwrap() {
        Value::Map(m) => {
            assert_eq!(m.fields["base"], Value::Decimal(dec("7")));
        }
        other => panic!("expected map value, got {:?}", other),
    }

    let mut outer = Context::new();
    outer.set_nested("order", order);
    let mut wrap = Context::new();
    wrap.set_nested("o", outer);
    match dsqlex::eval_string("o.order", &wrap).unwrap() {
        Value::Map(m) => {
            assert_eq!(m.fields["base"], Value::Decimal(dec("7")));
        }
        other => panic!("expected map value from dot path, got {:?}", other),
    }
}
