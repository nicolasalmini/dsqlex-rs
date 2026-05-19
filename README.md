# dsqlex-rs

A Rust implementation of the DSQLEX expression evaluator. Zero-copy string handling with `Rc<str>`, pre-parsed decimal literals, and `rust_decimal` for 128-bit fixed-point arithmetic.

## Usage

```rust
use dsqlex::evaluator::{Context, Value};

// One-shot: parse + evaluate
let mut ctx = Context::new();
ctx.set_decimal("price", "100.00");
ctx.set_decimal("quantity", "5");
ctx.set_decimal("tax", "50.00");

let result = dsqlex::eval_string("(price * quantity) + tax", &ctx).unwrap();
// result = Value::Decimal(550.00)

// Parse once, evaluate many
let ast = dsqlex::parse("amount * rate").unwrap();
for record in &records {
    let result = dsqlex::eval(&ast, record).unwrap();
}
```

## Building

```bash
cargo build --release
cargo test
```

## Dependencies

- [`rust_decimal`](https://crates.io/crates/rust_decimal) — 128-bit fixed-point decimal arithmetic
- [`regex`](https://crates.io/crates/regex) — LIKE pattern matching

## Supported Features

| Feature | Syntax |
|---------|--------|
| Arithmetic | `+`, `-`, `*`, `/` (decimal precision) |
| Comparison | `=`, `!=`, `<`, `>`, `<=`, `>=` |
| Logical | `AND`, `OR` (same-op chaining; mixing requires parens) |
| Conditionals | `CASE WHEN ... THEN ... ELSE ... END` |
| Functions | `ROUND()`, `COALESCE()`/`NVL()`, `UPPER()`, `LOWER()`, `ABS()`, `CONCAT()`, `EVENT()` |
| Membership | `IN (...)`, `NOT IN (...)` |
| Pattern | `LIKE`, `NOT LIKE` (case-insensitive) |
| Null check | `IS NULL`, `IS NOT NULL`, `IS TRUE`, `IS FALSE` |
| Literals | Numbers, strings (`'...'`), `TRUE`, `FALSE`, `NULL` |
| Dot-paths | `config.pricing.margin` (nested context access) |
| Comments | `--`, `#`, `/* ... */` |

## API

```rust
// Parse an expression string into an AST.
pub fn parse(expression: &str) -> Result<AstNode>;

// Evaluate a pre-parsed AST with the given context.
pub fn eval(ast: &AstNode, ctx: &Context) -> Result<Value>;

// Evaluate with options (resolver, event_resolver).
pub fn eval_with_options(ast: &AstNode, ctx: &Context, opts: &EvalOptions) -> Result<Value>;

// Parse and evaluate in one call.
pub fn eval_string(expression: &str, ctx: &Context) -> Result<Value>;
```

### Context

```rust
let mut ctx = Context::new();
ctx.set_decimal("amount", "100.50");
ctx.set_string("status", "active");
ctx.set_bool("enabled", true);
ctx.set_null("discount");

// Nested contexts for dot-path resolution
let mut nested = Context::new();
nested.set_decimal("margin", "0.15");
ctx.set_nested("config", nested);
```

### Value

```rust
pub enum Value {
    Decimal(Decimal),   // rust_decimal::Decimal
    String(Rc<str>),    // Zero-copy clone (refcount bump)
    Bool(bool),
    Null,
}
```

## Design Decisions

- **`Rc<str>` for strings**: Cloning a `Value::String` is a refcount bump, not a heap allocation. This makes string comparisons and field lookups very fast.
- **Pre-parsed decimals in AST**: `NumberLit` stores a `Decimal`, not a `String`. No conversion at eval time.
- **Explicit parentheses**: `a + b * c` is rejected. Use `(a + b) * c`.
- **Parse once, eval many**: The AST is immutable and cheaply cloneable.

## Related

- [dsqlex-c](https://github.com/nicolasalmini/dsqlex-c) — C/C++ implementation (mpdecimal)
- [dsqlex-go](https://github.com/nicolasalmini/dsqlex-go) — Go implementation (govalues/decimal)
- [dsqlex-py](https://github.com/nicolasalmini/dsqlex-py) — Python implementation
- [dsqlex-ts](https://github.com/nicolasalmini/dsqlex-ts) — TypeScript implementation
- [dsqlex-bench](https://github.com/nicolasalmini/dsqlex-bench) — Cross-language benchmark suite
