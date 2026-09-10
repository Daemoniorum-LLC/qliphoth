# React→Qliphoth Migration: Agent-TDD Roadmap

**Status:** Draft
**Active Spec:** [REACT-MIGRATION.md](./REACT-MIGRATION.md)
**SDD Phase:** Learn → **Specify** (spec complete, entering Implement)
**Author:** Claude (Conclave session: react-migration-design-2026-02-15)

---

## Philosophy

> "Tests are crystallized understanding, not coverage theater."

This roadmap follows Agent-TDD: tests express what we *understand* about React→Qliphoth transformation. Each test answers: **"How do we know this migration is correct?"**

**Key principle:** When any test reveals spec inadequacy → STOP → Update spec → Continue with correct foundation.

---

## Phase Structure

```
┌─────────────────────────────────────────────────────────────────┐
│  PHASE 1: React Extraction                                      │
│  ├── 1.1 JSX Parsing (swc integration)                         │
│  ├── 1.2 Component Detection                                    │
│  ├── 1.3 Hook Extraction                                        │
│  └── 1.4 Type Extraction                                        │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 2: Spec Generation                                       │
│  ├── 2.1 Recommendation Engine                                  │
│  ├── 2.2 Pattern Matching                                       │
│  ├── 2.3 Ambiguity Detection                                    │
│  └── 2.4 Dependency Analysis                                    │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 3: Qliphoth Generation                                   │
│  ├── 3.1 Actor Structure Generation                             │
│  ├── 3.2 VNode Builder Generation                               │
│  ├── 3.3 Message Handler Generation                             │
│  └── 3.4 Import/Module Generation                               │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 4: MCP Interface                                         │
│  ├── 4.1 Tool Implementation                                    │
│  ├── 4.2 Resource Endpoints                                     │
│  └── 4.3 State Persistence                                      │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 5: CLI Integration                                       │
│  ├── 5.1 Command Parsing                                        │
│  ├── 5.2 File Discovery                                         │
│  └── 5.3 Output Generation                                      │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 6: Extraction Fidelity ✅ (COMPLETED 2026-02-16)         │
│  ├── 6.1 Full Type Extraction ✅                                │
│  ├── 6.2 Helper Function Extraction ✅                          │
│  ├── 6.3 Handler Body Analysis ✅                               │
│  ├── 6.4 Hook Argument Expansion ✅                             │
│  └── 6.5 Architecture Mapping ✅                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: React Extraction

### 1.1 JSX Parsing

**Behavioral Contract:**
- Given valid React/TSX source → produce parsed AST
- Given invalid source → produce clear error with location
- Given non-React file → detect and skip gracefully

**Property Tests:**

```
∀ valid_tsx: String where is_valid_tsx(valid_tsx):
    parse(valid_tsx) → Ok(Ast)

∀ jsx_element ∈ parsed_ast:
    jsx_element.tag ∈ String
    jsx_element.props ∈ Vec<Prop>
    jsx_element.children ∈ Vec<JsxNode>

∀ source: String:
    parse(source).is_ok() ⟹ parse(source).unwrap().can_serialize()
```

**Specification Tests:**

| Test | Input | Expected Output | Status |
|------|-------|-----------------|--------|
| `test_parse_simple_element` | `<div>hello</div>` | JsxElement { tag: "div", children: [Text("hello")] } | 🔮 |
| `test_parse_nested_elements` | `<div><span>x</span></div>` | Nested structure preserved | 🔮 |
| `test_parse_component` | `<Counter />` | JsxElement { tag: "Counter", is_component: true } | 🔮 |
| `test_parse_props` | `<div className="x" id="y">` | Props extracted correctly | 🔮 |
| `test_parse_expression` | `<div>{count}</div>` | Expression node with identifier | 🔮 |
| `test_parse_event_handler` | `<button onClick={fn}>` | Event prop with handler reference | 🔮 |
| `test_parse_spread_props` | `<div {...props}>` | Spread prop detected | 🔮 |
| `test_parse_fragment` | `<><A/><B/></>` | Fragment with children | 🔮 |
| `test_parse_conditional` | `{cond && <X/>}` | Logical expression node | 🔮 |
| `test_parse_map` | `{items.map(i => <X key={i}/>)}` | Call expression with arrow | 🔮 |
| `test_invalid_jsx_error` | `<div><span></div>` | Error with line/column | 🔮 |

**Boundary Tests:**
- File with mixed JSX and non-JSX code
- TypeScript generics in JSX: `<Component<T>>`
- Self-closing vs explicit closing tags
- Unicode in JSX content

**Quality Gate:** All specification tests pass, property tests hold for 1000+ generated inputs.

---

### 1.2 Component Detection

**Behavioral Contract:**
- Identify function components (named and arrow)
- Identify class components
- Identify forwardRef/memo wrappers
- Distinguish components from regular functions

**Property Tests:**

```
∀ component ∈ detected_components:
    component.name ∈ String ∧ component.name[0].is_uppercase()

