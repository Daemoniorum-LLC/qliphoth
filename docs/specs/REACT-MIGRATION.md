# React to Qliphoth Migration System

**Status:** Draft
**Author:** Claude (Conclave session: evening-exploration-2026-02-15)
**Last Updated:** 2026-02-15

## 1. Overview

A migration system that transforms React/TypeScript applications into **Qliphoth** actor-based components. This is fundamentally different from the Rust→Sigil migration:

| Aspect | Rust → Sigil | React → Qliphoth |
|--------|--------------|------------------|
| **Type** | Syntactic | Semantic/Paradigm |
| **Scope** | Language keywords | Framework patterns |
| **Output** | Generic Sigil code | Qliphoth framework code |
| **Imports** | N/A | `qliphoth::prelude::*` |
| **Components** | N/A | Actors with message handlers |
| **Views** | N/A | VNode builder API |
| **Events** | N/A | Message dispatch |

**Critical distinction:** The output is not just "Sigil code" - it's **idiomatic Qliphoth code** that uses:
- `invoke qliphoth·prelude·*` for framework imports
- Actor components with `state` fields and `on Message { }` handlers
- `VNode·div()`, `VNode·button()`, etc. for view construction
- `.on_click(MessageName)` for event dispatch (not callbacks)
- `qliphoth-sys` for browser APIs (replacing `document.*`, `window.*`)
- `qliphoth-router` for routing (replacing React Router)

### 1.1 Design Philosophy

- **Agent-first**: Designed for AI agents to consume and produce, not for human typing
- **Deterministic extraction**: Parsing and structure extraction is deterministic (no LLM)
- **Semantic transformation**: LLM agents handle the paradigm mapping (hooks→actors, JSX→builders)
- **Rich context**: Specs contain everything an agent needs without file-hopping
- **Interactive validation**: MCP tools for real-time validation during migration

### 1.2 Scope

**Phase 1 (MVP):**
- Functional React components
- Core hooks: useState, useEffect, useCallback, useMemo, useRef
- JSX to VNode builder transformation
- Event handlers to message dispatch
- TypeScript types to Sigil types

**Phase 2:**
- Class components
- Custom hooks
- Context API
- React Router → qliphoth-router
- State management (Redux, Zustand, etc.)

**Phase 3:**
- Vue.js support (extraction layer is pluggable)
- Svelte support
- Framework-agnostic component detection

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        sigil migrate --from-react                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌────────────────┐   ┌────────────────┐   ┌────────────────────┐  │
│  │   Extractor    │──▶│   Spec Gen     │──▶│  MCP Server        │  │
│  │   (swc)        │   │                │   │  (optional)        │  │
│  └────────────────┘   └────────────────┘   └────────────────────┘  │
│          │                    │                     │              │
│          ▼                    ▼                     ▼              │
│  ┌────────────────┐   ┌────────────────┐   ┌────────────────────┐  │
│  │ ReactAST       │   │ MigrationSpec  │   │ Agent Interface    │  │
│  │ (internal)     │   │ (JSON files)   │   │ (tools + resources)│  │
│  └────────────────┘   └────────────────┘   └────────────────────┘  │
│                                                     │              │
│                               ┌─────────────────────┘              │
│                               ▼                                    │
│                       ┌────────────────┐                           │
│                       │   Validator    │                           │
│                       │ (sigil parser) │                           │
│                       └────────────────┘                           │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.1 Component Responsibilities

| Component | Language | Responsibility |
|-----------|----------|----------------|
| **Extractor** | Rust (swc) | Parse React/TSX, extract component structure |
| **Spec Generator** | Rust | Enrich extraction with Qliphoth patterns, recommendations |
| **MCP Server** | Rust | Serve specs to agents, validate output, track progress |
| **Validator** | Rust (sigil parser) | Parse and type-check generated Sigil code |

---

## 3. Extraction Phase

### 3.1 Parser: swc

Using `swc_ecma_parser` for:
- Full JSX/TSX support
- TypeScript type extraction
- Fast, Rust-native integration
- Battle-tested (used by Next.js, Turbopack)

