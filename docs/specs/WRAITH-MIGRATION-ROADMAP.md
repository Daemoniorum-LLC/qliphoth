# Wraith IDE → Qliphoth Migration Roadmap

**Status:** Active
**Source Project:** `~/dev/wraith/wraith-framework`
**Target Runtime:** Native
**Author:** Claude (Conclave session: wraith-migration-2026-02-16)

---

## Project Overview

Wraith is a browser-based IDE frontend built with React 18 + Vite + TypeScript. We are migrating it to Qliphoth for native execution.

### Source Codebase Stats

| Metric | Count |
|--------|-------|
| React Components (.tsx) | 125 |
| TypeScript Modules (.ts) | 47 |
| Custom Hooks | 3 |
| Test Files | 19 |
| Total Lines | ~15,000 (estimated) |

### Architecture

```
src/
├── auth/                  # OAuth (3 components)
├── contexts/              # Theme context
├── domains/               # Feature modules (DDD)
│   ├── devops/            # 32 components
│   ├── prompts/           # 11 components
│   ├── chat/              # 1 component
│   ├── terminal/          # 1 component
│   └── tasks/             # 1 component
├── ui/components/         # Atomic design
│   ├── atoms/             # Basic building blocks
│   ├── molecules/         # Composite components
│   ├── organisms/         # Complex components
│   └── panels/            # 20+ extension panels
├── wraith/hooks/          # Custom hooks
└── services/              # API services
```

---

## Key Decisions

### 1. Monaco Editor → Athame

**Decision:** Replace Monaco with Qliphoth-native Athame editor.

**Rationale:**
- Native runtime target eliminates browser dependency
- Full control over editor behavior
- Consistent Qliphoth architecture

**Migration Strategy:**
- Map `<CodeEditor>` component references → `Athame·view()`
- Translate props: `value` → `content`, `language` → `syntax_mode`
- Map `onChange` → message dispatch

**Affected Files:**
- `src/ui/components/organisms/CodeEditor.tsx`
- All files importing CodeEditor

### 2. xterm.js → Native Terminal

**Decision:** Replace xterm.js with native terminal (wrap `alacritty_terminal` crate).

**Rationale:**
- Native runtime target
- No JavaScript interop overhead
- Full integration with Qliphoth PTY handling

**Migration Strategy:**
```sigil
☉ actor Terminal {
    state pty: AlacrittyTerminal!,
    state scrollback: TerminalScrollback!,

    on Input { data } {
        self.pty ! Write { data };
    }

    on Resize { cols, rows } {
        self.pty ! Resize { cols, rows };
    }

    rite view(self) -> VNode! {
        TerminalGrid·view()
            ·content(self.pty.renderable_content())
            ·on_key(Input)
    }
}
```

**Affected Files:**
- `src/domains/terminal/`
- `src/services/terminalService.ts`

### 3. WebSocket/STOMP → Sigil Native Protocol

**Decision:** Use Sigil's native `protocol::websocket` module.

**Investigation Results (2026-02-16):**
- [x] Sigil has full RFC 6455 WebSocket support
- [x] Implementation: `tokio-tungstenite` backend (feature flag)
- [ ] STOMP framing layer needed (~100 LOC on top of WebSocket)

**Native WebSocket API:**
```sigil
// Connect
≔ ws = WebSocket·connect("wss://server.com/ws")?;

// Send/receive
ws·send(Message·text("hello"))?;
≔ msg = ws·receive()?;

// Close
ws·close()?;
```

**STOMP Actor (to be implemented):**
```sigil
☉ actor StompClient {
    state ws: WebSocket!,
    state subscriptions: Map<String, Subscription>!,

    on Connect { url, headers } {
        self.ws = WebSocket·connect(url)?;
        self.send_frame(StompFrame·Connect { headers });
    }

    on Subscribe { destination, callback } {
        ≔ id = self.next_sub_id();
        self.subscriptions.insert(id, Subscription { destination, callback });
        self.send_frame(StompFrame·Subscribe { id, destination });
    }

    on Send { destination, body } {
        self.send_frame(StompFrame·Send { destination, body });
    }
}
```

**Current Usage:**
- `@stomp/stompjs` for messaging
- `sockjs-client` for fallback (not needed for native)
- Used for real-time collaboration features

---

## Custom Hooks → Service Actors

The 3 custom hooks will become service actors:

### usePreferences → PreferencesService

**Source:** `src/wraith/hooks/usePreferences.ts`

```sigil
☉ actor PreferencesService {
    state layout: LayoutConfig!,
    state panels: PanelConfig!,
    state tabs: TabConfig!,
    state editor: EditorSettings!,

    on UpdatePreferences { patch } {
        // Apply patch to relevant state
        self.persist();
    }

    on ResetPreferences {
        self.layout = LayoutConfig·default();
        self.panels = PanelConfig·default();
        // ...
    }

    rite persist(self) {
        Storage ! Save { key: "wraith-preferences", value: self.serialize() };
    }
}
```