∀ function ∈ source where returns_jsx(function):
    function ∈ detected_components ∨ is_helper_function(function)

∀ class ∈ source where extends_react_component(class):
    class ∈ detected_components
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_detect_function_component` | `function Counter() { return <div/> }` | ComponentExtraction { type: "functional" } | 🔮 |
| `test_detect_arrow_component` | `const Counter = () => <div/>` | ComponentExtraction { type: "functional" } | 🔮 |
| `test_detect_class_component` | `class Counter extends Component` | ComponentExtraction { type: "class" } | 🔮 |
| `test_detect_memo_wrapper` | `memo(function X() {})` | ComponentExtraction { type: "memo" } | 🔮 |
| `test_detect_forward_ref` | `forwardRef((props, ref) => ...)` | ComponentExtraction { type: "forwardRef" } | 🔮 |
| `test_ignore_helper_function` | `function formatDate() { return str }` | Not in components list | 🔮 |
| `test_multiple_components` | File with 3 components | All 3 detected | 🔮 |
| `test_exported_default` | `export default Counter` | exported: true, exportType: "default" | 🔮 |
| `test_exported_named` | `export { Counter }` | exported: true, exportType: "named" | 🔮 |

**Quality Gate:** 100% accuracy on test corpus of 50+ real React components.

---

### 1.3 Hook Extraction

**Behavioral Contract:**
- Identify all React hook calls within components
- Extract hook parameters and return values
- Track dependencies arrays
- Detect custom hook usage

**Property Tests:**

```
∀ hook_call ∈ extracted_hooks:
    hook_call.hook_type ∈ HookType
    hook_call.location.line > 0

∀ use_effect ∈ extracted_hooks where hook_type == UseEffect:
    use_effect.dependencies ∈ { Vec<String>, "none", "empty" }

∀ use_state ∈ extracted_hooks where hook_type == UseState:
    use_state.state_name.is_some() ∧ use_state.setter_name.is_some()
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_extract_use_state` | `const [x, setX] = useState(0)` | HookUsage { type: UseState, stateName: "x", setterName: "setX", initial: 0 } | 🔮 |
| `test_extract_use_effect_empty_deps` | `useEffect(() => {}, [])` | dependencies: "empty" (mount only) | 🔮 |
| `test_extract_use_effect_with_deps` | `useEffect(() => {}, [a, b])` | dependencies: ["a", "b"] | 🔮 |
| `test_extract_use_effect_no_deps` | `useEffect(() => {})` | dependencies: "none" (every render) | 🔮 |
| `test_extract_use_callback` | `useCallback(() => x, [x])` | HookUsage { type: UseCallback, deps: ["x"] } | 🔮 |
| `test_extract_use_memo` | `useMemo(() => expensive(), [])` | HookUsage { type: UseMemo } | 🔮 |
| `test_extract_use_ref` | `const ref = useRef(null)` | HookUsage { type: UseRef, refName: "ref" } | 🔮 |
| `test_extract_use_context` | `useContext(ThemeCtx)` | HookUsage { type: UseContext, contextName: "ThemeCtx" } | 🔮 |
| `test_extract_custom_hook` | `const data = useQuery(key)` | HookUsage { type: Custom, name: "useQuery" } | 🔮 |
| `test_extract_use_reducer` | `const [s, d] = useReducer(r, i)` | HookUsage { type: UseReducer } | 🔮 |
| `test_multiple_hooks` | Component with 5 hooks | All 5 extracted in order | 🔮 |

**Quality Gate:** Correct extraction for all hooks in Infernum Observer codebase.

---

### 1.4 Type Extraction

**Behavioral Contract:**
- Extract TypeScript interfaces and type aliases
- Map fields with types, optionality, defaults
- Handle generics and union types
- Preserve JSDoc comments

**Property Tests:**

```
∀ interface ∈ extracted_types:
    interface.name ∈ String
    interface.fields ∈ Vec<Field>