**Crate dependencies:**
```toml
swc_ecma_parser = "0.144"
swc_ecma_ast = "0.113"
swc_common = "0.33"
```

### 3.2 Extracted Structure

The extractor produces a `ReactExtraction` for each source file:

```typescript
interface ReactExtraction {
  file: FileInfo;
  components: ComponentExtraction[];
  customHooks: CustomHookExtraction[];
  types: TypeExtraction[];
  imports: ImportInfo[];
  exports: ExportInfo[];
}

interface FileInfo {
  path: string;
  relativePath: string;  // relative to project root
  language: "typescript" | "javascript";
  hasJsx: boolean;
}

interface ComponentExtraction {
  name: string;
  type: "functional" | "class" | "forwardRef" | "memo";
  exported: boolean;
  exportType: "default" | "named" | null;
  location: SourceLocation;

  // Props
  props: PropExtraction[];
  propsType: string | null;  // name of interface/type if defined

  // Hooks (functional components)
  hooks: HookUsage[];

  // Class components
  classInfo: ClassComponentInfo | null;

  // JSX structure
  jsx: JsxTree;

  // Event handlers defined in component
  handlers: HandlerExtraction[];

  // Dependencies on other components
  childComponents: string[];
}

interface HookUsage {
  hookType: HookType;
  location: SourceLocation;

  // useState
  stateName: string | null;
  setterName: string | null;
  initialValue: Expression | null;

  // useEffect/useLayoutEffect
  dependencies: string[] | "none" | "empty";  // empty = [], none = no array
  effectBody: string;  // source code of effect function
  cleanupPresent: boolean;

  // useCallback/useMemo
  memoizedValue: string | null;
  memoDependencies: string[];

  // useRef
  refName: string | null;
  refInitial: Expression | null;

  // useContext
  contextName: string | null;

  // Custom hook
  customHookName: string | null;
  customHookArgs: Expression[];
  returnValue: string | null;
}

enum HookType {
  UseState,
  UseEffect,
  UseLayoutEffect,
  UseCallback,
  UseMemo,
  UseRef,
  UseContext,
  UseReducer,
  UseImperativeHandle,
  Custom
}

interface JsxTree {
  root: JsxNode;
}

interface JsxNode {
  nodeType: "element" | "fragment" | "text" | "expression" | "spread";

  // Element
  tag: string | null;  // "div", "Button", etc.
  isComponent: boolean;  // uppercase = component
  props: JsxProp[];
  children: JsxNode[];

  // Text
  text: string | null;

  // Expression
  expression: string | null;  // {count}, {items.map(...)}, etc.
  expressionType: "identifier" | "call" | "conditional" | "logical" | "other";
}

interface JsxProp {
  name: string;
  valueType: "string" | "expression" | "spread" | "boolean";
  value: string | null;
  isEventHandler: boolean;  // onClick, onSubmit, etc.
  handlerName: string | null;  // reference to handler function
}

interface HandlerExtraction {
  name: string;
  isInline: boolean;  // defined inline in JSX vs as separate function
  parameters: string[];
  body: string;  // source code
  stateAccess: StateAccess[];
  sideEffects: SideEffect[];
}

interface StateAccess {
  type: "read" | "write";
  stateName: string;
  setterCall: string | null;  // "setCount(c => c + 1)"
}

interface SideEffect {
  type: "console" | "fetch" | "dom" | "storage" | "other";
  description: string;
}
```

### 3.3 Type Extraction

```typescript
interface TypeExtraction {
  name: string;
  kind: "interface" | "type" | "enum";
  exported: boolean;

  // Interface/type fields
  fields: FieldExtraction[];

  // Enum variants
  variants: string[];

  // Generic parameters
  typeParams: string[];

  // Source for reference
  source: string;
}

interface FieldExtraction {
  name: string;
  type: string;
  optional: boolean;
  readonly: boolean;
  defaultValue: string | null;
}
```

---

## 4. Migration Spec Generation

The spec generator enriches the extraction with Qliphoth-specific guidance.

### 4.1 MigrationSpec Schema

