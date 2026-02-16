# Agent-Centric Qliphoth Specification

**Version:** 0.1.0
**Status:** Draft
**Date:** 2026-02-14
**Supersedes:** All prior React-inspired designs

---

## Abstract

This specification defines a complete redesign of Qliphoth around agent-centric
principles. The framework abandons React patterns (hooks, callbacks, JSX-style DSL)
in favor of Sigil's native paradigms: actors for components, messages for events,
evidentiality for data states, and explicit builder APIs for view construction.

The guiding principle: **agents generate code, they don't type it.** Every design
decision optimizes for unambiguous, explicit patterns that agents can reliably
produce and reason about.

---

## 1. Motivation

### 1.1 Why Not React Patterns?

React's design solves JavaScript's problems:

| React Pattern | JavaScript Problem | Sigil Reality |
|---------------|-------------------|---------------|
| Hooks | No classes in early React, workaround for state | Sigil has actors |
| Callbacks | JS is callback-native | Sigil has message passing |
| JSX/DSL | Reduce typing for humans | Agents don't type |
| Implicit re-render | Hide complexity from humans | Agents handle explicit |
| Context | Prop drilling painful for humans | Actors can receive deps |
| useEffect cleanup | Closures capture state | Actors have lifecycle |

### 1.2 Agent-Centric Principles

1. **Explicit over implicit** - No magic re-renders, no hidden state
2. **Messages over callbacks** - Typed, traceable, debuggable
3. **Actors over functions** - Natural state encapsulation
4. **Builders over DSL** - Unambiguous, no parser tricks
5. **Evidentiality over booleans** - `?` means loading, `!` means known

### 1.3 Benefits

- **Simpler parser** - No DSL, no function types, no lifetime bounds
- **Simpler runtime** - Actor scheduler already exists conceptually
- **Better debugging** - Message traces show exactly what happened
- **Agent-friendly** - Unambiguous patterns, no context-dependent syntax

---

## 2. Component Model

### 2.1 Components Are Actors

Every UI component is an actor with:
- **State fields** - Local component state
- **Message handlers** - Event responses and state transitions
- **View method** - Returns VNode tree for rendering

```sigil
actor Counter {
    state count: i64! = 0

    on Increment { self.count += 1 }
    on Decrement { self.count -= 1 }
    on Reset { self.count = 0 }

    rite view(self) -> VNode! {
        VNode::div()
            .class("counter")
            .child(VNode::button().on_click(Decrement).text("-"))
            .child(VNode::span().text(self.count.to_string()))
            .child(VNode::button().on_click(Increment).text("+"))
    }
}
```

### 2.2 Component Lifecycle

Actors receive lifecycle messages automatically:

```sigil
actor UserDashboard {
    state data: DashboardData? = None

    /// Called when component mounts to DOM
    on Mount {
        self·send(LoadData {})
    }

    /// Called when component unmounts from DOM
    on Unmount {
        // Cleanup subscriptions, timers, etc.
    }

    /// Called when props change (if applicable)
    on PropsChanged(new_props: Props) {
        // React to prop changes
    }

    on LoadData {
        fetch("/api/dashboard")
            |then{data => self·send(DataLoaded { data })}
            |catch{err => self·send(DataFailed { err })}
    }

    on DataLoaded(data: DashboardData) {
        self.data = Some(data)!
    }

    rite view(self) -> VNode! {
        ⌥ self.data {
            None => VNode::text("Loading..."),
            Some(data) => self.render_dashboard(data),
        }
    }
}
```

### 2.3 No Hooks Mapping

| React Hook | Qliphoth Actor Pattern |
|------------|----------------------|
| `useState` | `state field: Type = default` |
| `useEffect` | `on Mount`, `on Unmount`, `on PropsChanged` |
| `useReducer` | Message handlers |
| `useContext` | Constructor injection or parent messages |
| `useMemo` | Computed in `view()` or cached state field |
| `useCallback` | Not needed - use message types |
| `useRef` | `state ref: Option<ElementRef> = None` |

---

## 3. View Construction

### 3.1 Builder Pattern

