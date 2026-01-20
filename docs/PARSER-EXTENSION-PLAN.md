# Sigil Parser Extension Plan for Qliphoth

## Executive Summary

The Sigil parser (at `sigil/parser/`) needs extensions to fully support the qliphoth test suite. Analysis reveals that **lexer support is already complete** - all Unicode symbols tokenize correctly. The issues are in the **parser** and **runtime/interpreter**.

**Current State**: Lexer ✅ | Parser ⚠️ | Runtime ⚠️

---

## Analysis Results

### Lexer Status: ✅ Complete

All qliphoth Unicode symbols are already tokenized correctly:

| Symbol | Token | Status |
|--------|-------|--------|
| `≔` | `Let` | ✅ Works |
| `⊢` | `Impl` | ✅ Works |
| `⊤` | `Top` (yea) | ✅ Works |
| `⊥` | `Bottom` (nay) | ✅ Works |
| `⎇` | `If` | ✅ Works |
| `⎉` | `Else` | ✅ Works |
| `☉` | `Pub` | ✅ Works |
| `ᛈ` | `Enum` | ✅ Works |
| `⤺` | `Return` | ✅ Works |
| `→` | `Arrow` | ✅ Works |
| `·` | `MiddleDot` | ✅ Works |
| `⌥` | `Match` | ✅ Works |

**Verification**:
```bash
sigil lex /tmp/test.sg  # Shows all tokens correctly
```

### Parser Issues: ⚠️ Needs Work

| Issue | Severity | Description |
|-------|----------|-------------|
| Default field values | HIGH | `field: Type! = value` not evaluated at runtime |
| Static method calls | HIGH | `Type·method()` path resolution incomplete |
| Middledot method calls | MEDIUM | `obj·method()` doesn't pass `this` automatically |
| `This` type alias | MEDIUM | This-type in return position needs work |

### Runtime Issues: ⚠️ Needs Work

| Issue | Severity | Description |
|-------|----------|-------------|
| Default field initializers | HIGH | Sigil defaults not applied |
| Method receiver binding | HIGH | `·` separator doesn't bind receiver |
| Module resolution | MEDIUM | `tome·` and `above·` paths |

---

## Implementation Plan

### Phase 1: Parser Fixes (Priority: Critical)

#### 1.1 Fix Static Method Call Parsing

**File**: `parser/src/parser.rs`

**Problem**: `Type·method()` path resolution fails.

**Root Cause**: The `·` (middledot) isn't being parsed as a path separator in all contexts.