```typescript
interface MigrationSpec {
  version: "1.0";
  generatedAt: string;  // ISO timestamp
  projectRoot: string;

  // All components to migrate
  components: ComponentMigrationSpec[];

  // Shared types that need conversion
  types: TypeMigrationSpec[];

  // Migration state
  state: MigrationState;
}

interface ComponentMigrationSpec {
  id: string;  // unique identifier
  name: string;

  // Original source (included for context - no file hopping needed)
  source: {
    path: string;
    code: string;
    extraction: ComponentExtraction;
  };

  // Target information
  target: {
    suggestedPath: string;  // e.g., "src/components/counter.sigil"
    pattern: "actor" | "function";  // actors for stateful, functions for pure
  };

  // Transformation recommendations
  recommendations: {
    // State mapping: hook variable → Sigil state field
    stateFields: StateFieldRecommendation[];

    // Message definitions for event handlers
    messages: MessageRecommendation[];

    // Effect handling
    effects: EffectRecommendation[];

    // Props → constructor or message
    propsHandling: PropsRecommendation;
  };

  // Pattern examples relevant to this component
  patterns: PatternExample[];

  // Ambiguities requiring agent decision
  ambiguities: Ambiguity[];

  // Dependencies on other components
  dependencies: {
    components: string[];  // other component IDs that must migrate first
    types: string[];  // type IDs needed
  };

  // Complexity estimate
  complexity: "simple" | "moderate" | "complex";
  complexityFactors: string[];

  // Migration status
  status: "pending" | "in_progress" | "completed" | "blocked";
}

interface StateFieldRecommendation {
  fromHook: string;  // "useState:count"
  toField: string;   // "count"
  type: string;      // "i32"
  evidentiality: "!" | "?" | "~";  // known, uncertain, reported
  initialValue: string;
  reasoning: string;
}

interface MessageRecommendation {
  name: string;  // "Increment"
  fromHandler: string;  // "handleIncrement" or "onClick:button"
  payload: string | null;  // "{ amount: i32 }" or null
  stateChanges: string[];  // ["self.count += 1"]
  sideEffects: string[];  // ["update document title"]
}

interface EffectRecommendation {
  fromHook: string;  // "useEffect[count]"
  strategy: "inline" | "message" | "lifecycle" | "remove";
  reasoning: string;
  inlineIn: string | null;  // message name if strategy is "inline"
  lifecycleEvent: string | null;  // "Mount" | "Unmount" if strategy is "lifecycle"
}

interface PropsRecommendation {
  strategy: "constructor" | "message" | "none";
  fields: {
    name: string;
    type: string;
    fromProp: string;
  }[];
}

interface PatternExample {
  name: string;
  description: string;
  react: string;  // React code snippet
  sigil: string;  // Equivalent Sigil code
}

interface Ambiguity {
  id: string;
  category: "effect_placement" | "state_type" | "event_mapping" | "component_structure";
  question: string;
  options: {
    label: string;
    description: string;
    recommended: boolean;
  }[];
  defaultChoice: number;  // index of recommended option
}

interface TypeMigrationSpec {
  id: string;
  name: string;
  source: string;  // original TypeScript
  target: string;  // suggested Sigil
  manualReviewNeeded: boolean;
  notes: string[];
}

interface MigrationState {
  totalComponents: number;
  completed: number;
  inProgress: number;
  blocked: number;
  lastUpdated: string;
}
```

### 4.2 Pattern Library

The spec generator includes relevant patterns from a library:

```typescript
const PATTERN_LIBRARY: PatternExample[] = [
  {
    name: "useState_to_state",
    description: "Convert useState hook to actor state field",
    react: `const [count, setCount] = useState(0);`,
    sigil: `state count: i32! = 0,`
  },
  {
    name: "onClick_to_message",
    description: "Convert onClick handler to message dispatch",
    react: `<button onClick={() => setCount(c => c + 1)}>`,
    sigil: `VNode::button().on_click(Increment)`
  },
  {
    name: "useEffect_mount",
    description: "Convert mount-only useEffect to lifecycle",
    react: `useEffect(() => { init(); }, []);`,
    sigil: `on Mount { self.init(); }`
  },
  {
    name: "useEffect_deps",
    description: "Convert useEffect with deps to inline in message handler",
    react: `useEffect(() => { save(count); }, [count]);`,
    sigil: `// Inline in the message that changes count:\non Increment { self.count += 1; self.save(); }`
  },
  {
    name: "conditional_render",
    description: "Convert conditional JSX to .when()",
    react: `{isVisible && <Modal />}`,
    sigil: `.when(self.is_visible, Modal::render())`
  },
  {
    name: "list_render",
    description: "Convert .map() to explicit loop",
    react: `{items.map(item => <Item key={item.id} item={item} />)}`,
    sigil: `// Build children in a loop:
≔ children: Vec<VNode>! = vec![];
∀ item ∈ self.items {
    children.push(Item::render(item));
}
.children(children)`
  },
  {
    name: "jsx_to_builder",
    description: "Convert JSX element to VNode builder",
    react: `<div className="container" id="main">
  <h1>Title</h1>
  <p>Content</p>
</div>`,
    sigil: `VNode::div()
    .class("container")
    .id("main")
    .child(VNode::h1().text("Title"))
    .child(VNode::p().text("Content"))`
  },
  {
    name: "input_controlled",
    description: "Convert controlled input to message-based",
    react: `<input
  value={text}
  onChange={e => setText(e.target.value)}
/>`,
    sigil: `VNode::input()
    .attr("value", self.text.as_str())
    .on_input(TextChanged)`
  }
];
```

---

## 5. MCP Interface

### 5.1 Tools

```typescript
interface McpTools {
  // List all migrations with status
  list_migrations(): MigrationSummary[];

  // Get full spec for one component
  get_migration(componentId: string): ComponentMigrationSpec;

  // Validate generated Sigil code
  validate_sigil(code: string): ValidationResult;

  // Complete a migration (write file, update status)
  complete_migration(componentId: string, sigilCode: string): CompletionResult;

  // Get pattern examples by name or category
  get_patterns(filter?: PatternFilter): PatternExample[];

  // Resolve an ambiguity
  resolve_ambiguity(componentId: string, ambiguityId: string, choice: number): void;
}

interface MigrationSummary {
  id: string;
  name: string;
  status: "pending" | "in_progress" | "completed" | "blocked";
  complexity: "simple" | "moderate" | "complex";
  blockedBy: string[];  // component IDs this depends on
}

interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

interface ValidationError {
  line: number;
  column: number;
  message: string;
  suggestion: string | null;
}

interface CompletionResult {
  success: boolean;
  outputPath: string;
  nextSuggested: string[];  // component IDs to migrate next
}
```

### 5.2 Resources

```typescript
interface McpResources {
  // List of all pending migrations
  "migrations://pending": MigrationSummary[];

  // Pattern library
  "migrations://patterns": PatternExample[];

  // Individual component spec
  "migrations://component/{id}": ComponentMigrationSpec;

  // Project overview
  "migrations://overview": MigrationState;
}
```

---

## 6. CLI Integration

### 6.1 Commands

```bash
# Extract and generate specs for a React project
sigil migrate --from-react ./path/to/react/src --output ./migration-specs

# Options
--include <pattern>    # glob pattern for files to include
--exclude <pattern>    # glob pattern for files to exclude
--force               # overwrite existing specs
--dry-run             # show what would be extracted without writing

# Start MCP server for interactive migration
sigil migrate --from-react --serve ./migration-specs

# Validate a single Sigil file
sigil migrate --validate ./output/counter.sigil

# Show migration status
sigil migrate --status ./migration-specs
```

### 6.2 Output Structure

```
migration-specs/
├── manifest.json           # MigrationSpec root
├── components/
│   ├── counter.json        # ComponentMigrationSpec for Counter
│   ├── todo-list.json
│   └── user-profile.json
├── types/
│   ├── user.json           # TypeMigrationSpec
│   └── todo-item.json
├── patterns/
│   └── library.json        # Pattern examples
└── output/                 # Generated Sigil files
    ├── counter.sigil
    ├── todo_list.sigil
    └── types.sigil
