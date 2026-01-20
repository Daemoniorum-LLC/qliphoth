# Changelog

All notable changes to Qliphoth will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-01-19

### Added

- **Athame Editor Component** - Full-featured Sigil code editor with syntax highlighting, bracket matching, and undo/redo
- **Cross-platform support** - Compile to WASM (browser), SSR (server), or GTK4 (desktop)
- **Comprehensive hooks library**
  - State: `use_state`, `use_reducer`, `use_context`
  - Effects: `use_effect`, `use_layout_effect`
  - Performance: `use_memo`, `use_callback`, `use_transition`, `use_deferred_value`
  - Data: `use_fetch`, `use_mutation`
  - DOM: `use_ref`, `use_intersection`, `use_animation_frame`
  - Utilities: `use_debounce`, `use_throttle`, `use_local_storage`, `use_media_query`
- **Actor-based state management** with time-travel debugging
- **Type-safe router** with guards, protected routes, and dynamic parameters
- **Platform abstraction layer** for cross-platform development
- **E2E test suite** with Playwright

### Changed

- Restructured project with workspace packages (`qliphoth-sys`, `qliphoth-router`)
- Converted to standard Sigil syntax throughout codebase
- Improved VDOM reconciliation algorithm

## [0.1.0] - 2025-01-01

### Added

- Initial release
- Core VDOM implementation
- Basic component system with lifecycle events
- Signal-based reactivity
- Browser bindings via `qliphoth-sys`
- HTML element builders and primitives

[0.2.0]: https://github.com/Daemoniorum-LLC/qliphoth/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Daemoniorum-LLC/qliphoth/releases/tag/v0.1.0
