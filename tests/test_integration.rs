use dsqlex::evaluator::{Context, Value};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// ── Compound arithmetic ──

#[test]
fn multi_term_arithmetic() {
    let mut ctx = Context::new();
    ctx.set_decimal("price", "100.00");
    ctx.set_decimal("quantity", "5");
    ctx.set_decimal("discount_rate", "0.1");
    let r = dsqlex::eval_string(
        "(price * quantity) + (price * quantity * discount_rate)",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("550.000")));
}

// ── CASE with real-world pattern ──

#[test]
fn case_currency_dispatch() {
    let mut ctx = Context::new();
    ctx.set_string("currency", "BRL");
    ctx.set_decimal("amount_local", "500.00");
    ctx.set_decimal("amount_usd", "100.00");
    let r = dsqlex::eval_string(
        "CASE WHEN currency = 'USD' THEN amount_usd \
         WHEN currency = 'BRL' THEN amount_local \
         ELSE NULL END",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("500.00")));
}

// ── Nested function calls with NULL ──

#[test]
fn coalesce_with_null_field_in_round() {
    let mut ctx = Context::new();
    ctx.set_null("base_amount");
    ctx.set_decimal("rate", "1.5");
    let r = dsqlex::eval_string(
        "ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1), 4)",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("0.0000")));
}

#[test]
fn coalesce_both_present() {
    let mut ctx = Context::new();
    ctx.set_decimal("base_amount", "5000.00");
    ctx.set_decimal("rate", "1.25");
    let r = dsqlex::eval_string(
        "ROUND(COALESCE(base_amount, 0) * COALESCE(rate, 1), 4)",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("6250.0000")));
}

// ── CASE with ROUND ──

#[test]
fn case_with_round_active_branch() {
    let mut ctx = Context::new();
    ctx.set_string("status", "active");
    ctx.set_decimal("amount", "750.00");
    let r = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN ROUND(amount * 1.1, 2) \
         ELSE ROUND(amount * 0.9, 2) END",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("825.00")));
}

#[test]
fn case_with_round_else_branch() {
    let mut ctx = Context::new();
    ctx.set_string("status", "inactive");
    ctx.set_decimal("amount", "750.00");
    let r = dsqlex::eval_string(
        "CASE WHEN status = 'active' THEN ROUND(amount * 1.1, 2) \
         ELSE ROUND(amount * 0.9, 2) END",
        &ctx,
    ).unwrap();
    assert_eq!(r, Value::Decimal(dec("675.00")));
}

// ── Multi-branch CASE with score tiers ──

#[test]
fn multi_branch_case_score_tiers() {
    let mut ctx = Context::new();
    ctx.set_decimal("amount", "750.00");
    ctx.set_decimal("score", "850");
    let r = dsqlex::eval_string(
        "CASE WHEN score > 900 THEN ROUND(amount * 0.02, 4) \
         WHEN score > 700 THEN ROUND(amount * 0.035, 4) \
         WHEN score > 500 THEN ROUND(amount * 0.05, 4) \
         WHEN score > 300 THEN ROUND(amount * 0.075, 4) \
         ELSE ROUND(amount * 0.10, 4) END",
        &ctx,
    ).unwrap();
    // score=850 matches > 700 branch
    assert_eq!(r, Value::Decimal(dec("26.2500")));
}

// ── Complex nested CASE (monster expression) ──

#[test]
fn complex_nested_case_with_coalesce() {
    let mut ctx = Context::new();
    ctx.set_decimal("base_amount", "5000.00");
    ctx.set_decimal("rate", "1.25");
    ctx.set_decimal("discount_rate", "0.15");
    ctx.set_decimal("amount", "750.00");
    ctx.set_string("status", "active");
    ctx.set_decimal("score", "850");
    ctx.set_string("region", "NA");

    let r = dsqlex::eval_string(
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
    ).unwrap();
    // region=NA, score=850>800, status=active → first branch
    // ROUND(5000 * 1.25 * 0.92, 4) = ROUND(5750, 4) = 5750.0000
    assert_eq!(r, Value::Decimal(dec("5750.0000")));
}

// ── Parse-once, eval-many pattern ──

#[test]
fn parse_once_eval_many() {
    let ast = dsqlex::parse("amount * rate").unwrap();

    let mut ctx1 = Context::new();
    ctx1.set_decimal("amount", "100");
    ctx1.set_decimal("rate", "1.5");

    let mut ctx2 = Context::new();
    ctx2.set_decimal("amount", "200");
    ctx2.set_decimal("rate", "2.0");

    assert_eq!(dsqlex::eval(&ast, &ctx1).unwrap(), Value::Decimal(dec("150.0")));
    assert_eq!(dsqlex::eval(&ast, &ctx2).unwrap(), Value::Decimal(dec("400.0")));
}

// ── Edge cases ──

#[test]
fn coalesce_all_null() {
    assert_eq!(
        dsqlex::eval_string("COALESCE(NULL, NULL, NULL)", &Context::new()).unwrap(),
        Value::Null
    );
}

#[test]
fn case_no_else_no_match() {
    let mut ctx = Context::new();
    ctx.set_decimal("x", "0");
    let r = dsqlex::eval_string("CASE WHEN x > 100 THEN 'big' END", &ctx).unwrap();
    assert_eq!(r, Value::Null);
}

#[test]
fn nested_parenthesized_arithmetic() {
    let mut ctx = Context::new();
    ctx.set_decimal("a", "2");
    ctx.set_decimal("b", "3");
    ctx.set_decimal("c", "4");
    let r = dsqlex::eval_string("(a + b) * (c + a)", &ctx).unwrap();
    // (2+3) * (4+2) = 5 * 6 = 30
    assert_eq!(r, Value::Decimal(dec("30")));
}

#[test]
fn short_circuit_and() {
    // FALSE AND <error> should not evaluate the right side
    let mut ctx = Context::new();
    ctx.set_bool("flag", false);
    let r = dsqlex::eval_string("flag AND (1 / 0 > 0)", &ctx);
    // short-circuit should prevent division by zero
    assert_eq!(r.unwrap(), Value::Bool(false));
}

#[test]
fn short_circuit_or() {
    let mut ctx = Context::new();
    ctx.set_bool("flag", true);
    let r = dsqlex::eval_string("flag OR (1 / 0 > 0)", &ctx);
    assert_eq!(r.unwrap(), Value::Bool(true));
}