∀ field ∈ interface.fields:
    field.name ∈ String
    field.type_annotation ∈ String
    field.optional ∈ bool
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_extract_interface` | `interface Props { name: string }` | TypeExtraction { kind: "interface", fields: [...] } | 🔮 |
| `test_extract_type_alias` | `type ID = string \| number` | TypeExtraction { kind: "type" } | 🔮 |
| `test_extract_optional_field` | `{ name?: string }` | Field { optional: true } | 🔮 |
| `test_extract_readonly_field` | `{ readonly id: number }` | Field { readonly: true } | 🔮 |
| `test_extract_generics` | `interface Box<T> { value: T }` | typeParams: ["T"] | 🔮 |
| `test_extract_enum` | `enum Status { A, B }` | TypeExtraction { kind: "enum", variants: ["A", "B"] } | 🔮 |

**Quality Gate:** Parse all types from @daemoniorum/* packages without errors.

---

## Phase 2: Spec Generation

### 2.1 Recommendation Engine

**Behavioral Contract:**
- Given ReactExtraction → produce MigrationSpec with recommendations
- Each hook maps to a Qliphoth pattern
- Each event handler maps to a message

**Property Tests:**

```
∀ useState_hook ∈ extraction.hooks:
    ∃ state_field ∈ spec.recommendations.stateFields:
        state_field.fromHook == useState_hook.id

∀ onClick_handler ∈ extraction.handlers:
    ∃ message ∈ spec.recommendations.messages:
        message.fromHandler == onClick_handler.name

∀ spec.recommendations:
    spec.recommendations.stateFields.len() >= extraction.hooks.filter(UseState).len()
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_recommend_state_field` | useState("count", 0) | StateFieldRec { name: "count", type: "i32", evidentiality: "!" } | 🔮 |
| `test_recommend_message_from_handler` | onClick={() => setX(...)} | MessageRec { name: "SetX", fromHandler: "onClick" } | 🔮 |
| `test_recommend_mount_effect` | useEffect(..., []) | EffectRec { strategy: "lifecycle", event: "Mount" } | 🔮 |
| `test_recommend_inline_effect` | useEffect(..., [count]) | EffectRec { strategy: "inline", inlineIn: "all handlers that change count" } | 🔮 |
| `test_recommend_remove_callback` | useCallback(...) | No recommendation (removed) | 🔮 |
| `test_recommend_actor_pattern` | Component with state | target.pattern: "actor" | 🔮 |
| `test_recommend_function_pattern` | Pure component, no hooks | target.pattern: "function" | 🔮 |

**Quality Gate:** Recommendations match human expert judgment for 10 sample components.

---

### 2.2 Pattern Matching

**Behavioral Contract:**
- Include relevant Qliphoth patterns in spec
- Match patterns based on React constructs found
- Provide concrete code examples

**Specification Tests:**

| Test | Input | Expected Patterns | Status |
|------|-------|-------------------|--------|
| `test_pattern_for_usestate` | Component with useState | "useState_to_state" pattern included | 🔮 |
| `test_pattern_for_onclick` | Button with onClick | "onClick_to_message" pattern included | 🔮 |
| `test_pattern_for_map` | items.map(...) | "list_render" pattern included | 🔮 |
| `test_pattern_for_conditional` | {cond && <X/>} | "conditional_render" pattern included | 🔮 |
| `test_no_duplicate_patterns` | Any input | patterns.unique() | 🔮 |

---

### 2.3 Ambiguity Detection

**Behavioral Contract:**
- Detect when multiple valid Qliphoth patterns apply
- Generate question with options and recommendation
- Track ambiguities for agent resolution

**Property Tests:**

```
∀ ambiguity ∈ spec.ambiguities:
    ambiguity.options.len() >= 2
    ambiguity.defaultChoice < ambiguity.options.len()
    ∃ opt ∈ ambiguity.options: opt.recommended == true