### useTabManagement → TabService

**Source:** `src/wraith/hooks/useTabManagement.ts`

```sigil
ᛈ TabServiceMsg {
    OpenFile { path: Path },
    CloseTab { id: TabId },
    TogglePin { id: TabId },
    OpenInSplit { id: TabId, direction: SplitDirection },
    CloseSplit { id: SplitId },
    MoveToSplit { tab_id: TabId, split_id: SplitId },
    OpenGitDiff { path: Path, ref_a: GitRef, ref_b: GitRef },
    OpenGitBlame { path: Path },
    NavigateNext,
    NavigatePrev,
}

☉ actor TabService {
    state tabs: Vec<Tab>!,
    state active_tab: Option<TabId>!,
    state splits: SplitLayout!,

    on OpenFile { path } { /* ... */ }
    on CloseTab { id } { /* ... */ }
    // ... handlers for each message
}
```

### useKeyboardShortcuts → KeyboardService

**Source:** `src/wraith/hooks/useKeyboardShortcuts.ts`

```sigil
☉ actor KeyboardService {
    state shortcuts: Map<KeyCombo, Action>!,

    on Mount {
        self.register_defaults();
        Window ! AddKeyListener { handler: self.handle_key };
    }

    on KeyPress { combo } {
        ⌐ action = self.shortcuts.get(combo)? {
            self.dispatch_action(action);
        }
    }

    rite register_defaults(self) {
        self.shortcuts.insert(KeyCombo·new("Cmd+P"), Action·CommandPalette);
        self.shortcuts.insert(KeyCombo·new("Cmd+B"), Action·ToggleSidebar);
        self.shortcuts.insert(KeyCombo·new("Cmd+G"), Action·GoToLine);
        // ... 15+ shortcuts
    }
}
```

---

## Migration Phases

```
┌─────────────────────────────────────────────────────────────────┐
│  PHASE 0: Investigation                                         │
│  ├── 0.1 WebSocket support in Sigil/Qliphoth                   │
│  ├── 0.2 Storage/persistence primitives                         │
│  └── 0.3 Native window/input handling                           │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 1: Automated Extraction                                   │
│  ├── 1.1 Run migration tool on wraith                           │
│  ├── 1.2 Generate component specs                                │
│  ├── 1.3 Generate service actor specs                            │
│  └── 1.4 Identify manual intervention points                     │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 2: Core Infrastructure                                    │
│  ├── 2.1 PreferencesService actor                                │
│  ├── 2.2 TabService actor                                        │
│  ├── 2.3 KeyboardService actor                                   │
│  └── 2.4 ThemeContext → ThemeService                             │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 3: Specialized Components                                 │
│  ├── 3.1 Athame editor integration                               │
│  ├── 3.2 Native terminal (alacritty_terminal)                   │
│  └── 3.3 WebSocket/real-time communication                       │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 4: UI Component Migration                                 │
│  ├── 4.1 Atoms (Spinner, Badge, etc.)                           │
│  ├── 4.2 Molecules (PanelHeader, TabItem, etc.)                 │
│  ├── 4.3 Organisms (TabBar, Toolbar, PanelContainer)            │
│  └── 4.4 Panels (20+ extension panels)                          │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 5: Domain Migration                                       │
│  ├── 5.1 DevOps Studio (32 components)                          │
│  ├── 5.2 Prompt Designer (11 components)                        │
│  ├── 5.3 Chat interface                                          │
│  ├── 5.4 Tasks                                                   │
│  └── 5.5 Auth flow                                               │
├─────────────────────────────────────────────────────────────────┤
│  PHASE 6: Integration & Testing                                  │
│  ├── 6.1 End-to-end workflow testing                            │
│  ├── 6.2 Performance validation                                  │
│  └── 6.3 Native packaging                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Mapping Reference

### Special Replacements

| React Component | Qliphoth Replacement | Notes |
|-----------------|---------------------|-------|
| `CodeEditor` (Monaco) | `Athame·view()` | Custom editor |
| `Terminal` (xterm.js) | `NativeTerminal·view()` | alacritty_terminal |
| `usePreferences()` | `PreferencesService` | Actor |
| `useTabManagement()` | `TabService` | Actor |
| `useKeyboardShortcuts()` | `KeyboardService` | Actor |
| `ThemeContext` | `ThemeService` | Actor |

### Standard Mappings (Automated)

| React Pattern | Qliphoth Pattern |
|--------------|------------------|
| `useState(x)` | `state field: T! = x` |
| `useRef<T>()` | `state ref: Option<T>! = ∅` |
| `useEffect([], f)` | `on Mount { f() }` |
| `onClick={f}` | `·on_click(Msg)` |
| `className="x"` | `·class("x")` |
| `{cond && <X/>}` | `·when(cond, X·view())` |
| `items.map(...)` | `·children(items.iter().map(...))` |

---

## External Dependencies

### Keep (via FFI/interop if needed)
- TanStack Query → evaluate Qliphoth data fetching patterns
- axios → Qliphoth HTTP client
- react-markdown → Sigil markdown renderer (or port)

### Replace
- Monaco Editor → Athame
- xterm.js → alacritty_terminal wrapper
- @dnd-kit → Qliphoth drag-and-drop primitives
- lucide-react → Qliphoth icon system

### Evaluate
- @stomp/stompjs → depends on WebSocket investigation
- sockjs-client → likely not needed for native

---

## Open Questions

1. **Athame Status:** What's the current state of Athame? Is it ready for integration?
2. **Native Window:** How does Qliphoth handle native window creation/management?
3. **File System:** What's the Qliphoth pattern for file system access?
4. **IPC:** How do we communicate with backend services (Git, LSP, etc.)?

---

## Flagged Replacement Points

### Monaco Editor → Athame

**Files requiring manual replacement:**

| File | Component | Action |
|------|-----------|--------|
| `code-editor.sigil` | `Editor·view()` | Replace with `Athame·view()` |
| `home-page.sigil` | CodeEditor reference | Update import/usage |

**Prop Mapping:**
```sigil
// FROM (Monaco)
Editor·view()
    ·attr("value", content)
    ·attr("language", get_language(path))
    ·on_change(Change)
    ·attr("options", { ... })