**Fix** (in parser's implementation language):
```
// In parse_postfix_expression or parse_primary:
// When we see Ident followed by MiddleDot, parse as a path expression.

rite parse_path_or_call(&vary this) → Result<Expr>! {
    vary path = vec![this·parse_ident()?];

    ⟳ this·check(Token::MiddleDot) {
        this·advance();
        path·push(this·parse_ident()?);
    }

    ⎇ this·check(Token::LParen) {
        // It's a rite/method call
        this·parse_call(Expr::Path(path))
    } ⎉ {
        Ok(Expr::Path(path))
    }
}
```

**Test Case**:
```sigil
sigil Foo {}
⊢ Foo {
    ☉ rite new() → This! { Foo {} }
}
rite main() {
    ≔ f = Foo·new();  // Should work
}
```

#### 1.2 Fix Middledot Method Receiver

**File**: `parser/src/parser.rs`

**Problem**: `obj·method()` doesn't pass `obj` as first argument.

**Root Cause**: `MiddleDot` is being treated only as a path separator, not as method call operator.

**Fix** (in parser's implementation language):
```
// In parse_postfix_expression:
rite parse_postfix(&vary this, vary expr: Expr) → Result<Expr>! {
    forever {
        ⌥ this·peek_token() {
            Some(Token::MiddleDot) => {
                this·advance();
                ≔ method = this·parse_ident()?;
                ⎇ this·check(Token::LParen) {
                    // Method call - expr becomes first argument (this)
                    ≔ args = this·parse_call_args()?;
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        method,
                        args,
                    };
                } ⎉ {
                    // Field access
                    expr = Expr::FieldAccess {
                        object: Box::new(expr),
                        field: method,
                    };
                }
            }
            // ... other postfix operators
            _ => ⊲,
        }
    }
    Ok(expr)
}
```

**Test Case**:
```sigil
sigil Counter { value: i64! }
⊢ Counter {
    rite inc(&vary this) { this·value += 1; }
}
rite main() {
    vary c = Counter { value: 0 };
    c·inc();      // Should increment c·value
    c·value       // Should be 1
}
```

### Phase 2: Runtime Fixes (Priority: High)

#### 2.1 Default Field Initializers

**File**: `parser/src/interpreter.rs`

**Problem**: `sigil Foo { bar: i64! = 42 }` doesn't use default when field omitted.

**Fix** (in parser's implementation language):
```
// In sigil instantiation evaluation:
rite eval_sigil_literal(&vary this, name: &str, fields: &[(String, Expr)]) → Result<Value>! {
    ≔ sigil_def = this·get_sigil_def(name)?;
    vary values = HashMap::new();

    // First, apply defaults from sigil definition
    each field_def of &sigil_def·fields {
        ⎇ let Some(default_expr) = &field_def·default {
            values·insert(field_def·name·clone(), this·eval(default_expr)?);
        }
    }

    // Then, override with provided values
    each (name, expr) of fields {
        values·insert(name·clone(), this·eval(expr)?);
    }

    // Check all required fields are present
    each field_def of &sigil_def·fields {
        ⎇ !values·contains_key(&field_def·name) {
            ⤺ Err(format!("missing field '{}'", field_def·name));
        }
    }

    Ok(Value::Sigil { name: name·to_string(), fields: values })
}
```

**Test Case**:
```sigil
sigil Config {
    debug: bool! = ⊥,
    timeout: i64! = 30
}
rite main() {
    ≔ c = Config {};      // Should use defaults
    c·debug               // Should be nay (⊥)
}
```

#### 2.2 Method Receiver Binding

**File**: `parser/src/interpreter.rs`

**Problem**: Method calls via `·` don't bind receiver to `this`.

**Fix** (in parser's implementation language):
```
// In method call evaluation:
rite eval_method_call(&vary this, receiver: Value, method: &str, args: Vec<Value>) → Result<Value>! {
    ≔ type_name = receiver·type_name();
    ≔ method_def = this·get_method(&type_name, method)?;

    // Create new scope with `this` bound to receiver
    this·push_scope();
    this·define("this", receiver·clone());

    // If method takes &this or &vary this, bind appropriately
    ⎇ method_def·takes_this_ref {
        // Bind as reference
    }

    // Bind other arguments
    each (param, arg) of method_def·params·iter()·zip(args) {
        this·define(&param·name, arg);
    }

    ≔ result = this·eval_block(&method_def·body)?;
    this·pop_scope();

    Ok(result)
}
```

### Phase 3: AST Extensions (Priority: Medium)

#### 3.1 Support `This` Type Alias

**File**: `parser/src/ast.rs`, `parser/src/parser.rs`

**Problem**: `This` as return type needs to resolve to enclosing type.

**Fix** (in parser's implementation language):
```
// In AST:
ᛈ Type {
    // ...existing variants...
    ThisType,  // `This` - resolved during type checking
}

// In parser, when parsing return type:
rite parse_type(&vary this) → Result<Type>! {
    ⌥ this·peek_token() {
        Some(Token::SelfUpper) => {
            this·advance();
            Ok(Type::ThisType)
        }
        // ...existing cases...
    }
}

// In type checker:
rite resolve_this_type(&this, ty: &Type, context: &ImplContext) → Type! {
    ⌥ ty {
        Type::ThisType => Type::Named(context·implementing_type·clone()),
        _ => ty·clone(),
    }
}
```

### Phase 4: Module System (Priority: Medium)

#### 4.1 Tome-Relative Paths

**Problem**: `tome·foo·bar` and `above·baz` don't resolve.

**Sigil Module Keywords**:
- `tome` - the current package/library root
- `above` - parent scroll (module)
- `scroll` - module declaration

**Fix** (in parser's implementation language):
```
sigil ModuleTree {
    root: Module!,
    current_path: Vec<String>!,
}

⊢ ModuleTree {
    rite resolve(&this, path: &[String]) → Option<&Item>? {
        ⌥ path·first()·map(|s| s·as_str()) {
            Some("tome") => this·resolve_from_root(&path[1..]),
            Some("above") => this·resolve_from_parent(&path[1..]),
            _ => this·resolve_relative(path),
        }
    }
}
```

---

## Testing Strategy

### Unit Tests for Parser

Add to `parser/src/parser.rs` tests:

```sigil
#[test]
rite test_static_method_call() {
    ≔ ast = parse("Foo·new()")·unwrap();
    // Assert it's a Call with Path ["Foo", "new"]
}

#[test]
rite test_middledot_method_call() {
    ≔ ast = parse("x·foo()")·unwrap();
    // Assert it's a MethodCall with receiver x, method foo
}

#[test]
rite test_default_field_value() {
    ≔ ast = parse("sigil Foo { bar: i64! = 42 }")·unwrap();
    // Assert field has default Some(Literal(42))
}
```

### Integration Tests

Create `jormungandr/tests/qliphoth/` directory with tests:

```
qliphoth/
├── P0_001_static_method.sg
├── P0_001_static_method.expected
├── P0_002_middledot_method.sg
├── P0_002_middledot_method.expected
├── P0_003_default_fields.sg
├── P0_003_default_fields.expected
└── ...
```

---

## Implementation Order

| Sprint | Focus | Deliverable |
|--------|-------|-------------|
| 1 | Static method calls | `Type·method()` works |
| 2 | Middledot methods | `obj·method()` passes this |
| 3 | Default field values | Sigil defaults applied |
| 4 | This type alias | `→ This!` resolves |
| 5 | Module paths | `tome·` and `above·` work |

---

## Files to Modify

| File | Changes |
|------|---------|
| `parser/src/lexer.rs` | None needed (complete) |
| `parser/src/parser.rs` | Path parsing, method calls |
| `parser/src/ast.rs` | ThisType variant, default field |
| `parser/src/interpreter.rs` | Method binding, defaults |
| `parser/src/typeck.rs` | This resolution |

---

## Success Criteria

1. All 513 qliphoth tests parse without error
2. MockPlatform can be instantiated and used
3. Performance tests can run and report timings
4. Error handling tests verify error propagation

---

## Appendix: Sigil Symbol Reference

### Complete Sigil Vocabulary

| Symbol/Keyword | Purpose | Token Name |
|----------------|---------|------------|
| `rite` | Function declaration | `Fn` |
| `sigil` | Data structure declaration | `Struct` |
| `aspect` | Interface/behavior declaration | `Trait` |
| `⊢` | Implementation block | `Impl` |
| `☉` | Public visibility | `Pub` |
| `≔` | Binding declaration | `Let` |
| `vary` | Mutable modifier | `Mut` |
| `⊤` / `yea` | Boolean true | `Top` / `True` |
| `⊥` / `nay` | Boolean false | `Bottom` / `False` |
| `⎇` | Conditional (if) | `If` |
| `⎉` | Alternative (else) | `Else` |
| `→` | Return type arrow | `Arrow` |
| `·` | Path/method separator | `MiddleDot` |
| `This` | Enclosing type reference | `SelfUpper` |
| `this` | Instance reference | `SelfLower` |
| `⤺` / `ret` | Return from rite | `Return` |
| `⌥` | Pattern match | `Match` |
| `tome` | Package/library root | `Crate` |
| `above` | Parent scroll | `Super` |
| `scroll` | Module declaration | `Mod` |
| `invoke` | Import declaration | `Use` |
| `ᛈ` | Enumeration declaration | `Enum` |
| `forever` | Infinite loop | `Loop` |
| `each` | Iteration | `For` |
| `of` | Membership/iteration | `In` |
| `⟳` | Conditional loop | `While` |
| `⊲` | Exit loop | `Break` |
| `⊳` | Skip iteration | `Continue` |
| `∋` | Where clause | `Where` |

### Sigil Design Principles

1. **Symbolic Precision**: Unicode symbols chosen for semantic clarity
2. **Middledot Universality**: `·` replaces both `.` and `::` for unified path/method syntax
3. **Evidentiality Markers**: `!` suffix indicates direct evidence/certainty
4. **Polysynthetic Morphemes**: Greek letters for data operations (τ, φ, σ, ρ, λ)

### Example: Complete Sigil Program

```sigil
// A counter with default value
☉ sigil Counter {
    ☉ value: i64! = 0
}

⊢ Counter {
    ☉ rite new() → This! {
        Counter {}  // Uses default
    }

    ☉ rite with_value(initial: i64) → This! {
        Counter { value: initial }
    }

    ☉ rite inc(&vary this) {
        this·value += 1;
    }

    ☉ rite get(&this) → i64! {
        this·value
    }
}

rite main() {
    vary counter = Counter·new();
    counter·inc();
    counter·inc();
    counter·get()  // Returns 2
}
```