```

**Specification Tests:**

| Test | Input | Expected Ambiguity | Status |
|------|-------|-------------------|--------|
| `test_ambiguity_effect_placement` | useEffect with deps | "Where should this effect logic go?" | 🔮 |
| `test_ambiguity_callback_prop` | onSomething prop passed down | "How to handle parent callback?" | 🔮 |
| `test_no_ambiguity_simple` | Simple counter | ambiguities: [] | 🔮 |

---

### 2.4 Dependency Analysis

**Behavioral Contract:**
- Detect component dependencies (imports other components)
- Order migrations to respect dependencies
- Flag circular dependencies

**Property Tests:**

```
∀ component ∈ spec.components:
    ∀ dep ∈ component.dependencies:
        dep ∈ spec.components ∨ dep.is_external()

// No circular dependencies
∀ component_a, component_b ∈ spec.components:
    a.depends_on(b) ∧ b.depends_on(a) ⟹ flagged_as_circular
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_detect_component_import` | `import { Button } from './Button'` | dependencies: ["Button"] | 🔮 |
| `test_detect_type_import` | `import type { Props }` | types: ["Props"] | 🔮 |
| `test_order_by_dependency` | A imports B | B appears before A in migration order | 🔮 |
| `test_flag_circular` | A ↔ B | Both flagged, warning issued | 🔮 |

---

## Phase 3: Qliphoth Generation

### 3.1 Actor Structure Generation

**Behavioral Contract:**
- Generate syntactically valid Sigil actor
- Include all state fields from recommendations
- Generate message enum from recommendations

**Property Tests:**

```
∀ generated_actor:
    sigil_parse(generated_actor).is_ok()

∀ state_field ∈ recommendations:
    state_field.name ∈ generated_actor.fields

∀ message ∈ recommendations:
    message.name ∈ generated_actor.message_enum
```

**Specification Tests:**

| Test | Input Recommendations | Expected Sigil | Status |
|------|----------------------|----------------|--------|
| `test_generate_empty_actor` | No state, no messages | `actor X { rite view() -> VNode! { ... } }` | 🔮 |
| `test_generate_actor_with_state` | state count: i32! | `state count: i32! = 0,` in actor body | 🔮 |
| `test_generate_message_enum` | [Increment, Decrement] | `ᛈ XMsg { Increment, Decrement }` | 🔮 |
| `test_generate_message_handlers` | Increment → count += 1 | `on Increment { self.count += 1; }` | 🔮 |
| `test_generate_constructor` | initial_count prop | `rite new(initial_count: i32) -> This!` | 🔮 |

**Quality Gate:** Generated Sigil parses without errors for all test cases.

---

### 3.2 VNode Builder Generation

**Behavioral Contract:**
- Convert JSX tree to VNode builder chain
- Preserve structure and attributes
- Handle events, conditionals, loops

**Property Tests:**

```
∀ jsx_element ∈ input:
    ∃ vnode_call ∈ output:
        vnode_call.tag == jsx_element.tag (lowercased)

∀ jsx_prop ∈ input where !is_event(jsx_prop):
    prop_value ∈ output.builder_calls
