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

## Compliance Audit Checkpoints

After each phase, conduct compliance audit:

1. **Phase 1 Complete:** All extraction tests pass, property tests hold
2. **Phase 2 Complete:** Spec generation matches manual recommendations
3. **Phase 3 Complete:** Generated Sigil parses and type-checks
4. **Phase 4 Complete:** MCP tools work in Claude Code
5. **Phase 5 Complete:** CLI works end-to-end on Infernum Observer

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
