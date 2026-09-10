# DSL View Syntax Specification

**Version:** 0.1.0
**Status:** SUPERSEDED
**Date:** 2026-02-14
**Parent Spec:** UI-COMPONENTS.md
**Superseded By:** AGENT-CENTRIC-QLIPHOTH.md

> **NOTE:** This spec has been superseded. After critical review, DSL syntax was
> determined to optimize for human typing ergonomics rather than agent code
> generation. The new agent-centric design uses explicit VNode builder methods
> instead. See AGENT-CENTRIC-QLIPHOTH.md for the replacement architecture.

---

## Abstract

This specification defines a domain-specific language (DSL) for declarative view
templates in Qliphoth. The syntax enables concise, composable UI definitions that
compile to efficient WASM code. The design draws from JSX, Elm, and SwiftUI while
maintaining Sigil's aesthetic.

---

## 1. Conceptual Foundation

### 1.1 The Problem

Qliphoth applications need a way to express UI structure declaratively:

```sigil
// Current: Verbose builder pattern
≔ view = VNode·element("div")
    ·attr("class", "container")
    ·child(VNode·text("Hello"))
    ·child(VNode·element("button")
        ·attr("onclick", handler)
        ·child(VNode·text("Click me")));

// Desired: Concise DSL
≔ view = div class="container" {
    "Hello"
    button onclick={handler} { "Click me" }
};
```

### 1.2 Design Goals

- **Concise**: Minimal syntax overhead for common patterns
- **Composable**: Elements nest naturally, components are first-class
- **Type-safe**: Props validated at compile time where possible
- **Efficient**: Compiles to direct VNode construction, no runtime parsing
- **Familiar**: Recognizable to developers from JSX/HTML backgrounds

### 1.3 Non-Goals

- Full HTML compatibility (we subset to common elements)
- Runtime template parsing (all templates compile statically)
- String-based templating (no `{{ interpolation }}` syntax)

---

## 2. Syntax Overview

### 2.1 Element Syntax

```
element_expr :=
    | element_name attribute* "{" children "}"
    | element_name attribute* "{" "}"
    | element_name attribute*

element_name := lowercase_ident
component_name := uppercase_ident | path

attribute :=
    | ident "=" expr
    | ident "=" "{" expr "}"
    | ident                        // boolean shorthand
    | "..." expr                   // spread

children :=
    | child*

child :=
    | string_literal               // text node
    | "{" expr "}"                 // expression
    | element_expr                 // nested element
    | component_expr               // component invocation
```

### 2.2 Examples

```sigil
// Simple element
div { "Hello, World!" }

// With attributes
div class="container" id="main" {
    "Content"
}

// Nested elements
nav {
    ul {
        li { a href="/" { "Home" } }
        li { a href="/about" { "About" } }
    }
}

// Dynamic content
div {
    "Welcome, "
    { user.name }
    "!"
}

// Conditional rendering
div {
    ⎇ is_logged_in {
        UserProfile { user }
    } ⎉ {
        LoginButton {}
    }
}

// List rendering
ul {
    ∀ item ∈ items {
        li key={item.id} { { item.name } }
    }
}
```

---

## 3. Element Detection

### 3.1 The Disambiguation Problem

The parser must distinguish:

```sigil
// Struct literal
Point { x: 10, y: 20 }

// View element
div { "content" }

// Component (uppercase)
MyButton { onclick: handler }
```

### 3.2 Detection Rules

```
element_detection(name, first_token_after_brace):
    if name starts with uppercase:
        → component (may have named fields)

    if name ∈ KNOWN_HTML_ELEMENTS:
        if first_token_after_brace is string_literal:
            → element with text child
        if first_token_after_brace is "{":
            → element with expression child
        if first_token_after_brace is lowercase_ident AND NOT followed by ":":
            → element with nested element
        if first_token_after_brace is uppercase_ident:
            → element with nested component
        else:
            → struct literal (field: value pattern)

    else:
        → struct literal
```

### 3.3 Known HTML Elements

```
KNOWN_HTML_ELEMENTS = {
    // Document structure
    "html", "head", "body", "main", "header", "footer", "nav", "aside",
    "section", "article",

    // Block elements
    "div", "p", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "blockquote",

    // Inline elements
    "span", "a", "strong", "em", "code", "br", "hr",

    // Lists
    "ul", "ol", "li", "dl", "dt", "dd",

    // Tables
    "table", "thead", "tbody", "tfoot", "tr", "th", "td",

    // Forms
    "form", "input", "textarea", "select", "option", "button", "label",
    "fieldset", "legend",

    // Media
    "img", "audio", "video", "canvas", "svg",

    // Interactive
    "details", "summary", "dialog",
}
```

### 3.4 Extensibility

Projects can register additional element names:

```sigil
//@ rune: element_names(icon, logo, card)
```

---

## 4. Attribute Syntax

### 4.1 Attribute Forms

```sigil
// String literal
div class="container" { }

// Expression (braces required for non-literal)
button onclick={handle_click} { }
input value={state.text} { }

// Boolean shorthand
button disabled { }  // equivalent to disabled={true}

// Spread attributes
div ...base_props class="override" { }
```

### 4.2 Attribute Compilation

```
compile_attributes(attrs) → Vec<(String, Expr)>:
    result ← []
    for attr in attrs:
        match attr:
            StringAttr(name, value):
                result.push((name, Expr::Literal(value)))
            ExprAttr(name, expr):
                result.push((name, compile_expr(expr)))
            BoolAttr(name):
                result.push((name, Expr::Literal(true)))
            SpreadAttr(expr):
                // Merge at runtime
                result.push(("...", compile_expr(expr)))
    return result
```

### 4.3 Reserved Attributes

| Attribute | Purpose | Type |
|-----------|---------|------|
| `key` | List diffing identity | any hashable |
| `ref` | Element reference capture | Ref<Element> |
| `class` | CSS class names | String |
| `style` | Inline styles | String or StyleObject |

---

## 5. Children Compilation

### 5.1 Child Types

```sigil
div {
    "Static text"           // TextChild
    { dynamic_value }       // ExprChild
    span { "Nested" }       // ElementChild
    MyComponent { prop }    // ComponentChild
    ⎇ cond { a } ⎉ { b }   // ConditionalChild
    ∀ x ∈ xs { li { x } }  // IterChild
}
```

### 5.2 Compilation Strategy

```
compile_element(name, attrs, children) → Expr:
    // Build VNode construction
    vnode ← Expr::Call("VNode::element", [Expr::Literal(name)])

    // Add attributes
    for (attr_name, attr_expr) in compile_attributes(attrs):
        vnode ← Expr::MethodCall(vnode, "attr", [attr_name, attr_expr])

    // Add children
    for child in children:
        child_expr ← compile_child(child)
        vnode ← Expr::MethodCall(vnode, "child", [child_expr])

    return vnode

compile_child(child) → Expr:
    match child:
        TextChild(text):
            return Expr::Call("VNode::text", [Expr::Literal(text)])
        ExprChild(expr):
            // Runtime conversion via Into<VNode>
            return Expr::Call("VNode::from", [compile_expr(expr)])
        ElementChild(elem):
            return compile_element(elem)
        ComponentChild(comp, props):
            return compile_component(comp, props)
        ConditionalChild(cond, then_child, else_child):
            return Expr::If(cond, compile_child(then_child), compile_child(else_child))
        IterChild(var, iter, body):
            // Generates VNode::fragment with mapped children
            return Expr::Call("VNode::fragment", [
                Expr::MethodCall(iter, "map", [
                    Expr::Closure([var], compile_child(body))
                ])
            ])
```

---

## 6. Components

### 6.1 Component Invocation

Components are uppercase identifiers:

```sigil
// Simple invocation
Button { "Click me" }

// With props
UserCard user={current_user} show_avatar={true} {
    // Children passed as `children` prop
    Badge { "Pro" }
}

// Namespaced
ui·Button variant={ButtonVariant·Primary} { "Submit" }
```

### 6.2 Component Definition

Components are functions or structs implementing `Component`:

```sigil
// Function component
☉ rite Greeting(name: String!) -> VNode! {
    div class="greeting" {
        "Hello, "
        { name }
        "!"
    }
}

// Struct component
☉ Σ Counter {
    initial: i32 = 0,
}

⊢ Component for Counter {
    rite render(self, ctx: &RenderContext!) -> VNode! {
        ≔ count = ctx·use_state(self.initial);
        div {
            p { "Count: " { count.get() } }
            button onclick={|_| count.update(|n| n + 1)} { "+" }
        }
    }
}
```

---

## 7. Expression Integration

### 7.1 Inline Expressions

Expressions in braces are evaluated and converted to children:

```sigil
div {
    { format!("Score: {}", score) }     // String → text node
    { maybe_node? }                      // Option<VNode> → conditional
    { nodes }                            // Vec<VNode> → fragment
}
```

### 7.2 Evidentiality in Views

```sigil
div {
    // Known data renders directly
    { user.name! }

    // Uncertain data shows loading state
    { user.avatar? ⎇ { img src={it} } ⎉ { Skeleton {} } }

    // Reported data shows indicator
    { data~ }
}
```

---

## 8. Parser Integration

### 8.1 Parse Context

The parser tracks context to disambiguate:

```
ParseContext {
    in_view_block: bool,
    known_elements: HashSet<String>,
    known_components: HashSet<String>,
}
```