```

**Specification Tests:**

| Test | JSX Input | Expected VNode | Status |
|------|-----------|----------------|--------|
| `test_gen_simple_div` | `<div>` | `VNode·div()` | 🔮 |
| `test_gen_with_class` | `<div className="x">` | `VNode·div()·class("x")` | 🔮 |
| `test_gen_with_id` | `<div id="y">` | `VNode·div()·id("y")` | 🔮 |
| `test_gen_with_attr` | `<a href="/x">` | `VNode·a()·attr("href", "/x")` | 🔮 |
| `test_gen_with_style` | `<div style={{color: "red"}}>` | `VNode·div()·style("color", "red")` | 🔮 |
| `test_gen_text_child` | `<span>hello</span>` | `VNode·span()·text_child("hello")` | 🔮 |
| `test_gen_nested` | `<div><span/></div>` | `VNode·div()·child(VNode·span())` | 🔮 |
| `test_gen_fragment` | `<><A/><B/></>` | `VNode·fragment()·child(A)·child(B)` | 🔮 |
| `test_gen_conditional` | `{cond && <X/>}` | `·when(cond, X·view())` | 🔮 |
| `test_gen_event` | `<button onClick={...}>` | `VNode·button()·on_click(MsgName)` | 🔮 |
| `test_gen_expression` | `{count}` | `·text_child(self.count·to_string())` | 🔮 |

**Quality Gate:** Visual equivalence when rendered (same DOM structure).

---

### 3.3 Message Handler Generation

**Behavioral Contract:**
- Generate `on MessageName { }` blocks
- Include state mutations from original handlers
- Include side effects extracted from useEffect

**Specification Tests:**

| Test | Input Handler | Expected Handler | Status |
|------|---------------|------------------|--------|
| `test_gen_simple_handler` | `setCount(c => c + 1)` | `on Increment { self.count += 1; }` | 🔮 |
| `test_gen_handler_with_effect` | setCount + useEffect[count] | Handler includes effect logic inline | 🔮 |
| `test_gen_handler_with_payload` | `onSelect(id)` | `on Select { ≔ id = msg.id; ... }` | 🔮 |

---

### 3.4 Import/Module Generation

**Behavioral Contract:**
- Generate correct Qliphoth imports
- Map React imports to Qliphoth equivalents
- Handle qliphoth-sys for browser APIs

**Specification Tests:**

| Test | React Import | Qliphoth Import | Status |
|------|--------------|-----------------|--------|
| `test_gen_prelude` | Any component | `invoke qliphoth·prelude·*;` | 🔮 |
| `test_gen_router_import` | `useNavigate` | `invoke qliphoth_router·*;` | 🔮 |
| `test_gen_sys_import` | `document.title` | `invoke qliphoth_sys·*;` | 🔮 |

---

## Phase 4: MCP Interface

### 4.1 Tool Implementation

**Behavioral Contract:**
- Each MCP tool operates correctly
- Errors are returned as structured responses
- State is maintained across calls

**Specification Tests:**

| Test | Tool Call | Expected | Status |
|------|-----------|----------|--------|
| `test_list_migrations_empty` | list_migrations() on empty | [] | 🔮 |
| `test_list_migrations_populated` | After extraction | [{ id, name, status }] | 🔮 |
| `test_get_migration` | get_migration("counter") | Full ComponentMigrationSpec | 🔮 |
| `test_get_migration_not_found` | get_migration("xxx") | Error: not found | 🔮 |
| `test_validate_sigil_valid` | validate_sigil(valid_code) | { valid: true } | 🔮 |
| `test_validate_sigil_invalid` | validate_sigil("garbage") | { valid: false, errors: [...] } | 🔮 |
| `test_complete_migration` | complete_migration(id, code) | File written, status updated | 🔮 |

---

### 4.2 Resource Endpoints

**Specification Tests:**

| Test | Resource | Expected | Status |
|------|----------|----------|--------|
| `test_resource_pending` | migrations://pending | List of pending migrations | 🔮 |
| `test_resource_patterns` | migrations://patterns | Pattern library | 🔮 |
| `test_resource_component` | migrations://component/counter | ComponentMigrationSpec | 🔮 |

---

## Phase 5: CLI Integration

### 5.1 Command Parsing

**Specification Tests:**

| Test | Command | Expected | Status |
|------|---------|----------|--------|
| `test_parse_from_react` | `sigil migrate --from-react ./src` | MigrateReact { path: "./src" } | 🔮 |
| `test_parse_dry_run` | `--dry-run` | dry_run: true | 🔮 |
| `test_parse_output` | `-o ./out` | output_dir: Some("./out") | 🔮 |
| `test_parse_serve` | `--serve` | Start MCP server mode | 🔮 |

---

### 5.2 File Discovery

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_find_tsx_files` | Directory with .tsx | All .tsx files found | 🔮 |
| `test_skip_node_modules` | Directory with node_modules | node_modules skipped | 🔮 |
| `test_include_pattern` | `--include "*.tsx"` | Only .tsx files | 🔮 |
| `test_exclude_pattern` | `--exclude "*.test.tsx"` | Test files excluded | 🔮 |