Views are constructed with explicit builder methods, not DSL:

```sigil
rite view(self) -> VNode! {
    VNode::div()
        .id("main")
        .class("container")
        .attr("data-theme", "dark")
        .child(VNode::h1().text("Welcome"))
        .child(VNode::p().text("Hello, world!"))
}
```

### 3.2 VNode API

```sigil
☉ Σ VNode {
    // Internal representation
}

⊢ VNode {
    // Element constructors
    ☉ rite div() -> Self!
    ☉ rite span() -> Self!
    ☉ rite button() -> Self!
    ☉ rite input() -> Self!
    ☉ rite a() -> Self!
    ☉ rite img() -> Self!
    ☉ rite form() -> Self!
    // ... all HTML elements

    // Text node
    ☉ rite text(content: &str) -> Self!

    // Fragment (multiple children, no wrapper)
    ☉ rite fragment() -> Self!

    // Attributes
    rite id(self, value: &str) -> Self!
    rite class(self, value: &str) -> Self!
    rite attr(self, name: &str, value: &str) -> Self!
    rite style(self, property: &str, value: &str) -> Self!

    // Children
    rite child(self, node: VNode) -> Self!
    rite children(self, nodes: Vec<VNode>) -> Self!
    rite text(self, content: &str) -> Self!

    // Events - take message types, not callbacks
    rite on_click<M: Message>(self, msg: M) -> Self!
    rite on_input<M: Message>(self, handler: rite(&str) -> M) -> Self!
    rite on_submit<M: Message>(self, msg: M) -> Self!
    rite on_change<M: Message>(self, handler: rite(&str) -> M) -> Self!

    // Conditionals
    rite when(self, condition: bool, node: VNode) -> Self!
    rite when_some<T>(self, opt: Option<T>, f: rite(T) -> VNode) -> Self!

    // Iteration
    rite each<T, K: Hash>(self, items: &[T], key: rite(&T) -> K, render: rite(&T) -> VNode) -> Self!

    // Component embedding
    rite component<A: Actor>(self, actor: A) -> Self!
}
```

### 3.3 Event Handling

Events dispatch messages to the component actor:

```sigil
actor LoginForm {
    state username: String! = ""
    state password: String! = ""
    state submitting: bool! = false

    on UsernameChanged(value: String) {
        self.username = value
    }

    on PasswordChanged(value: String) {
        self.password = value
    }

    on Submit {
        self.submitting = true
        auth·login(self.username, self.password)
            |then{token => self·send(LoginSuccess { token })}
            |catch{err => self·send(LoginFailed { err })}
    }

    on LoginSuccess(token: String) {
        self.submitting = false
        router·navigate("/dashboard")
    }

    on LoginFailed(err: Error) {
        self.submitting = false
        // Show error
    }

    rite view(self) -> VNode! {
        VNode::form()
            .on_submit(Submit)
            .child(VNode::input()
                .attr("type", "text")
                .attr("placeholder", "Username")
                .attr("value", &self.username)
                .on_input(|v| UsernameChanged { value: v.to_string() }))
            .child(VNode::input()
                .attr("type", "password")
                .attr("placeholder", "Password")
                .attr("value", &self.password)
                .on_input(|v| PasswordChanged { value: v.to_string() }))
            .child(VNode::button()
                .attr("type", "submit")
                .attr("disabled", self.submitting.to_string())
                .text(⎇ self.submitting { "Logging in..." } ⎉ { "Login" }))
    }
}
```

---

## 4. Evidentiality Integration

### 4.1 Data States as Evidentiality

Sigil's evidentiality markers map perfectly to UI data states:

| Marker | Meaning | UI Interpretation |
|--------|---------|-------------------|
| `!` | Known/Computed | Data is available, render it |
| `?` | Uncertain | Data is loading or may fail |
| `~` | Reported | Data from external source, may be stale |
| `‽` | Untrusted | User input, needs validation |

### 4.2 Loading States