// TO (Athame)
Athame·view()
    ·content(content)
    ·syntax_mode(get_language(path))
    ·on_change(ContentChanged)
    ·settings(EditorSettings { ... })
```

### xterm.js → Native Terminal

**Files requiring manual replacement:**

| File | Component | Action |
|------|-----------|--------|
| `terminal-panel.sigil` | `render_terminals()` | Replace with `NativeTerminal·view()` |
| `container-actions.sigil` | Terminal usage | Update to native |
| `home-page.sigil` | Terminal reference | Update import |

**Architecture:**
```sigil
// Terminal service actor
☉ actor TerminalService {
    state terminals: Map<TerminalId, Terminal>!,
    state active: Option<TerminalId>!,

    on Create { profile } {
        ≔ term = Terminal·spawn(profile)?;
        ≔ id = TerminalId·new();
        self.terminals.insert(id, term);
        self.active = Some(id);
    }

    on Write { id, data } {
        ⌐ term = self.terminals.get_mut(&id)? {
            term.pty ! Write { data };
        }
    }
}

// Native terminal view
NativeTerminal·view()
    ·terminal(self.terminals.get(&id))
    ·on_input(Input)
    ·on_resize(Resize)
```

### Custom Hooks → Service Actors (Manual)

The 3 custom hooks are in separate files and weren't detected during per-file analysis. They must be manually converted:

| Hook File | Target Actor |
|-----------|-------------|
| `src/wraith/hooks/usePreferences.ts` | `PreferencesService` |
| `src/wraith/hooks/useTabManagement.ts` | `TabService` |
| `src/wraith/hooks/useKeyboardShortcuts.ts` | `KeyboardService` |

---

## Migration Output

**Generated:** 2026-02-16
**Location:** `/tmp/wraith-migration/`

| Artifact | Count | Path |
|----------|-------|------|
| Sigil Components | 125 | `output/*.sigil` |
| Component Specs | 125 | `components/*.json` |
| Type Definitions | ~50 | `types/*.json` |
| Patterns | 1 | `patterns/library.json` |

---

## Session Log

### 2026-02-16: Initial Planning
- Explored wraith codebase structure
- Identified 125 React components, 3 custom hooks
- Decided: Monaco → Athame, xterm.js → native
- Created migration roadmap

### 2026-02-16: Investigation & Migration
- Investigated WebSocket support - **AVAILABLE** in Sigil `protocol::websocket`
- Ran migration tool - **125 Sigil files generated**
- Flagged Monaco replacement: `code-editor.sigil`, `home-page.sigil`
- Flagged Terminal replacement: `terminal-panel.sigil`, `container-actions.sigil`
- Noted: Custom hooks need manual conversion (cross-file analysis gap)

### Next Steps
1. Copy generated Sigil files to Qliphoth project structure
2. Implement `PreferencesService`, `TabService`, `KeyboardService` actors
3. Replace Monaco with Athame editor
4. Implement native terminal with `alacritty_terminal`
5. Add STOMP framing layer for WebSocket communication