---

## Phase 6: Extraction Fidelity ✅

> **Gap Discovery:** During infernum-observer migration test (2026-02-16), we found that structured extraction is insufficient for agent-only migration. Agents must parse `source.code` to complete migrations. This phase addresses that gap.
>
> **Completed:** 2026-02-16. All 5 sub-phases implemented with 25 tests. Quality reviews completed.

**Reference Spec:** [REACT-MIGRATION-PHASE6-ENHANCEMENTS.md](../../../sigil-lang/docs/specs/REACT-MIGRATION-PHASE6-ENHANCEMENTS.md)

### 6.1 Full Type Extraction

**Behavioral Contract:**
- Extract all interface/type fields with type annotations
- Mark optional fields correctly
- Preserve union type variants
- Handle extends/inheritance
- Result is sufficient to generate Qliphoth Σ without parsing source

**Property Tests:**

```
∀ interface I in source:
    extract_type(I).fields.len() == I.field_count

∀ field F in interface:
    extract_type(I).field(F.name).type_annotation.is_some()
    extract_type(I).field(F.name).optional == F.has_question_mark

∀ extracted_type T:
    generate_qliphoth_sigma(T).compiles() ∧ !requires_source_parsing(T)
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_type_extraction_captures_all_fields` | `interface Props { a: string; b: number }` | 2 fields with types | ✅ |
| `test_type_extraction_marks_optional_fields` | `{ name?: string }` | optional: true | ✅ |
| `test_type_extraction_preserves_union_types` | `type Role = 'user' \| 'admin'` | variants: ["user", "admin"] | ✅ |
| `test_type_extraction_handles_extends` | `interface B extends A` | extends: ["A"], merged fields | ✅ |
| `test_type_extraction_resolves_type_references` | `field: OtherType` | type resolved or marked external | ✅ |
| `test_type_extraction_handles_generics` | `interface Box<T> { value: T }` | type_params: ["T"] | 🔮 |
| `test_type_extraction_function_types` | `onClick: (e: Event) => void` | kind: "function", params, return | 🔮 |
| `test_type_extraction_array_types` | `items: string[]` | kind: "array", element_type: "string" | 🔮 |
| `test_type_extraction_record_types` | `map: Record<string, number>` | kind: "record", key, value types | 🔮 |
| `test_type_extraction_doc_comments` | `/** Description */ field: T` | doc: "Description" | 🔮 |

**Quality Gate:** `ButtonProps` extraction includes all 10 fields. `generate_qliphoth_sigma(extract_type(ButtonProps))` produces valid Sigil.

---

### 6.2 Helper Function Extraction

**Behavioral Contract:**
- Extract module-scope functions
- Extract component-scope helper functions
- Capture parameters with types
- Capture return type
- Detect purity (no side effects)
- Track which components reference the function

**Property Tests:**

```
∀ function F referenced by component C:
    F ∈ extraction.helper_functions

∀ helper H:
    H.parameters.all(|p| p.type_annotation.is_some() ∨ p.inferred_type.is_some())
    H.return_type.is_some() ∨ H.inferred_return.is_some()

∀ pure_function P where no_side_effects(P):
    extract(P).is_pure == true
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_helper_extraction_finds_module_scope_functions` | `function format() {...}` at top level | In helper_functions | ✅ |
| `test_helper_extraction_finds_component_scope_functions` | `const helper = () => ...` inside component | In helper_functions | ✅ |
| `test_helper_extraction_captures_parameters_and_return_type` | `function f(a: string, b: number): string` | params + return_type | ✅ |
| `test_helper_extraction_detects_purity` | `add(a,b)` pure, `log(x)` impure | is_pure: true/false | ✅ |
| `test_helper_extraction_tracks_usage_sites` | Helper used in component | Helper found with metadata | ✅ |
| `test_helper_extraction_infers_return_type` | `function f() { return 42 }` | inferred_return: "number" | 🔮 |
| `test_helper_extraction_arrow_functions` | `const f = (x) => x * 2` | Extracted as helper | 🔮 |
| `test_helper_extraction_async_functions` | `async function fetch()` | is_async: true | 🔮 |