```

---

## 7. Qliphoth Framework Mapping

This section defines how React patterns map to **Qliphoth framework** APIs specifically.

### 7.1 Core Framework Mapping

| React | Qliphoth | Module |
|-------|----------|--------|
| `import React from 'react'` | `invoke qliphoth·prelude·*` | `qliphoth::prelude` |
| `ReactDOM.render(<App/>, root)` | `App·mount("#root", view)` | `qliphoth::core::App` |
| `ReactDOM.hydrate()` | `App·hydrate()` | `qliphoth::core::App` |
| `renderToString()` | `qliphoth·ssr·render_to_string()` | `qliphoth::ssr` |

### 7.2 Component Model Mapping

| React | Qliphoth | Notes |
|-------|----------|-------|
| `function Component()` | `actor Component { }` | Stateful components become actors |
| `const Component = () => {}` | `actor Component { }` | Same - all stateful are actors |
| Pure component (no hooks) | `rite component() -> VNode!` | Stateless can be functions |
| `React.memo()` | Remove | Actors don't need memoization |
| `React.forwardRef()` | Actor with ref state | Handle refs differently |
| `class Component extends React.Component` | `actor Component { }` | Class → Actor |

### 7.3 Hooks → Actor Pattern Mapping

| React Hook | Qliphoth Pattern | Location |
|------------|------------------|----------|
| `useState(initial)` | `state field: Type! = initial,` | Actor state field |
| `useEffect(fn, [])` | `on Mount { }` | Lifecycle message |
| `useEffect(fn, [deps])` | Inline in message handlers | See below |
| `useEffect(() => cleanup)` | `on Unmount { }` | Cleanup message |
| `useLayoutEffect` | `on BeforeRender { }` | Pre-render message |
| `useCallback(fn, deps)` | Remove | Not needed in actors |
| `useMemo(fn, deps)` | Computed or inline | Evaluate in place |
| `useRef(init)` | `state ref: Type! = init,` | Non-reactive state |
| `useContext(Ctx)` | Message passing / shared actor | See routing |
| `useReducer(r, init)` | Native actor pattern! | Actions = Messages |

### 7.4 Event Handling Mapping

| React | Qliphoth |
|-------|----------|
| `onClick={() => setState(...)}` | `·on_click(MessageName)` + `on MessageName { }` |
| `onClick={handleClick}` | `·on_click(MessageName)` + `on MessageName { }` |
| `onChange={e => ...}` | `·on_change(MessageName)` + handler |
| `onSubmit={handleSubmit}` | `·on_submit(MessageName)` + handler |
| `onInput={...}` | `·on_input(MessageName)` + handler |
| Event object `e.target.value` | Message payload extraction |
| `e.preventDefault()` | Handled in message handler |

### 7.5 JSX → VNode Builder Mapping

| JSX | Qliphoth VNode |
|-----|----------------|
| `<div>` | `VNode·div()` |
| `<span>` | `VNode·span()` |
| `<button>` | `VNode·button()` |
| `<input>` | `VNode·input()` |
| `<form>` | `VNode·form()` |
| `<Component />` | `Component·view()` or `Component·new()·view()` |
| `<>...</>` (fragment) | `VNode·fragment()` |
| `className="x"` | `·class("x")` |
| `id="y"` | `·id("y")` |
| `style={{color: "red"}}` | `·style("color", "red")` |
| `href="/path"` | `·attr("href", "/path")` |
| `{children}` | `·child(child_vnode)` or `·children(vec)` |
| `"text content"` | `·text_child("text content")` |
| `{expression}` | `·text_child(expression·to_string())` |

### 7.6 Conditional Rendering Mapping

| React | Qliphoth |
|-------|----------|
| `{cond && <X/>}` | `·when(cond, X·view())` |
| `{cond ? <A/> : <B/>}` | `·child(⎇ cond { A·view() } ⎉ { B·view() })` |
| `{opt && <X data={opt}/>}` | `·when_some(opt·map(\|v\| X·view(v)))` |

### 7.7 List Rendering Mapping

React:
```tsx
{items.map(item => <Item key={item.id} data={item} />)}
```

Qliphoth:
```sigil
// Build children in a loop (explicit, no closures)
≔ Δ children! = Vec·new();
∀ item ∈ self.items {
    children·push(Item·view(item)·key(item.id·to_string()))
}
parent·children(children)
```

### 7.8 Browser API Mapping (qliphoth-sys)

| React/DOM | Qliphoth (qliphoth-sys) |
|-----------|-------------------------|
| `document.title = x` | `Platform·set_title(x)` |
| `document.getElementById(id)` | `Document·get_element_by_id(id)` |
| `window.location.href` | `Location·href()` |
| `localStorage.getItem(k)` | `Storage·local()·get(k)` |
| `fetch(url)` | `Fetch·get(url)` |
| `console.log(x)` | `Console·log(x)` |
| `setTimeout(fn, ms)` | `Timers·set_timeout(msg_id, ms)` |
| `history.pushState()` | `History·push_state()` |

### 7.9 Routing Mapping (qliphoth-router)

| React Router | Qliphoth Router |
|--------------|-----------------|
| `<BrowserRouter>` | Router actor initialization |
| `<Routes>` | `Router·routes()` |
| `<Route path="/" element={<Home/>}/>` | `Route·new("/", Home·view)` |
| `<Link to="/about">` | `Link·new("/about", "text")·view()` |
| `useParams()` | `Params·get()` in message handler |
| `useNavigate()` | `Router·send(Navigate { to: path })` |
| `useLocation()` | `Location·current()` |

---

## 8. Transformation Rules

### 8.1 Type Mapping

| TypeScript | Sigil | Notes |
|------------|-------|-------|
| `string` | `String` | |
| `number` | `f64` | or `i32` if always integer |
| `boolean` | `bool` | |
| `null` | `∅` | Option::None |
| `undefined` | `∅` | Option::None |
| `T \| null` | `Option<T>` | |
| `T[]` | `Vec<T>` | |
| `Record<K, V>` | `HashMap<K, V>` | |
| `Promise<T>` | `Future<T>` | async context |
| `React.FC<P>` | actor or function | depends on hooks |
| `React.ReactNode` | `VNode` | |

### 8.2 Hook Mapping

| React Hook | Qliphoth Pattern |
|------------|------------------|
| `useState(init)` | `state field: Type! = init,` |
| `useEffect(fn, [])` | `on Mount { ... }` |
| `useEffect(fn, [deps])` | Inline in message handlers that change deps |
| `useEffect(fn)` | `on Mount` + `on Update` (rare, usually a smell) |
| `useCallback(fn, deps)` | Remove (actors don't need memoization) |
| `useMemo(fn, deps)` | Computed field or inline |
| `useRef(init)` | `state ref: Type! = init,` (non-reactive) |
| `useContext(Ctx)` | Message passing or shared state actor |
| `useReducer(r, init)` | Actor with message handlers (natural fit!) |

### 8.3 JSX Mapping

| JSX | VNode Builder |
|-----|---------------|
| `<div>` | `VNode::div()` |
| `<Component />` | `Component::render()` |
| `className="x"` | `.class("x")` |
| `id="y"` | `.id("y")` |
| `style={{color: "red"}}` | `.style("color", "red")` |
| `onClick={fn}` | `.on_click(MessageName)` |
| `{expression}` | `.text(expression.to_string())` |
| `{cond && <X/>}` | `.when(cond, X::render())` |
| `{cond ? <A/> : <B/>}` | `.child(if cond { A::render() } else { B::render() })` |
| `<><A/><B/></>` | `VNode::fragment().child(A).child(B)` |
| `{items.map(...)}` | Loop building `Vec<VNode>` |

---

## 9. Example Migration

### 9.1 React Source

```tsx
// Counter.tsx
import { useState, useEffect } from 'react';