```sigil
actor UserProfile {
    state user: User? = None          // Loading
    state posts: Vec<Post>? = None    // Loading

    on Mount {
        self·send(LoadUser {})
        self·send(LoadPosts {})
    }

    on UserLoaded(user: User) {
        self.user = Some(user)!       // Now certain
    }

    on PostsLoaded(posts: Vec<Post>) {
        self.posts = Some(posts)!     // Now certain
    }

    rite view(self) -> VNode! {
        VNode::div()
            .child(self.render_user())
            .child(self.render_posts())
    }

    rite render_user(self) -> VNode! {
        ⌥ self.user {
            None => VNode::div().class("skeleton"),
            Some(user) => VNode::div()
                .class("user-header")
                .child(VNode::img().attr("src", &user.avatar))
                .child(VNode::h1().text(&user.name)),
        }
    }

    rite render_posts(self) -> VNode! {
        ⌥ self.posts {
            None => VNode::div().class("loading").text("Loading posts..."),
            Some(posts) => VNode::div()
                .class("posts")
                .each(posts, |p| p.id, |post| {
                    VNode::article()
                        .child(VNode::h2().text(&post.title))
                        .child(VNode::p().text(&post.body))
                }),
        }
    }
}
```

### 4.3 Error States

```sigil
actor DataView {
    state data: Result<Data, Error>? = None

    on Mount {
        fetch_data()
            |then{d => self·send(Success { data: d })}
            |catch{e => self·send(Failure { error: e })}
    }

    on Success(data: Data) {
        self.data = Some(Ok(data))!
    }

    on Failure(error: Error) {
        self.data = Some(Err(error))!
    }

    rite view(self) -> VNode! {
        ⌥ self.data {
            None => VNode::text("Loading..."),
            Some(Ok(data)) => self.render_data(data),
            Some(Err(err)) => VNode::div()
                .class("error")
                .child(VNode::text(&format!("Error: {}", err)))
                .child(VNode::button()
                    .on_click(Mount)  // Retry
                    .text("Retry")),
        }
    }
}
```

### 4.4 User Input (Untrusted)

```sigil
actor CommentForm {
    state content: String‽ = ""       // Untrusted user input
    state validated: bool! = false

    on ContentChanged(value: String) {
        self.content = value‽         // Mark as untrusted
        self.validated = self.validate_content()
    }

    on Submit {
        ⎇ self.validated {
            // Safe to use - we validated
            ≔ sanitized! = sanitize(self.content)
            api·post_comment(sanitized)
        } ⎉ {
            // Show validation error
        }
    }

    rite validate_content(self) -> bool! {
        self.content.len() > 0 && self.content.len() < 1000
    }
}
```

---

## 5. Composition

### 5.1 Nested Components

Components embed other components:

```sigil
actor App {
    rite view(self) -> VNode! {
        VNode::div()
            .class("app")
            .component(Header::new())
            .component(Sidebar::new())
            .child(VNode::main()
                .component(Router::new()))
            .component(Footer::new())
    }
}
```

### 5.2 Props via Constructor

Pass data to child components via constructors:

```sigil
actor UserCard {
    state user: User!
    state show_details: bool!

    rite new(user: User, show_details: bool) -> Self! {
        UserCard { user, show_details }
    }

    rite view(self) -> VNode! {
        VNode::div()
            .class("user-card")
            .child(VNode::img().attr("src", &self.user.avatar))
            .child(VNode::h3().text(&self.user.name))
            .when(self.show_details,
                VNode::p().text(&self.user.bio))
    }
}

// Usage
actor UserList {
    state users: Vec<User>! = vec![]

    rite view(self) -> VNode! {
        VNode::div()
            .class("user-list")
            .each(&self.users, |u| u.id, |user| {
                VNode::fragment()
                    .component(UserCard::new(user.clone(), true))
            })
    }
}
```

### 5.3 Child-to-Parent Communication

Children send messages to parents via callbacks or shared actors:

```sigil
// Option 1: Callback in constructor
actor TodoItem {
    state todo: Todo!
    state on_delete: rite(u64)!

    rite new(todo: Todo, on_delete: rite(u64)) -> Self! {
        TodoItem { todo, on_delete }
    }

    on Delete {
        (self.on_delete)(self.todo.id)
    }
}

// Option 2: Shared state actor
actor TodoApp {
    state store: TodoStore!

    rite view(self) -> VNode! {
        VNode::div()
            .each(&self.store.todos, |t| t.id, |todo| {
                VNode::fragment()
                    .component(TodoItem::new(todo.clone(), self.store))
            })
    }
}
```