**Quality Gate:** `transformToSigilEvents` from ChatPanel.tsx appears in extraction with full signature.

---

### 6.3 Handler Body Analysis

**Behavioral Contract:**
- Parse handler function bodies completely
- Extract all function calls with their sources
- Identify state mutations
- Detect early returns and conditionals
- Track side effects

**Property Tests:**

```
∀ handler H:
    ∀ function_call C in H.body:
        C ∈ extraction.handlers[H].statements

∀ call C to custom_hook_function F:
    C.source == "custom_hook:{hook_name}"

∀ handler H that calls setState:
    extract(H).state_mutations.len() > 0
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_handler_body_extracts_function_calls` | `handler() { foo(); bar(); }` | calls: [call(foo), call(bar)] | ✅ |
| `test_handler_body_identifies_call_sources` | `setCount`, `fetch` | StateSetter, Global sources | ✅ |
| `test_handler_body_detects_early_returns` | `if (!x) return;` | has_early_return: true | ✅ |
| `test_handler_body_captures_conditionals` | `if (cond) { a() }` | has_conditionals: true | ✅ |
| `test_handler_body_infers_state_mutations` | `setCount(c => c + 1)` | state_mutations detected | ✅ |
| `test_handler_body_identifies_prop_sources` | `props.onComplete()` | source: "prop:onComplete" | 🔮 |
| `test_handler_body_identifies_import_sources` | `lodash.debounce()` | source: "import:lodash" | 🔮 |
| `test_handler_body_tracks_side_effects` | `fetch('/api')` | side_effects: [{type: "network"}] | 🔮 |
| `test_handler_body_nested_calls` | `foo(bar(x))` | Both calls captured | 🔮 |
| `test_handler_body_chained_calls` | `arr.filter().map()` | Chain captured | 🔮 |

**Quality Gate:** `handleSend` from ChatPanel shows calls to `addMessage` (useChat) and `runAgent` (useAgent).

---

### 6.4 Hook Argument Expansion

**Behavioral Contract:**
- Fully expand object arguments to hooks
- Parse arrow function arguments as handler bodies
- Preserve array arguments (query keys, deps)
- Handle nested objects

**Property Tests:**

```
∀ hook_call H with object_arg O:
    ∀ property P in O:
        P ∈ extract(H).arguments[0].properties

∀ callback C in hook_args:
    C.body is analyzed per Handler Body Analysis (6.3)

∀ array_arg A:
    A.elements.all(|e| e.is_extracted())
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_hook_args_expand_object_properties` | `useHook({ a: 1, b: 2 })` | args[0].properties: [a, b] | ✅ |
| `test_hook_args_capture_arrow_functions` | `useStore((s) => s.user)` | Function with params and body | ✅ |
| `test_hook_args_analyze_callback_bodies` | `{ onSuccess: () => updateCache() }` | Callback with calls and side_effects | ✅ |
| `test_hook_args_preserve_array_arguments` | `useQuery({ queryKey: ['users'] })` | Array elements preserved | ✅ |
| `test_hook_args_handle_nested_objects` | `{ options: { retry: 3 } }` | Nested structure preserved | ✅ |
| `test_hook_args_shorthand_properties` | `{ loading, error }` | properties with shorthand: true | 🔮 |
| `test_hook_args_spread_properties` | `{ ...defaults, custom: 1 }` | spread + explicit properties | 🔮 |
| `test_hook_args_computed_properties` | `{ [key]: value }` | computed_key: true | 🔮 |
| `test_hook_args_function_expression` | `{ fn: function() {} }` | type: "function_expression" | 🔮 |
| `test_hook_args_template_literals` | `useQuery(\`/api/${id}\`)` | template with expressions | 🔮 |

**Quality Gate:** `useAgent({onComplete: (answer) => addMessage(...)})` fully expanded with callback body analyzed.