interface CounterProps {
  initialCount?: number;
  onCountChange?: (count: number) => void;
}

export function Counter({ initialCount = 0, onCountChange }: CounterProps) {
  const [count, setCount] = useState(initialCount);

  useEffect(() => {
    document.title = `Count: ${count}`;
    onCountChange?.(count);
  }, [count, onCountChange]);

  const increment = () => setCount(c => c + 1);
  const decrement = () => setCount(c => c - 1);
  const reset = () => setCount(initialCount);

  return (
    <div className="counter">
      <span className="count">{count}</span>
      <div className="buttons">
        <button onClick={decrement}>-</button>
        <button onClick={increment}>+</button>
        <button onClick={reset}>Reset</button>
      </div>
    </div>
  );
}
```

### 9.2 Generated Spec (simplified)

```json
{
  "id": "counter",
  "name": "Counter",
  "source": {
    "path": "src/components/Counter.tsx",
    "code": "... full source ...",
    "extraction": { "... parsed structure ..." }
  },
  "target": {
    "suggestedPath": "src/components/counter.sigil",
    "pattern": "actor"
  },
  "recommendations": {
    "stateFields": [
      {
        "fromHook": "useState:count",
        "toField": "count",
        "type": "i32",
        "evidentiality": "!",
        "initialValue": "initial_count",
        "reasoning": "Integer counter, locally computed"
      }
    ],
    "messages": [
      {
        "name": "Increment",
        "fromHandler": "increment",
        "payload": null,
        "stateChanges": ["self.count += 1"],
        "sideEffects": ["update title", "notify parent"]
      },
      {
        "name": "Decrement",
        "fromHandler": "decrement",
        "payload": null,
        "stateChanges": ["self.count -= 1"],
        "sideEffects": ["update title", "notify parent"]
      },
      {
        "name": "Reset",
        "fromHandler": "reset",
        "payload": null,
        "stateChanges": ["self.count = self.initial_count"],
        "sideEffects": ["update title", "notify parent"]
      }
    ],
    "effects": [
      {
        "fromHook": "useEffect[count,onCountChange]",
        "strategy": "inline",
        "reasoning": "Side effects directly tied to state changes, inline in each message handler",
        "inlineIn": "all message handlers"
      }
    ],
    "propsHandling": {
      "strategy": "constructor",
      "fields": [
        { "name": "initial_count", "type": "i32", "fromProp": "initialCount" }
      ]
    }
  },
  "patterns": [
    { "name": "useState_to_state", "..." },
    { "name": "onClick_to_message", "..." }
  ],
  "ambiguities": [
    {
      "id": "callback-prop",
      "category": "event_mapping",
      "question": "The onCountChange callback notifies a parent. How should this be handled?",
      "options": [
        { "label": "Parent message", "description": "Send message to parent actor", "recommended": true },
        { "label": "Event emission", "description": "Emit event that parent subscribes to", "recommended": false },
        { "label": "Remove", "description": "Props callbacks don't fit actor model, remove", "recommended": false }
      ],
      "defaultChoice": 0
    }
  ],
  "complexity": "simple",
  "status": "pending"
}
```

### 9.3 Expected Qliphoth Output

```sigil
// counter.sigil
// Qliphoth actor component - migrated from React functional component