---

## 6. Routing

### 6.1 Router Actor

```sigil
actor Router {
    state current_path: String! = "/"
    state params: Map<String, String>! = Map::new()

    on Mount {
        // Listen to browser navigation
        window·on_popstate(|path| self·send(Navigate { path }))
        self.current_path = window·location·pathname()
    }

    on Navigate(path: String) {
        self.current_path = path
        window·history·push_state(path)
    }

    on NavigateReplace(path: String) {
        self.current_path = path
        window·history·replace_state(path)
    }

    rite view(self) -> VNode! {
        ⌥ self.match_route() {
            Route::Home => VNode::fragment().component(HomePage::new()),
            Route::User(id) => VNode::fragment().component(UserPage::new(id)),
            Route::Settings => VNode::fragment().component(SettingsPage::new()),
            Route::NotFound => VNode::text("404 Not Found"),
        }
    }

    rite match_route(self) -> Route! {
        ⌥ self.current_path.as_str() {
            "/" => Route::Home,
            path ⎇ path.starts_with("/user/") => {
                ≔ id! = path[6..].parse::<u64>().ok()
                ⌥ id {
                    Some(id) => Route::User(id),
                    None => Route::NotFound,
                }
            }
            "/settings" => Route::Settings,
            _ => Route::NotFound,
        }
    }
}

ᛈ Route {
    Home,
    User(u64),
    Settings,
    NotFound,
}
```

### 6.2 Navigation Links

```sigil
// Link component that navigates on click
actor Link {
    state href: String!
    state children: VNode!

    rite new(href: &str, children: VNode) -> Self! {
        Link { href: href.to_string(), children }
    }

    on Click {
        router·send(Navigate { path: self.href.clone() })
    }

    rite view(self) -> VNode! {
        VNode::a()
            .attr("href", &self.href)
            .on_click(Click)
            .child(self.children.clone())
    }
}
```

---

## 7. State Management

### 7.1 Local State

Actor state fields handle local component state:

```sigil
actor ToggleButton {
    state active: bool! = false

    on Toggle { self.active = !self.active }

    rite view(self) -> VNode! {
        VNode::button()
            .class(⎇ self.active { "active" } ⎉ { "inactive" })
            .on_click(Toggle)
            .text(⎇ self.active { "ON" } ⎉ { "OFF" })
    }
}
```

### 7.2 Shared State Actor

For global/shared state, use a dedicated state actor:

```sigil
actor AppState {
    state user: Option<User>? = None
    state theme: Theme! = Theme::Dark
    state notifications: Vec<Notification>! = vec![]

    on Login(user: User) {
        self.user = Some(user)!
    }

    on Logout {
        self.user = None?
    }

    on SetTheme(theme: Theme) {
        self.theme = theme
    }

    on AddNotification(notif: Notification) {
        self.notifications.push(notif)
    }

    on DismissNotification(id: u64) {
        self.notifications.retain(|n| n.id != id)
    }
}

// Components receive AppState reference
actor Header {
    state app: &AppState!

    rite new(app: &AppState) -> Self! {
        Header { app }
    }

    rite view(self) -> VNode! {
        VNode::header()
            .child(⌥ self.app.user {
                None => VNode::fragment().component(LoginButton::new()),
                Some(user) => VNode::span().text(&user.name),
            })
    }
}
```

---

## 8. Rendering Pipeline

### 8.1 Render Cycle

1. Actor receives message
2. Message handler updates state
3. Runtime calls `view()` method
4. Runtime diffs new VNode tree against previous
5. Runtime applies minimal DOM updates

```
Message → Handler → State Change → view() → Diff → DOM Patch
```

### 8.2 Batching

Multiple messages in the same frame are batched:

```sigil
// These three messages result in ONE re-render
self·send(UpdateA {})
self·send(UpdateB {})
self·send(UpdateC {})
// view() called once after all handlers complete
```

### 8.3 Scheduling

Updates are scheduled with priorities:

```sigil
ᛈ Priority {
    Immediate,    // User input, must respond now
    High,         // Animations, transitions
    Normal,       // Data updates
    Low,          // Background sync
    Idle,         // When browser is idle
}

// Runtime schedules based on message source
// on_click → Immediate
// on Mount → Normal
// background fetch → Low
```

---

## 9. Platform Abstraction

### 9.1 Platform Trait

```sigil
trait Platform {
    rite query_selector(&self, selector: &str) -> Option<ElementRef>?
    rite create_element(&self, tag: &str) -> ElementRef!
    rite create_text(&self, content: &str) -> ElementRef!
    rite append_child(&self, parent: &ElementRef, child: &ElementRef)
    rite remove_child(&self, parent: &ElementRef, child: &ElementRef)
    rite set_attribute(&self, el: &ElementRef, name: &str, value: &str)
    rite add_event_listener(&self, el: &ElementRef, event: &str, handler: u32)
    rite request_animation_frame(&self, callback: rite())
}
```

### 9.2 Browser Platform

```sigil
Σ BrowserPlatform {}

⊢ Platform for BrowserPlatform {
    // Implemented via web-sys / wasm-bindgen interop
}
```

### 9.3 Server Platform (SSR)

```sigil
Σ ServerPlatform {
    output: String!
}

⊢ Platform for ServerPlatform {
    // Builds HTML string instead of DOM operations
}
```

---

## 10. Migration Plan

### 10.1 Files to Delete

```
src/hooks/           # All of it - hooks are gone
src/core/scheduler.sigil   # Replace with actor scheduler
src/core/reconciler.sigil  # Simplify for actor model
```

### 10.2 Files to Rewrite

```
src/core/mod.sigil      # Actor-based runtime
src/core/vdom.sigil     # Simplified VNode with builders
src/components/         # Convert to actors
src/router/             # Actor-based router
src/state/              # Actor-based state management
```

### 10.3 Files to Keep (modified)

```
src/lib.sigil           # Update exports
src/dom/                # DOM abstraction stays
src/platform/           # Platform abstraction stays
```

### 10.4 New Files

```
src/runtime/mod.sigil       # Actor runtime for UI
src/runtime/scheduler.sigil # Message scheduling
src/runtime/reconciler.sigil # VNode diffing
```

---

## 11. Example: Complete Application

