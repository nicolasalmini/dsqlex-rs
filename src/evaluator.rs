use crate::ast::{AstNode, BinOp};
use crate::{DsqlexError, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use regex::Regex;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Decimal(Decimal),
    String(Rc<str>),
    Bool(bool),
    Null,
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    NaiveDateTime(NaiveDateTime),
    Time(NaiveTime),
    List(Vec<Value>),
    Map(Box<Context>),
}

impl Value {
    #[inline]
    pub fn string(s: impl Into<Rc<str>>) -> Self {
        Value::String(s.into())
    }

    /// Extract as &str if this is a String variant.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Decimal(d) => write!(f, "{}", d),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            Value::Null => write!(f, "NULL"),
            Value::Date(d) => write!(f, "{}", d),
            Value::DateTime(d) => write!(f, "{}", d.to_rfc3339()),
            Value::NaiveDateTime(d) => write!(f, "{}", d),
            Value::Time(t) => write!(f, "{}", t),
            Value::List(items) => {
                let parts: Vec<std::string::String> =
                    items.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", parts.join(","))
            }
            Value::Map(_) => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub fields: HashMap<Rc<str>, Value>,
    pub nested: HashMap<Rc<str>, Context>,
    pub lists: HashMap<Rc<str>, Vec<Context>>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<Rc<str>>, val: Value) {
        self.fields.insert(key.into(), val);
    }

    pub fn set_decimal(&mut self, key: impl Into<Rc<str>>, val: &str) {
        self.fields.insert(
            key.into(),
            Value::Decimal(Decimal::from_str(val).unwrap()),
        );
    }

    pub fn set_string(&mut self, key: impl Into<Rc<str>>, val: impl Into<Rc<str>>) {
        self.fields.insert(key.into(), Value::String(val.into()));
    }

    pub fn set_bool(&mut self, key: impl Into<Rc<str>>, val: bool) {
        self.fields.insert(key.into(), Value::Bool(val));
    }

    pub fn set_null(&mut self, key: impl Into<Rc<str>>) {
        self.fields.insert(key.into(), Value::Null);
    }

    pub fn set_nested(&mut self, key: impl Into<Rc<str>>, ctx: Context) {
        self.nested.insert(key.into(), ctx);
    }

    pub fn set_list(&mut self, key: impl Into<Rc<str>>, items: Vec<Context>) {
        self.lists.insert(key.into(), items);
    }

    pub fn set_date(&mut self, key: impl Into<Rc<str>>, val: NaiveDate) {
        self.fields.insert(key.into(), Value::Date(val));
    }

    pub fn set_datetime(&mut self, key: impl Into<Rc<str>>, val: DateTime<Utc>) {
        self.fields.insert(key.into(), Value::DateTime(val));
    }

    pub fn set_naive_datetime(&mut self, key: impl Into<Rc<str>>, val: NaiveDateTime) {
        self.fields.insert(key.into(), Value::NaiveDateTime(val));
    }

    pub fn set_time(&mut self, key: impl Into<Rc<str>>, val: NaiveTime) {
        self.fields.insert(key.into(), Value::Time(val));
    }
}

pub type ResolverFn = Box<dyn Fn(&str, &HashSet<Rc<str>>) -> Result<Value>>;
pub type EventResolverFn =
    Box<dyn Fn(&str, &str, &Context, &HashSet<Rc<str>>) -> Result<Value>>;

pub struct EvalOptions {
    pub resolver: Option<ResolverFn>,
    pub event_resolver: Option<EventResolverFn>,
    pub visited: HashSet<Rc<str>>,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            resolver: None,
            event_resolver: None,
            visited: HashSet::new(),
        }
    }
}

// ── helpers ──

#[inline]
fn is_truthy(v: &Value) -> bool {
    !matches!(v, Value::Null | Value::Bool(false))
}

#[inline]
fn value_to_decimal(v: &Value) -> Result<Decimal> {
    match v {
        Value::Decimal(d) => Ok(*d),
        Value::String(s) => Decimal::from_str(s)
            .map_err(|_| DsqlexError(format!("Cannot convert '{}' to decimal", s))),
        Value::Bool(_) => Err(DsqlexError("Cannot convert boolean to decimal".into())),
        Value::Null => Err(DsqlexError("Cannot convert NULL to decimal".into())),
        Value::List(_) => Err(DsqlexError("Cannot convert list to decimal".into())),
        Value::Map(_) => Err(DsqlexError("Cannot convert map to decimal".into())),
        _ => Err(DsqlexError("Cannot convert value to decimal".into())),
    }
}