invoke qliphoth·prelude·*;

// Message types for this actor
ᛈ CounterMsg {
    Increment,
    Decrement,
    Reset,
}

☉ actor Counter {
    state count: i64! = 0,
    state initial_count: i64! = 0,

    rite new(initial_count: i64) -> This! {
        Counter {
            count: initial_count,
            initial_count: initial_count,
        }
    }

    // Message handlers (replaces onClick callbacks)
    on Increment {
        self.count += 1;
        self.after_count_change();
    }

    on Decrement {
        self.count -= 1;
        self.after_count_change();
    }

    on Reset {
        self.count = self.initial_count;
        self.after_count_change();
    }

    // Extracted from useEffect - called after state changes
    rite after_count_change(self) {
        // qliphoth-sys platform call (replaces document.title = ...)
        Platform·set_title(format!("Count: {}", self.count));
    }

    // View method using VNode builder API (replaces JSX return)
    rite view(self) -> VNode! {
        VNode·div()
            ·class("counter")
            ·child(
                VNode·span()
                    ·class("count")
                    ·text_child(self.count·to_string())
            )
            ·child(
                VNode·div()
                    ·class("buttons")
                    ·child(VNode·button()·text_child("-")·on_click(Decrement))
                    ·child(VNode·button()·text_child("+")·on_click(Increment))
                    ·child(VNode·button()·text_child("Reset")·on_click(Reset))
            )
    }
}