---

### 6.5 Architecture Mapping

**Behavioral Contract:**
- Identify service actor boundaries from custom hooks
- Map Zustand stores to Qliphoth patterns
- Suggest message types for mutations
- Determine state ownership
- Recommend communication patterns

**Property Tests:**

```
∀ custom_hook H with stateful_returns:
    ∃ service_actor S ∈ architecture_mapping:
        S.derived_from == H.name

∀ zustand_store Z:
    Z.qliphoth_equivalent ∈ {"service_actor", "context"}

∀ mutation_function M from custom_hook:
    ∃ message_type T ∈ architecture_mapping:
        T.from_function == M.name
```

**Specification Tests:**

| Test | Input | Expected | Status |
|------|-------|----------|--------|
| `test_architecture_identifies_service_actors` | `useChat`, `useAgent` | ChatService, AgentService recommended | ✅ |
| `test_architecture_maps_zustand_stores` | `useAppStore(selector)` | AppActor with selectors/actions | ✅ |
| `test_architecture_suggests_message_types` | `save, load, reset` | Save, Load, Reset messages | ✅ |
| `test_architecture_determines_state_ownership` | Local useState + shared Zustand | Self vs Shared ownership | ✅ |
| `test_architecture_recommends_communication_patterns` | Hook functions in handlers | DataService detected | ✅ |
| `test_architecture_context_injection` | `useDisplaySettings()` | Context injection pattern | 🔮 |
| `test_architecture_read_only_state` | Selector returns only values | read_only access pattern | 🔮 |
| `test_architecture_bidirectional_state` | Hook returns getters + setters | Full actor with messages | 🔮 |
| `test_architecture_async_patterns` | `useQuery` usage | Async message pattern | 🔮 |
| `test_architecture_subscription_patterns` | `useMutation` with callbacks | Subscription/broadcast | 🔮 |

**Quality Gate:** ChatPanel extraction recommends ChatService + AgentService actors with clear ownership.

---

## Compliance Audit Checkpoints

After each phase, conduct compliance audit:

1. **Phase 1 Complete:** All extraction tests pass, property tests hold
2. **Phase 2 Complete:** Spec generation matches manual recommendations
3. **Phase 3 Complete:** Generated Sigil parses and type-checks
4. **Phase 4 Complete:** MCP tools work in Claude Code
5. **Phase 5 Complete:** CLI works end-to-end on Infernum Observer
6. **Phase 6 Complete:** Agent can migrate ChatPanel without reading `source.code`

---

## Integration Test: Infernum Observer

**The ultimate test:** Migrate Infernum Observer end-to-end.

**Acceptance Criteria:**
- [ ] All 30 components extracted correctly
- [ ] Migration specs generated with sensible recommendations
- [ ] Generated Sigil compiles to WASM
- [ ] Playwright E2E tests pass against Qliphoth version
- [ ] Performance within 10% of React version
- [ ] No manual intervention required (except ambiguity resolution)

---

## Test Infrastructure

### Location
```
sigil-lang/parser/src/migrate/
├── react/
│   ├── mod.rs
│   ├── extract.rs      # Phase 1
│   ├── specgen.rs      # Phase 2
│   ├── codegen.rs      # Phase 3
│   └── tests/
│       ├── extract_tests.rs
│       ├── specgen_tests.rs
│       ├── codegen_tests.rs
│       └── fixtures/
│           ├── simple_counter.tsx
│           ├── complex_form.tsx
│           └── ...
```

### Test Data
- **Fixtures:** Real React components from Infernum Observer
- **Property tests:** Use proptest crate for input generation
- **Snapshot tests:** Golden file comparisons for generated code

---

## Next Actions

1. **Create `parser/src/migrate/react/mod.rs`** - Module structure
2. **Add swc dependencies to Cargo.toml**
3. **Write first test: `test_parse_simple_element`**
4. **Implement until test passes**
5. **Proceed through Phase 1 tests**

**Remember:** When any test reveals spec inadequacy → STOP → Update REACT-MIGRATION.md → Continue.

---

*This roadmap is a living document. Update as understanding crystallizes.*