```sigil
// main.sigil
invoke qliphoth·prelude·*

actor TodoApp {
    state todos: Vec<Todo>! = vec![]
    state filter: Filter! = Filter::All
    state new_todo_text: String! = ""

    on Mount {
        // Load from localStorage
        ≔ saved? = storage·get("todos")
        ⌥ saved {
            Some(data) => self.todos = parse_todos(data)!,
            None => {}
        }
    }

    on NewTodoTextChanged(text: String) {
        self.new_todo_text = text
    }

    on AddTodo {
        ⎇ !self.new_todo_text.is_empty() {
            self.todos.push(Todo {
                id: next_id(),
                text: self.new_todo_text.clone(),
                completed: false,
            })
            self.new_todo_text = ""
            self·send(SaveTodos {})
        }
    }

    on ToggleTodo(id: u64) {
        ∀ todo ∈ &Δ self.todos {
            ⎇ todo.id == id {
                todo.completed = !todo.completed
            }
        }
        self·send(SaveTodos {})
    }

    on DeleteTodo(id: u64) {
        self.todos.retain(|t| t.id != id)
        self·send(SaveTodos {})
    }

    on SetFilter(filter: Filter) {
        self.filter = filter
    }

    on SaveTodos {
        storage·set("todos", serialize_todos(&self.todos))
    }

    rite view(self) -> VNode! {
        VNode::div()
            .class("todo-app")
            .child(self.render_header())
            .child(self.render_list())
            .child(self.render_footer())
    }

    rite render_header(self) -> VNode! {
        VNode::header()
            .child(VNode::h1().text("todos"))
            .child(VNode::form()
                .on_submit(AddTodo)
                .child(VNode::input()
                    .class("new-todo")
                    .attr("placeholder", "What needs to be done?")
                    .attr("value", &self.new_todo_text)
                    .on_input(|v| NewTodoTextChanged { text: v.to_string() })))
    }

    rite render_list(self) -> VNode! {
        ≔ filtered! = self.filtered_todos()
        VNode::ul()
            .class("todo-list")
            .each(&filtered, |t| t.id, |todo| {
                VNode::li()
                    .class(⎇ todo.completed { "completed" } ⎉ { "" })
                    .child(VNode::input()
                        .attr("type", "checkbox")
                        .attr("checked", todo.completed.to_string())
                        .on_click(ToggleTodo { id: todo.id }))
                    .child(VNode::span().text(&todo.text))
                    .child(VNode::button()
                        .class("delete")
                        .on_click(DeleteTodo { id: todo.id })
                        .text("×"))
            })
    }

    rite render_footer(self) -> VNode! {
        ≔ remaining! = self.todos.iter().filter(|t| !t.completed).count()
        VNode::footer()
            .child(VNode::span()
                .text(&format!("{} items left", remaining)))
            .child(VNode::div()
                .class("filters")
                .child(self.filter_button(Filter::All, "All"))
                .child(self.filter_button(Filter::Active, "Active"))
                .child(self.filter_button(Filter::Completed, "Completed")))
    }

    rite filter_button(self, filter: Filter, label: &str) -> VNode! {
        VNode::button()
            .class(⎇ self.filter == filter { "selected" } ⎉ { "" })
            .on_click(SetFilter { filter })
            .text(label)
    }

    rite filtered_todos(self) -> Vec<Todo>! {
        ⌥ self.filter {
            Filter::All => self.todos.clone(),
            Filter::Active => self.todos.iter().filter(|t| !t.completed).cloned().collect(),
            Filter::Completed => self.todos.iter().filter(|t| t.completed).cloned().collect(),
        }
    }
}

Σ Todo {
    id: u64!,
    text: String!,
    completed: bool!,
}

ᛈ Filter {
    All,
    Active,
    Completed,
}

rite main() {
    App::mount("#root", TodoApp::new())
}
```

---

## 12. Constraints & Invariants

### 12.1 Syntactic Invariants

```
I1: Components are actors with a `view(self) -> VNode!` method
I2: Events dispatch messages, never raw callbacks
I3: State changes only in message handlers, never in view()
I4: VNode construction uses builder methods, no DSL
```

### 12.2 Semantic Invariants

```
I5: Evidentiality markers reflect actual data certainty
I6: Message handlers are synchronous (async via send)
I7: view() is pure - no side effects
I8: Child components receive data via constructor, not context
```

### 12.3 Performance Invariants

```
I9: Multiple messages in one frame batch to single render
I10: VNode diffing is O(n) in tree size
I11: DOM updates are minimal (only changed attributes/children)
```

---

## 13. Implementation Priority

### Phase 1: Core Runtime
1. Actor runtime for UI components
2. VNode builder API
3. Basic reconciler (full replacement, no diff)
4. Browser platform

### Phase 2: Efficient Rendering
1. VNode diffing algorithm
2. Keyed children optimization
3. Batch scheduling
4. Event delegation

### Phase 3: Developer Experience
1. DevTools integration
2. Hot reload support
3. Error boundaries
4. SSR support

---

## 14. Open Questions

1. **Actor identity**: How are component actors identified for reconciliation?
   - Option A: Explicit keys everywhere
   - Option B: Position-based identity (React-style)
   - Option C: Content-addressed identity

2. **Async in handlers**: Should message handlers be async?
   - Current: No, use `self·send()` for async completion
   - Alternative: `async on Fetch { ... }` syntax

3. **CSS**: How to handle component-scoped styles?
   - Option A: Inline styles via builder
   - Option B: CSS modules with build step
   - Option C: CSS-in-Sigil (separate spec)

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-02-14 | Initial draft. Complete redesign from React patterns to actor-centric model. |