// Entry point (replaces ReactDOM.render)
rite main() {
    App·mount("#root", Counter·new(0)·view())
}
```

**Key Qliphoth patterns in output:**
1. `invoke qliphoth·prelude·*` - framework imports
2. Message enum `ᛈ CounterMsg` - typed messages for actor
3. `actor Counter` with `state` fields - not a function component
4. `on Message { }` handlers - not event callbacks
5. `VNode·div()` builder chain - not JSX
6. `·on_click(MessageName)` - dispatch to actor, not callback
7. `Platform·set_title()` - qliphoth-sys, not raw DOM
8. `App·mount()` - Qliphoth app entry point

---

## 10. Implementation Roadmap

### Phase 1: Core Extraction (Week 1-2)
- [ ] Add swc dependencies to sigil-lang parser
- [ ] Implement React/TSX file detection
- [ ] Build extraction for functional components
- [ ] Extract hooks (useState, useEffect, useCallback, useMemo, useRef)
- [ ] Extract JSX tree structure
- [ ] Extract TypeScript types/interfaces
- [ ] Output ReactExtraction JSON

### Phase 2: Spec Generation (Week 2-3)
- [ ] Build pattern library
- [ ] Implement recommendation engine
- [ ] Generate state field recommendations
- [ ] Generate message recommendations
- [ ] Generate effect handling recommendations
- [ ] Detect and flag ambiguities
- [ ] Calculate complexity scores
- [ ] Output MigrationSpec JSON

### Phase 3: MCP Server (Week 3-4)
- [ ] Implement MCP server in sigil-lang
- [ ] Add list_migrations tool
- [ ] Add get_migration tool
- [ ] Add validate_sigil tool
- [ ] Add complete_migration tool
- [ ] Add get_patterns tool
- [ ] Implement resource endpoints

### Phase 4: CLI Integration (Week 4)
- [ ] Add `--from-react` flag to `sigil migrate`
- [ ] Implement `--serve` for MCP mode
- [ ] Add `--status` for migration overview
- [ ] Add `--validate` for single file validation
- [ ] Documentation and help text

### Phase 5: Testing & Refinement
- [ ] Test against real React codebases
- [ ] Refine pattern library based on common patterns
- [ ] Improve ambiguity detection
- [ ] Performance optimization for large codebases

---

## 11. Future Extensions

### 11.1 Vue.js Support

The extraction layer is designed to be pluggable:

```rust
trait FrameworkExtractor {
    fn detect(path: &Path) -> bool;
    fn extract(source: &str) -> FrameworkExtraction;
}

impl FrameworkExtractor for ReactExtractor { ... }
impl FrameworkExtractor for VueExtractor { ... }  // Future
impl FrameworkExtractor for SvelteExtractor { ... }  // Future
```

Vue-specific considerations:
- Single-file components (.vue) with `<template>`, `<script>`, `<style>`
- Options API vs Composition API
- `ref()`, `reactive()`, `computed()` → Sigil state
- `v-if`, `v-for`, `v-bind` → VNode builder equivalents

### 11.2 Batch Mode

For large codebases, support batch processing:

```bash
sigil migrate --from-react ./src --batch --parallel 4
```

Agent can request batch specs and submit batch completions.

### 11.3 Incremental Migration

Support migrating a codebase incrementally:
- Mark components as "boundary" (keep React, wrap for Qliphoth)
- Generate interop wrappers
- Track migration progress over time

---

## 12. Open Questions

1. **Custom hooks**: How deep do we go? Extract the hook itself or just its usage?

2. **Third-party components**: How to handle `<MaterialUI.Button>`? Suggest Qliphoth equivalent or generate wrapper?

3. **CSS-in-JS**: Styled-components, Emotion, etc. - how do these map to Qliphoth styling?

4. **Server components**: React Server Components have different semantics. Scope for Phase 2?

5. **Testing**: Should we also migrate React tests to Sigil tests? What's the mapping?

---

## Appendix A: Full Schema (JSON Schema format)

*To be generated from TypeScript interfaces above*

## Appendix B: Pattern Library (Complete)

*To be expanded as we encounter more patterns in real migrations*
