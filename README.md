# Qliphoth (git@github.com:Daemoniorum-LLC/qliphoth.git)

**The meta-statement: A web platform built entirely in Sigil.**

Qliphoth is the unified web frontend for the Daemoniorum ecosystem, written 100% in Sigil and compiled to WebAssembly. It replaces the existing React (daemoniorum-app) and Vue (docs-site) applications with a single, coherent platform.

## Architecture

```
qliphoth/
├── crates/
│   ├── qliphoth-core/    # API client, routing, state management
│   ├── qliphoth-ui/      # 40+ Corporate Goth components
│   ├── qliphoth-viz/     # Data visualization with morphemes
│   ├── qliphoth-app/     # Main web application
│   ├── qliphoth-docs/    # Documentation portal
│   └── qliphoth-chat/    # Infernum chat widget
├── scripts/              # Build and dev scripts
├── dist/                 # Build output (generated)
├── Sigil.toml           # Workspace configuration
└── Makefile             # Build commands
```

## Technology

- **Language**: Sigil (100% - no JavaScript)
- **Compilation**: WASM via LLVM backend
- **Styling**: Inline CSS with design tokens
- **State**: Signal-based reactivity
- **Routing**: Type-safe client-side routing

## Design System

The Corporate Goth aesthetic:

- **Colors**: Void (#0a0a0a), Phthalo (#123524), Crimson (#8b0000)
- **Typography**: Inter, JetBrains Mono
- **Shadows**: Phthalo glow, Crimson glow
- **Animations**: Fade, slide, pulse, serpent

## Components (40+)

### Layout
- Container, Grid, Stack, Flex
- PageShell, Header, Footer, Sidebar
- Section, Divider, Spacer

### Input
- Button, Input, Textarea, Select
- Checkbox, Radio, Switch
- Form, FormField

### Typography
- Heading, Text, Paragraph
- Code, Pre, Link
- Badge, Label

### Feedback
- Alert, Toast, Spinner
- Progress, Skeleton

### Navigation
- Nav, NavItem, Breadcrumb
- Tabs, TabPanel, Pagination
- Menu, MenuItem

### Data Display
- Card, CardHeader, CardBody
- Table, List, Avatar
- Tooltip

### Overlay
- Modal, Drawer
- Popover, Dropdown

### Specialized
- CodeBlock, Evidence
- ChatBubble, ProductCard
- StatCard, MetricDisplay
- JormungandrViz, SerpentPath

## Data Visualization (qliphoth-viz)

SVG-based charts where data flows through morpheme transformations:

```sigil
let chart! = data
    |φ{d => d.value > 0}           // Filter: keep positive
    |τ{d => DataPoint::new(d)}     // Transform: to chart points
    |σ{a, b => a.x·cmp(&b.x)}      // Sort: by x-axis
    |ρ{BarChart::render};          // Reduce: into visualization
```

### Charts
- **BarChart** - Vertical/horizontal bars with grouping
- **LineChart** - Lines with multiple interpolation modes (linear, monotone, step, cardinal)
- **AreaChart** - Filled areas with stacking and normalization
- **PieChart** - Circular proportions with slice interactivity
- **DonutChart** - Pie with center hole for totals
- **Sparkline** - Inline mini-charts (line, area, bar, bullet)

### Components
- **Axis** - Configurable axes with ticks, labels, grid
- **Scale** - Linear, Category, and Time scales
- **Legend** - Series identification with toggle support
- **Tooltip** - Contextual hover information

### Design
- Calibrated for dark backgrounds (Corporate Goth)
- 8-color series palette for distinguishability
- Semantic colors (positive/negative/warning)
- Smooth animations with configurable easing

## Getting Started

### Prerequisites

- Sigil compiler (v0.1.0+)
- wasm-bindgen-cli
- wasm-opt (optional, for optimization)

### Build

```bash
# Build all targets
make build

# Build specific target
make build-app
make build-docs

# Development mode with hot reload
make dev
```

### Development

```bash
# Start development server
make dev

# Run tests
make test

# Format code
make fmt

# Lint
make lint
```

## Deployment

Production builds are output to `dist/`:

```
dist/
├── qliphoth-app/
│   ├── index.html
│   ├── qliphoth_app.js
│   └── qliphoth_app_bg.wasm
└── qliphoth-docs/
    ├── index.html
    ├── qliphoth_docs.js
    └── qliphoth_docs_bg.wasm
```

Deploy to:
- **App**: qliphoth.dev
- **Docs**: docs.qliphoth.dev

## Sigil Features Used

- **Morphemes**: τ (transform), φ (filter), σ (sort), ρ (reduce)
- **Evidentiality**: ! (known), ? (uncertain), ~ (reported)
- **Incorporation**: · (middle dot for method chaining)
- **Bracket Generics**: `Vec[T]` instead of `Vec<T>`
- **Type Definitions**: `type Name = struct { ... }`
- **Async**: ⌛ (hourglass) for await

## License

Proprietary - Daemoniorum LLC