### 8.2 Entry Points

View syntax activates in specific contexts:

1. After `=` in variable binding with element/component name
2. Inside view block `{}`
3. As return expression in component function

### 8.3 Lookahead Requirements

```
disambiguate_brace_expr(name):
    if name ∈ known_elements:
        peek ← lookahead(2)
        if peek matches (Ident, Colon):
            → struct_literal   // div { field: value }
        else:
            → element          // div { "content" }

    if name starts with uppercase:
        // Could be struct or component
        peek ← lookahead(2)
        if peek matches (Ident, Colon):
            // Ambiguous: props look like fields
            // Resolve via type/import information
            → component_or_struct
        else:
            → component

    → struct_literal
```

---

## 9. WASM Compilation

### 9.1 VNode Representation

```
VNode in WASM memory:

┌────────────────────────────────────┐
│ tag: u8                            │  0 = Element, 1 = Text, 2 = Fragment
├────────────────────────────────────┤
│ data: i64 (pointer or inline)      │
└────────────────────────────────────┘

Element data:
┌────────────────────────────────────┐
│ name_ptr: i32                      │
│ attrs_ptr: i32                     │
│ children_ptr: i32                  │
│ children_len: i32                  │
└────────────────────────────────────┘
```

### 9.2 Compilation Example

```sigil
// Source
div class="greeting" {
    "Hello, "
    { name }
}

// Compiled WASM (pseudocode)
call $alloc_vnode_element
  i32.const offset("div")     // name
  call $alloc_attrs
    i32.const 1               // attr count
    i32.const offset("class")
    i32.const offset("greeting")
  call $alloc_children
    i32.const 2               // child count
    call $alloc_vnode_text
      i32.const offset("Hello, ")
    call $vnode_from_string
      local.get $name
```

---

## 10. Constraints & Invariants

### 10.1 Syntactic Invariants

```
I1: Element names are always lowercase ASCII identifiers
I2: Component names always start with uppercase
I3: Children cannot contain bare identifiers (must be { expr } or element)
I4: Attributes before children: `div class="x" { }` not `div { } class="x"`
```

### 10.2 Semantic Invariants

```
I5: `key` attribute must be unique among siblings
I6: Components must return exactly one VNode (use fragment for multiple)
I7: Event handler attributes (on*) must be callable
I8: `ref` captures happen after render, not during
```

---

## 11. Error Conditions

| Condition | Error |
|-----------|-------|
| Unknown element name | "unknown element 'xyz', did you mean component 'Xyz'?" |
| Missing closing brace | "unclosed element 'div' started at line N" |
| Invalid attribute | "attribute 'onclick' expects function, got string" |
| Ambiguous syntax | "ambiguous: 'Foo { x }' could be struct or component, use 'Foo { x: x }' for struct" |
| Child in void element | "'input' cannot have children" |

---

## 12. Open Questions

1. **Self-closing syntax**: Should we support `<div />` or require `div {}`?
   - Pro: Familiar to JSX users
   - Con: Introduces angle brackets, diverges from Sigil aesthetic

2. **Attribute shorthand**: Should `{name}` expand to `name={name}`?
   - Pro: Less repetition for common pattern
   - Con: Implicit behavior may confuse

3. **Fragment syntax**: How to express multiple root elements?
   - Option A: `<> a b c </>` (JSX-style)
   - Option B: `fragment { a b c }`
   - Option C: Implicit in component returns

4. **Style objects**: Should we support `style={{ color: "red" }}`?
   - Pro: Type-safe styling
   - Con: Adds complexity, CSS-in-Sigil is alternative

5. **Event handler syntax**: `onclick={handler}` or `on·click={handler}`?
   - Current: `onclick` (HTML-style)
   - Alternative: `on·click` (Sigil method-call style)

---

## 13. Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Element detection | ❌ | Requires parser lookahead |
| Attribute parsing | ❌ | Basic string attrs first |
| Text children | ❌ | String literals in element body |
| Expression children | ❌ | `{ expr }` syntax |
| Nested elements | ❌ | Recursive parsing |
| Component invocation | ❌ | Uppercase detection |
| Conditional children | ❌ | `⎇/⎉` in view context |
| Iteration | ❌ | `∀/∈` in view context |
| WASM compilation | ❌ | VNode construction |

---

## 14. References

- [JSX Specification](https://facebook.github.io/jsx/) - Inspiration for syntax
- [Elm HTML](https://package.elm-lang.org/packages/elm/html/latest/) - Functional approach
- [SwiftUI](https://developer.apple.com/xcode/swiftui/) - Declarative DSL patterns
- UI-COMPONENTS.md - Qliphoth component specifications

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-02-14 | Initial draft. Gap discovered during qliphoth WASM compilation. |
