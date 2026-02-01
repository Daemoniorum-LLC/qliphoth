# Qliphoth

A React-inspired web application framework built on Sigil's polysynthetic programming paradigm.

## Overview

Qliphoth leverages Sigil's unique features to create a powerful, type-safe web framework:

- **Evidentiality-Driven State**: Track data provenance (`!` known, `?` uncertain, `◊` predicted, `~` reported, `‽` paradox)
- **Morpheme Components**: Compose UI with pipe operators and Greek letter transformations
- **Actor-Based State Management**: Predictable state updates via message passing
- **Zero-Cost Abstractions**: Compile-time optimization for production builds

## Quick Start

```sigil
use qliphoth::prelude::*

// Define a component
component Counter {
    state count: i64! ≔ 0

    rite render(self) → Element {
        div {
            h1 { "Count: {self·count}" }
            button[onclick: || self·count += 1] { "Increment" }
        }
    }
}

// Mount to DOM
rite main() {
    App·mount("#root", Counter·new())
}
```

## Core Concepts

### Components

Components are the building blocks of Qliphoth applications:

```sigil
// Functional component
rite Greeting(props: {name: String}) → Element {
    h1 { "Hello, {props·name}!" }
}

// Stateful component
component Timer {
    state seconds: i64! ≔ 0

    on Mount {
        interval(1000, || self·seconds += 1)
    }

    rite render(self) → Element {
        span { "Elapsed: {self·seconds}s" }
    }
}
```

### Evidentiality in UI

Sigil's evidentiality system naturally maps to UI data flow:

| Marker | Meaning | UI Context |
|--------|---------|------------|
| `!` | Known | Local state, derived values |
| `?` | Uncertain | Optional props, nullable data |
| `◊` | Predicted | AI/ML outputs, probabilistic data |
| `~` | Reported | API responses, external data |
| `‽` | Paradox | User input, explicit trust boundary |

```sigil
component UserProfile {
    state user: User~ ≔ User·empty()  // Remote data
    state editing: bool! ≔ false       // Local state

    rite render(self) → Element {
        match self·user {
            User·empty() → Spinner {},
            user~ → ProfileCard { user: user~|validate‽ }
        }
    }
}
```

### Pipe-Based Composition

Use Sigil's pipe operators for elegant component composition:

```sigil
rite UserList(users: Vec<User>~) → Element {
    users
        |φ{_·active}           // Filter active users
        |σ{_·name}             // Sort by name
        |τ{user → UserCard { user }}  // Map to components
        |into_fragment         // Collect into fragment
}
```

### Hooks

React-inspired hooks with evidentiality tracking:

```sigil
rite SearchBox() → Element {
    ≔ (query, set_query) ≔ use_state!("")
    ≔ results~ ≔ use_fetch("/api/search?q={query}")
    ≔ debounced? ≔ use_debounce(query, 300)

    div {
        input[value: query, oninput: set_query]
        match results~ {
            Loading → Spinner {},
            Error(e~) → ErrorBanner { message: e~ },
            Data(items~) → ResultList { items: items~ }
        }
    }
}
```

### Routing

Declarative routing with type-safe parameters:

```sigil
use qliphoth::router::*

rite App() → Element {
    Router {
        Route[path: "/"] { Home {} }
        Route[path: "/docs/:section"] { |params|
            Docs { section: params·section }
        }
        Route[path: "/api/:module/:function"] { |params|
            ApiReference {
                module: params·module,
                function: params·function
            }
        }
        Route[path: "*"] { NotFound {} }
    }
}
```

## Cross-Platform Support

Qliphoth runs on multiple platforms from the same codebase:

| Platform | Target | Backend | UI Renderer |
|----------|--------|---------|-------------|
| **Browser** | `wasm32` | LLVM→WASM | DOM via JS FFI |
| **Server** | native | LLVM | HTML strings (SSR) |
| **Desktop** | native | LLVM | GTK4 widgets |

### Build for Different Platforms

```bash
# Web (WebAssembly)
sigil compile --target wasm32-unknown-unknown -o app.wasm

# Server (SSR)
sigil compile -o app-server

# Desktop (GTK4)
sigil compile --features gtk -o app-desktop
```

### Platform-Specific Code

Use `#[cfg(...)]` for platform-specific behavior:

```sigil
component App {
    rite render(self) → Element {
        div {
            h1 { "Cross-Platform App" }

            #[cfg(target_arch = "wasm32")]
            { p { "Running in browser" } }

            #[cfg(feature = "gtk")]
            { p { "Running on desktop" } }
        }
    }
}
```

### Platform Abstraction

The `Platform` trait provides a unified interface:

```sigil
use qliphoth::platform::{Platform, detect}

rite main() {
    // Auto-detect platform
    ≔ platform ≔ detect()

    // Use platform-agnostic APIs
    ≔ (width, height) ≔ platform·window_size()
    platform·set_timeout(|| println!("Hello!"), 1000)
}
```

## Architecture

```
qliphoth/
├── src/
│   ├── core/           # Core runtime and reconciliation
│   ├── components/     # Base component system
│   ├── hooks/          # React-style hooks
│   ├── router/         # Client-side routing
│   ├── dom/            # Virtual DOM implementation
│   ├── state/          # Actor-based state management
│   └── platform/       # Platform bindings (browser, SSR, GTK)
├── docs/               # Framework documentation
├── examples/           # Example applications
└── tests/              # Test suite
```

## Installation

```bash
# Add to your Sigil project
sigil add qliphoth

# Or clone for development
git clone https://github.com/Daemoniorum-LLC/qliphoth
cd qliphoth && sigil build
```

## Documentation

- [Getting Started Guide](docs/guides/getting-started.md)

Additional documentation coming soon:
- Component API
- Hooks Reference
- Router Guide
- State Management

## Examples

- [Counter](examples/counter.sigil) - Simple state management
- [Cross-Platform Counter](examples/counter_cross_platform.sigil) - **Same code runs on Web, Server, and Desktop**
- [Todo App](examples/todo.sigil) - CRUD operations
- [Docs Platform](examples/docs-platform/) - Full documentation site

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Copyright (c) 2025 Daemoniorum, LLC