fn val_to_string(v: &Value) -> std::string::String {
    match v {
        Value::Decimal(d) => d.to_string(),
        Value::String(s) => s.to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Null => "NULL".to_string(),
        other => other.to_string(),
    }
}

/// Returns -1 (less), 0 (equal), 1 (greater), -2 (not comparable)
#[inline]
fn compare_values(lhs: &Value, rhs: &Value) -> i32 {
    match (lhs, rhs) {
        (Value::Null, Value::Null) => 0,
        (Value::Null, _) | (_, Value::Null) => -2,
        (Value::Decimal(a), Value::Decimal(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::String(a), Value::String(b)) => match a.as_ref().cmp(b.as_ref()) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::Date(a), Value::Date(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::DateTime(a), Value::DateTime(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::NaiveDateTime(a), Value::NaiveDateTime(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::Time(a), Value::Time(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        (Value::Bool(a), Value::Bool(b)) => match (*a as u8).cmp(&(*b as u8)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        // Mixed: string vs decimal or vice versa — try decimal conversion
        (Value::Decimal(_), Value::String(s)) => {
            if let Ok(d) = Decimal::from_str(s) {
                let a = if let Value::Decimal(a) = lhs { a } else { unreachable!() };
                return match a.cmp(&d) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
            }
            -2
        }
        (Value::String(s), Value::Decimal(_)) => {
            if let Ok(d) = Decimal::from_str(s) {
                let b = if let Value::Decimal(b) = rhs { b } else { unreachable!() };
                return match d.cmp(b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
            }
            -2
        }
        _ => {
            // Fallback: string representation comparison
            let sa = val_to_string(lhs);
            let sb = val_to_string(rhs);
            match sa.cmp(&sb) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }
        }
    }
}

fn like_match(text: &str, pattern: &str) -> bool {
    // Build regex from LIKE pattern
    let mut regex_str = std::string::String::with_capacity(pattern.len() * 2 + 8);
    regex_str.push_str("(?i)^");
    for ch in pattern.chars() {
        match ch {
            '%' => regex_str.push_str(".*"),
            '_' => regex_str.push('.'),
            c if ".+*?^${}()|[]\\".contains(c) => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            c => regex_str.push(c),
        }
    }
    regex_str.push('$');
    Regex::new(&regex_str)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

// ── resolver ──

fn resolve_identifier(name: &str, ctx: &Context, opts: &EvalOptions) -> Result<Value> {
    // Direct lookup — HashMap uses Borrow trait, so &str works on Rc<str> keys
    if let Some(v) = ctx.fields.get(name) {
        return Ok(v.clone()); // Rc<str> clone is just a refcount bump
    }

    // Dot-path resolution
    if name.contains('.') {
        let parts: Vec<&str> = name.split('.').collect();
        return resolve_dot_path(&parts, PathAcc::Context(ctx), name, opts);
    }

    if let Some(nested) = ctx.nested.get(name) {
        return Ok(Value::Map(Box::new(nested.clone())));
    }
    if let Some(list) = ctx.lists.get(name) {
        return Ok(Value::List(
            list.iter()
                .map(|item| Value::Map(Box::new(item.clone())))
                .collect(),
        ));
    }

    // Custom resolver
    if let Some(ref resolver) = opts.resolver {
        let key: Rc<str> = name.into();
        if opts.visited.contains(&key) {
            return Err(DsqlexError(format!(
                "Circular reference detected: {}",
                name
            )));
        }
        return resolver(name, &opts.visited);
    }

    Err(DsqlexError(format!("Unknown field: {}", name)))
}

enum PathAcc<'a> {
    Context(&'a Context),
    Value(&'a Value),
    List(&'a [Context]),
}

fn is_decimal_like(v: &Value) -> bool {
    match v {
        Value::Decimal(_) => true,
        Value::String(s) => Decimal::from_str(s).is_ok(),
        _ => false,
    }
}

fn resolve_dot_path(
    parts: &[&str],
    acc: PathAcc<'_>,
    path: &str,
    opts: &EvalOptions,
) -> Result<Value> {
    if parts.is_empty() {
        return match acc {
            PathAcc::Value(v) => Ok(v.clone()),
            PathAcc::Context(c) => Ok(Value::Map(Box::new(c.clone()))),
            PathAcc::List(items) => Ok(Value::List(
                items
                    .iter()
                    .map(|item| Value::Map(Box::new(item.clone())))
                    .collect(),
            )),
        };
    }

    if let PathAcc::List(items) = acc {
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(resolve_dot_path(parts, PathAcc::Context(item), path, opts)?);
        }
        if results.iter().all(is_decimal_like) {
            let mut sum = Decimal::ZERO;
            for r in &results {
                sum += value_to_decimal(r)?;
            }
            return Ok(Value::Decimal(sum));
        }
        return Ok(Value::List(results));
    }

    match acc {
        PathAcc::Context(c) => {
            let key = parts[0];
            if let Some(v) = c.fields.get(key) {
                return resolve_dot_path(&parts[1..], PathAcc::Value(v), path, opts);
            }
            if let Some(nested) = c.nested.get(key) {
                return resolve_dot_path(&parts[1..], PathAcc::Context(nested), path, opts);
            }
            if let Some(list) = c.lists.get(key) {
                return resolve_dot_path(&parts[1..], PathAcc::List(list), path, opts);
            }
            Err(DsqlexError(format!(
                "Unknown field: {} (failed at '{}')",
                path, key
            )))
        }
        _ => Err(DsqlexError(format!(
            "Cannot access '{}' on non-map value in path '{}'",
            parts[0], path
        ))),
    }
}

// ── main evaluate ──

pub fn evaluate(ast: &AstNode, ctx: &Context, opts: &EvalOptions) -> Result<Value> {
    match ast {
        AstNode::Select(inner) => evaluate(inner, ctx, opts),

        AstNode::NumberLit(d) => Ok(Value::Decimal(*d)),

        AstNode::StringLit(s) => Ok(Value::String(s.clone())), // Rc clone = refcount bump

        AstNode::BoolLit(b) => Ok(Value::Bool(*b)),
        AstNode::NullLit => Ok(Value::Null),
        AstNode::Identifier(name) => resolve_identifier(name, ctx, opts),

        AstNode::BinaryOp { op, left, right } => eval_binop(op, left, right, ctx, opts),

        AstNode::UnaryOp { operand } => {
            let val = evaluate(operand, ctx, opts)?;
            if matches!(&val, Value::Null) {
                return Ok(Value::Null);
            }
            let d = value_to_decimal(&val)?;
            Ok(Value::Decimal(-d))
        }

        AstNode::CaseExpr { whens, else_clause } => {
            for wc in whens {
                let cond = evaluate(&wc.condition, ctx, opts)?;
                if is_truthy(&cond) {
                    return evaluate(&wc.result, ctx, opts);
                }
            }
            if let Some(else_expr) = else_clause {
                evaluate(else_expr, ctx, opts)
            } else {
                Ok(Value::Null)
            }
        }

        AstNode::FunctionCall { name, args } => eval_function(name, args, ctx, opts),

        AstNode::InExpr { expr, items } => {
            let val = evaluate(expr, ctx, opts)?;
            for item in items {
                let item_val = evaluate(item, ctx, opts)?;
                if compare_values(&val, &item_val) == 0 {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }

        AstNode::NotInExpr { expr, items } => {
            let val = evaluate(expr, ctx, opts)?;
            for item in items {
                let item_val = evaluate(item, ctx, opts)?;
                if compare_values(&val, &item_val) == 0 {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }

        AstNode::LikeExpr { expr, pattern } => {
            let val = evaluate(expr, ctx, opts)?;
            let pat = evaluate(pattern, ctx, opts)?;
            if matches!(&val, Value::Null) || matches!(&pat, Value::Null) {
                return Ok(Value::Null);
            }
            let text = val_to_string(&val);
            let pat_str = val_to_string(&pat);
            Ok(Value::Bool(like_match(&text, &pat_str)))
        }

        AstNode::NotLikeExpr { expr, pattern } => {
            let val = evaluate(expr, ctx, opts)?;
            let pat = evaluate(pattern, ctx, opts)?;
            if matches!(&val, Value::Null) || matches!(&pat, Value::Null) {
                return Ok(Value::Null);
            }
            let text = val_to_string(&val);
            let pat_str = val_to_string(&pat);
            Ok(Value::Bool(!like_match(&text, &pat_str)))
        }
    }
}

#[inline]
fn eval_binop(
    op: &BinOp,
    left: &AstNode,
    right: &AstNode,
    ctx: &Context,
    opts: &EvalOptions,
) -> Result<Value> {
    // Short-circuit for AND/OR
    match op {
        BinOp::And => {
            let lv = evaluate(left, ctx, opts)?;
            if !is_truthy(&lv) {
                return Ok(lv);
            }
            return evaluate(right, ctx, opts);
        }
        BinOp::Or => {
            let lv = evaluate(left, ctx, opts)?;
            if is_truthy(&lv) {
                return Ok(lv);
            }
            return evaluate(right, ctx, opts);
        }
        _ => {}
    }

    let lv = evaluate(left, ctx, opts)?;
    let rv = evaluate(right, ctx, opts)?;

    match op {
        BinOp::Plus | BinOp::Minus | BinOp::Multiply | BinOp::Divide => {
            if matches!(&lv, Value::Null) || matches!(&rv, Value::Null) {
                return Ok(Value::Null);
            }
            let ld = value_to_decimal(&lv)?;
            let rd = value_to_decimal(&rv)?;
            let result = match op {
                BinOp::Plus => ld + rd,
                BinOp::Minus => ld - rd,
                BinOp::Multiply => ld * rd,
                BinOp::Divide => {
                    if rd.is_zero() {
                        return Err(DsqlexError("Division by zero".into()));
                    }
                    ld / rd
                }
                _ => unreachable!(),
            };
            Ok(Value::Decimal(result))
        }
        BinOp::Eq => {
            // Fast path: string equality without full compare_values overhead
            match (&lv, &rv) {
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(Rc::ptr_eq(a, b) || a == b)),
                (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Bool(a == b)),
                _ => Ok(Value::Bool(compare_values(&lv, &rv) == 0)),
            }
        }
        BinOp::Neq => {
            match (&lv, &rv) {
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(!Rc::ptr_eq(a, b) && a != b)),
                (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Bool(a != b)),
                _ => Ok(Value::Bool(compare_values(&lv, &rv) != 0)),
            }
        }
        BinOp::Lt => Ok(Value::Bool(compare_values(&lv, &rv) == -1)),
        BinOp::Gt => Ok(Value::Bool(compare_values(&lv, &rv) == 1)),
        BinOp::Lte => {
            let cmp = compare_values(&lv, &rv);
            Ok(Value::Bool(cmp == 0 || cmp == -1))
        }
        BinOp::Gte => {
            let cmp = compare_values(&lv, &rv);
            Ok(Value::Bool(cmp == 0 || cmp == 1))
        }
        BinOp::And | BinOp::Or => unreachable!(),
    }
}

fn eval_function(
    name: &str,
    args: &[AstNode],
    ctx: &Context,
    opts: &EvalOptions,
) -> Result<Value> {
    match name {
        "ROUND" => {
            if args.len() != 2 {
                return Err(DsqlexError("ROUND requires exactly 2 arguments".into()));
            }
            let val = evaluate(&args[0], ctx, opts)?;
            let prec_val = evaluate(&args[1], ctx, opts)?;
            if matches!(&val, Value::Null) || matches!(&prec_val, Value::Null) {
                return Ok(Value::Null);
            }
            let d = value_to_decimal(&val)?;
            let prec = value_to_decimal(&prec_val)?
                .to_i32()
                .ok_or_else(|| DsqlexError("ROUND precision must be an integer".into()))?;
            let rounded = d.round_dp_with_strategy(
                prec as u32,
                rust_decimal::RoundingStrategy::MidpointAwayFromZero,
            );
            Ok(Value::Decimal(rounded))
        }
        "COALESCE" => {
            for arg in args {
                let val = evaluate(arg, ctx, opts)?;
                if !matches!(&val, Value::Null) {
                    return Ok(val);
                }
            }
            Ok(Value::Null)
        }
        "UPPER" => {
            if args.len() != 1 {
                return Err(DsqlexError("UPPER requires exactly 1 argument".into()));
            }
            let val = evaluate(&args[0], ctx, opts)?;
            if matches!(&val, Value::Null) {
                return Ok(Value::Null);
            }
            match &val {
                Value::String(s) => Ok(Value::String(s.to_uppercase().into())),
                other => Ok(Value::String(val_to_string(other).into())),
            }
        }
        "LOWER" => {
            if args.len() != 1 {
                return Err(DsqlexError("LOWER requires exactly 1 argument".into()));
            }
            let val = evaluate(&args[0], ctx, opts)?;
            if matches!(&val, Value::Null) {
                return Ok(Value::Null);
            }
            match &val {
                Value::String(s) => Ok(Value::String(s.to_lowercase().into())),
                other => Ok(Value::String(val_to_string(other).into())),
            }
        }
        "ABS" => {
            if args.len() != 1 {
                return Err(DsqlexError("ABS requires exactly 1 argument".into()));
            }
            let val = evaluate(&args[0], ctx, opts)?;
            if matches!(&val, Value::Null) {
                return Ok(Value::Null);
            }
            let d = value_to_decimal(&val)?;
            Ok(Value::Decimal(d.abs()))
        }
        "CONCAT" => {
            let mut result = std::string::String::new();
            for arg in args {
                let val = evaluate(arg, ctx, opts)?;
                match &val {
                    Value::String(s) => result.push_str(s),
                    other => result.push_str(&val_to_string(other)),
                }
            }
            Ok(Value::String(result.into()))
        }
        "LEAST" | "GREATEST" => {
            if args.is_empty() {
                return Err(DsqlexError(
                    "LEAST/GREATEST requires at least one argument".into(),
                ));
            }
            let mut vals = Vec::with_capacity(args.len());
            let mut has_null = false;
            for arg in args {
                let v = evaluate(arg, ctx, opts)?;
                if matches!(&v, Value::Null) {
                    has_null = true;
                }
                vals.push(v);
            }
            if has_null {
                return Ok(Value::Null);
            }
            let target = if name == "LEAST" { -1 } else { 1 };
            let mut best = vals[0].clone();
            for v in &vals[1..] {
                if compare_values(v, &best) == target {
                    best = v.clone();
                }
            }
            Ok(best)
        }
        "EVENT" => {
            let valid = (args.len() == 2 || args.len() == 3)
                && args.iter().all(|a| matches!(a, AstNode::Identifier(_)));
            if !valid {
                return Err(DsqlexError(
                    "EVENT requires 2 or 3 arguments: EVENT(type, subtype) or EVENT(type, subtype, context_source)".into(),
                ));
            }
            let (type_str, subtype_str) = match (&args[0], &args[1]) {
                (AstNode::Identifier(t), AstNode::Identifier(s)) => (t.as_ref(), s.as_ref()),
                _ => unreachable!(),
            };
            if args.len() == 2 {
                return resolve_event(type_str, subtype_str, ctx, opts);
            }
            let source = match &args[2] {
                AstNode::Identifier(s) => s.as_ref(),
                _ => unreachable!(),
            };
            if let Some(list) = ctx.lists.get(source) {
                let mut sum = Decimal::ZERO;
                for item in list {
                    let v = resolve_event(type_str, subtype_str, item, opts)?;
                    sum += value_to_decimal(&v)?;
                }
                return Ok(Value::Decimal(sum));
            }
            if let Some(nested) = ctx.nested.get(source) {
                return resolve_event(type_str, subtype_str, nested, opts);
            }
            if ctx.fields.contains_key(source) {
                return Err(DsqlexError(format!(
                    "EVENT context source '{}' must be a map or list of maps",
                    source
                )));
            }
            Err(DsqlexError(format!(
                "EVENT context source '{}' not found in context",
                source
            )))
        }
        _ => Err(DsqlexError(format!("Unknown function: {}", name))),
    }
}

fn resolve_event(
    type_str: &str,
    subtype_str: &str,
    ctx: &Context,
    opts: &EvalOptions,
) -> Result<Value> {
    let event_resolver = opts
        .event_resolver
        .as_ref()
        .ok_or_else(|| DsqlexError("EVENT() calls require an :event_resolver option".into()))?;

    let key: Rc<str> = format!("{}.{}", type_str, subtype_str).into();
    if opts.visited.contains(&key) {
        return Err(DsqlexError(format!(
            "Circular reference detected: {}",
            key
        )));
    }
    let mut visited = opts.visited.clone();
    visited.insert(key);
    event_resolver(type_str, subtype_str, ctx, &visited)
}
