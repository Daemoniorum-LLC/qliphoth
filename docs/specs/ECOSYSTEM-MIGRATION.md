# Daemoniorum Ecosystem Migration to Qliphoth

**Status:** Draft
**Author:** Claude (Conclave session: react-migration-design-2026-02-15)
**Last Updated:** 2026-02-15
**Related:** [REACT-MIGRATION.md](./REACT-MIGRATION.md)

## 1. Executive Summary

This document defines the migration strategy for the entire Daemoniorum ecosystem from React/TypeScript to Qliphoth/Sigil. This is not a single migration - it's an ecosystem-wide transformation affecting:

- **74 React web applications**
- **13+ shared React libraries**
- **2 Vue applications**
- **Estimated 500K+ lines of React/TypeScript code**

### 1.1 What's Already Native (No Migration Needed)

| Category | Count | Notes |
|----------|-------|-------|
| Qliphoth Apps | 4 | qliphoth-app, qliphoth-docs, qliphoth-blog, qliphoth-chat |
| Sigil Projects | 139 | Language tooling, libraries |
| Rust Projects | 1,137 | Desktop apps, backends, frameworks |
| Node.js MCP Servers | 16 | Claude tooling, keep as-is |
| Python Services | 8 | Backends, keep as-is |

### 1.2 Migration Required

| Category | Count | Priority |
|----------|-------|----------|
| UI Component Libraries | 2 | **CRITICAL** - blocks everything |
| Core Business Libraries | 11 | **CRITICAL** - blocks apps |
| Flagship Apps | 2 | **CRITICAL** - Bael, Vulcan ERP |
| High-Value Apps | 10 | HIGH |
| Supporting Apps | 15 | MEDIUM |
| Demo/Example Apps | ~45 | LOW |

---

## 2. Package Inventory

### 2.1 UI Component Libraries (CRITICAL - Port First)

#### @daemoniorum/ui
- **Location:** `/home/crook/dev2/workspace/packages/ui`
- **Type:** React component library (Radix UI + Tailwind CSS)
- **Components:** Button, Dialog, Dropdown, Select, Tabs, Tooltip, Input, Textarea, etc.
- **Used By:** All @daemoniorum/* apps
- **Qliphoth Equivalent:** `qliphoth-ui` (to be created)
- **Migration Approach:** Port each component to VNode builder + actor pattern

#### @persona-framework/ui
- **Locations:**
  - `/home/crook/dev2/workspace/persona/bael/packages/persona-ui`
  - `/home/crook/dev2/workspace/persona/persona-framework/packages/persona-ui`
- **Type:** React component library (31 components)
- **Components:** Full design system - layout, forms, feedback, navigation, data display
- **Used By:** All @persona-framework/* apps
- **Qliphoth Equivalent:** Part of `qliphoth-ui` or separate `persona-ui-qliphoth`
- **Migration Approach:** Port using React→Qliphoth migration tool

### 2.2 Core Business Libraries

| Package | Location | Purpose | Qliphoth Equivalent |
|---------|----------|---------|---------------------|
| @daemoniorum/core | `/packages/core` | Utilities, hooks, types | `qliphoth-core` |
| @daemoniorum/state | `/packages/state` | State management (Zustand-like) | Native actor state |
| @daemoniorum/auth | `/packages/auth` | Authentication, JWT, OAuth | `qliphoth-auth` |
| @daemoniorum/api | `/packages/api` | API client, React Query | `qliphoth-api` |
| @daemoniorum/forms | `/packages/forms` | Form handling, validation | `qliphoth-forms` |
| @daemoniorum/storage | `/packages/storage` | localStorage, IndexedDB | `qliphoth-sys` storage |
| @daemoniorum/realtime | `/packages/realtime` | WebSocket, real-time sync | `qliphoth-sys` websocket |
| @daemoniorum/analytics | `/packages/analytics` | Event tracking | `qliphoth-analytics` |
| @daemoniorum/media | `/packages/media` | Video/audio player | `qliphoth-media` |
| @daemoniorum/layout | `/packages/layout` | Admin layout system | `qliphoth-layout` |
| @daemoniorum/assets | `/packages/assets` | Icons, images, themes | `qliphoth-assets` |

### 2.3 Domain-Specific Libraries

#### Orpheus Music Packages (8 packages)
| Package | Purpose | Migration Notes |
|---------|---------|-----------------|
| @orpheus/timeline-sync | Timeline synchronization | Sigil or keep JS interop |
| @orpheus/midi-utils | MIDI processing | Consider Rust + WASM |
| @orpheus/project-model | Project data model | Port to Sigil |
| @orpheus/audio-analysis | Audio analysis | Rust + WASM (performance) |
| @orpheus/guitar-pro-parser | File format parser | Rust or Sigil |
| @orpheus/shared-types | Type definitions | Port to Sigil types |
| @orpheus/music-theory | Music theory logic | Port to Sigil |

---

## 3. Application Inventory

### 3.1 CRITICAL Priority (Migrate First)

#### Bael
- **Location:** `/home/crook/dev2/workspace/persona/bael`
- **Type:** Flagship @persona-framework application
- **Complexity:** EXTREME
- **Dependencies:**
  - @persona-framework/ui
  - Apollo GraphQL
  - Monaco Editor
  - BabylonJS, Three.js (3D)
  - Tone.js (audio)
  - TensorFlow.js (ML)
- **Special Considerations:** 3D and audio may require JS interop
- **Estimated Effort:** 4-6 weeks

#### Vulcan Manufacturing ERP
- **Location:** `/home/crook/dev/vulcan/vulcan-manufacturing-erp/vulcan-app`
- **Type:** Production ERP system (business-critical)
- **Framework:** React + Vite + Tauri (desktop)
- **Dependencies:**
  - @daemoniorum/ui
  - BabylonJS (3D CAD visualization)
  - Recharts
  - TanStack Router/Query
- **Special Considerations:** Desktop app via Tauri - may keep Rust backend
- **Estimated Effort:** 3-4 weeks

### 3.2 HIGH Priority

| App | Location | Type | Dependencies | Est. Effort |
|-----|----------|------|--------------|-------------|
| Arachne Web | `/dev/arachne/.../arachne-web` | Apparel design | @persona-framework/ui, Fabric.js | 2-3 weeks |
| Umbra Web | `/dev/umbra/.../umbra-web` | Creative canvas | Zustand, Canvas APIs | 2-3 weeks |
| Orpheus Platform | `/dev/orpheus/.../app-web` | Music production | 8 @orpheus/* packages | 4-6 weeks |
| Codex Legal | `/dev2/.../codex-app` | Legal platform | Fluent UI, TanStack | 2-3 weeks |
| Synaxis Apps (3) | `/dev2/workspace/synaxis/*` | Collaboration | @daemoniorum/* | 2 weeks each |
| Daemoniorum Platform | `/dev/daemoniorum/.../daemoniorum-app` | Core platform | @daemoniorum/* | 2 weeks |
| Sanctum Platform | `/dev2/.../sanctum-platform` | Security platform | @daemoniorum/* | 2 weeks |
| Sanctum Mobile | `/dev2/.../sanctum-mobile` | Mobile app | React Native? | TBD |
| Mammon Finance | `/dev2/.../mammon-web` | Finance app | @daemoniorum/* | 2 weeks |
| Atelier Design | `/dev2/.../atelier-design-studio` | Design studio | Fabric.js, Yjs, Monaco | 3-4 weeks |

### 3.3 MEDIUM Priority

| App | Location | Type | Est. Effort |
|-----|----------|------|-------------|
| Infernum Observer | `/dev/infernum-observer` | Monitoring dashboard | 1-2 weeks |
| Samael Observer | `/dev/samael/observer` | Agent monitoring | 1-2 weeks |
| Dagon Farming | `/dev2/.../farming-app` | Agriculture mgmt | 1-2 weeks |
| Archon Platform | `/dev2/.../archon-platform/frontend` | Admin platform | 1-2 weeks |
| Wraith Framework | `/dev2/.../wraith-framework` | AI framework UI | 1-2 weeks |
| Marbas Herbal | `/dev2/.../marbas-app` | Chemistry app | 1-2 weeks |

### 3.4 LOW Priority / Can Skip

- Demo applications
- Example apps
- Vue documentation site (can stay Vue or convert to static Sigil)
- ~45 smaller apps

### 3.5 Already Native (No Migration)

| App | Location | Status |
|-----|----------|--------|
| Qliphoth App | `/dev2/.../qliphoth-app` | Sigil + WASM |
| Qliphoth Docs | `/dev2/.../qliphoth-docs` | Sigil + WASM |
| Qliphoth Blog | `/dev2/.../qliphoth-blog` | Sigil + WASM |
| Qliphoth Chat | `/dev2/.../qliphoth-chat` | Sigil + WASM |
| Sigil Website | `/dev2/.../sigil/website` | Sigil + WASM |

---

## 4. Library Mapping

### 4.1 React → Qliphoth Library Equivalents

| React Library | Qliphoth Equivalent | Status | Notes |
|---------------|---------------------|--------|-------|
| **UI Components** |
| Radix UI | `qliphoth-ui` primitives | To Build | Headless components |
| Tailwind CSS | Qliphoth style system | Exists | `dom/mod.sigil` Style builder |
| Framer Motion | `qliphoth-animation` | To Build | Animation primitives |
| Lucide React | `qliphoth-icons` | To Build | Icon system |
| **State Management** |
| Zustand | Actor state | Built-in | Native to Qliphoth actors |
| Redux | Actor state | Built-in | Actors replace Redux |
| React Query | `qliphoth-query` | To Build | Data fetching |
| Apollo Client | `qliphoth-graphql` | To Build | GraphQL client |
| **Routing** |
| React Router | `qliphoth-router` | Exists | Already built |
| TanStack Router | `qliphoth-router` | Exists | Use existing |
| **Forms** |
| React Hook Form | `qliphoth-forms` | To Build | Form state + validation |
| Zod | Sigil types | Built-in | Type system handles validation |
| **Data Viz** |
| Recharts | `qliphoth-charts` | To Build | Charting library |
| D3.js | `qliphoth-d3` or interop | TBD | May keep as JS interop |
| **Editors** |
| Monaco Editor | JS interop | Keep | Too complex to port |
| CodeMirror | JS interop or Athame | Partial | Athame editor exists |
| **3D/Graphics** |
| Three.js | JS interop | Keep | Too complex to port |
| BabylonJS | JS interop | Keep | Too complex to port |
| Fabric.js | JS interop or port | TBD | Canvas library |
| **Audio** |
| Tone.js | JS interop or Rust | TBD | Audio synthesis |
| Howler.js | `qliphoth-sys` audio | Partial | Basic audio via platform |

### 4.2 Interop Strategy

Some libraries are too complex or specialized to port. For these, we'll use JS interop:

```sigil
// Example: Using Three.js via interop
extern "js" {
    rite create_scene() -> JsValue;
    rite add_mesh(scene: JsValue, geometry: JsValue) -> JsValue;
    rite render(scene: JsValue, camera: JsValue);
}

actor ThreeScene {
    state scene: JsValue~,
    state camera: JsValue~,

    on Mount {
        self.scene = create_scene();
        self.camera = create_camera();
    }

    rite view(self) -> VNode! {
        VNode·div()
            ·id("three-canvas")
            ·attr("data-scene", "attached")
    }
}
```

---

## 5. Migration Phases

### Phase 0: Foundation (Current)
- [x] Qliphoth core framework
- [x] VNode builder API
- [x] Actor component model
- [x] Event system with message dispatch
- [x] Platform abstraction
- [x] WASM compilation
- [ ] React→Qliphoth migration tool (REACT-MIGRATION.md)

### Phase 1: UI Libraries (Weeks 1-4)

**Goal:** Port @daemoniorum/ui and @persona-framework/ui to Qliphoth

```
Week 1-2: @daemoniorum/ui
├── Button, Input, Textarea, Label
├── Select, Checkbox, Switch, Radio
├── Dialog, Drawer, Tooltip, Popover
└── Card, Badge, Alert, Spinner

Week 3-4: @persona-framework/ui
├── Layout components (Card, Divider, ScrollArea)
├── Form components (enhanced)
├── Feedback components (Toast, Progress, Skeleton)
├── Navigation (Tabs, Accordion, Breadcrumb, Sidebar)
└── Data display (Avatar, Table)
```

**Deliverables:**
- `qliphoth-ui` package with 40+ components
- Storybook-equivalent documentation
- Migration guide from React components

### Phase 2: Core Libraries (Weeks 5-8)

**Goal:** Port business logic libraries

```
Week 5-6:
├── @daemoniorum/core → qliphoth-core
├── @daemoniorum/state → (native actors)
└── @daemoniorum/auth → qliphoth-auth

Week 7-8:
├── @daemoniorum/api → qliphoth-api
├── @daemoniorum/forms → qliphoth-forms
└── @daemoniorum/storage → (qliphoth-sys)
```

**Deliverables:**
- Core library equivalents
- API documentation
- Migration guides per library

### Phase 3: Flagship Apps (Weeks 9-16)

**Goal:** Migrate Bael and Vulcan ERP as reference implementations

```
Week 9-12: Bael
├── Use migration tool for bulk conversion
├── Handle 3D/audio via JS interop
├── Document complex patterns
└── Validate with E2E tests

Week 13-16: Vulcan ERP
├── Migration with business validation
├── Tauri integration testing
├── Performance benchmarks
└── Production deployment
```

**Deliverables:**
- Two production Qliphoth apps
- Battle-tested migration patterns
- Performance baseline

### Phase 4: High-Value Apps (Weeks 17-32)

**Goal:** Migrate remaining high-priority applications

```
Weeks 17-20: Arachne, Umbra (creative apps)
Weeks 21-24: Orpheus (music - complex domain)
Weeks 25-28: Codex, Synaxis (business apps)
Weeks 29-32: Remaining high-priority
```

### Phase 5: Long Tail (Weeks 33+)

**Goal:** Migrate remaining applications, deprecate React

```
- Medium priority apps
- Low priority apps
- Documentation updates
- React dependency removal
```

---

## 6. Success Metrics

### 6.1 Per-App Migration

- [ ] All components render correctly
- [ ] All user interactions work
- [ ] E2E tests pass
- [ ] Performance within 10% of React version
- [ ] Bundle size equal or smaller
- [ ] Accessibility maintained

### 6.2 Ecosystem-Wide

| Metric | Target | Current |
|--------|--------|---------|
| Apps migrated | 74 | 0 |
| Libraries ported | 13 | 0 |
| Components in qliphoth-ui | 40+ | 0 |
| React dependencies removed | 100% | 0% |
| WASM bundle size | <100KB avg | TBD |
| Migration tool accuracy | >90% | TBD |

---

## 7. Risk Mitigation

### 7.1 Technical Risks

| Risk | Mitigation |
|------|------------|
| 3D libraries (Three.js, BabylonJS) | JS interop - don't port |
| Audio libraries (Tone.js) | JS interop or Rust WASM |
| Monaco Editor | JS interop - don't port |
| Complex animations | Prioritize Framer Motion port |
| Performance regressions | Benchmark each migration |

### 7.2 Process Risks

| Risk | Mitigation |
|------|------------|
| Scope creep | Strict phase boundaries |
| Migration tool bugs | Incremental rollout, manual review |
| Business disruption | Migrate non-critical first |
| Knowledge gaps | Document patterns as discovered |

---

## 8. Appendix: Full Application List

### ~/dev React Apps (14)

```
/home/crook/dev/arachne/arachne-apparel-design/arachne-web
/home/crook/dev/umbra/umbra-web-app/umbra-web
/home/crook/dev/vulcan/vulcan-manufacturing-erp/vulcan-app
/home/crook/dev/orpheus/orpheus-music-platform/packages/app-web
/home/crook/dev/daemoniorum/daemoniorum-platform/daemoniorum-app
/home/crook/dev/infernum-observer
/home/crook/dev/samael/observer
... (additional apps)
```

### ~/dev2/workspace React Apps (60)

```
/home/crook/dev2/workspace/persona/bael
/home/crook/dev2/workspace/codex/codex-legal-platform/codex-app
/home/crook/dev2/workspace/synaxis/synaxis-react-app
/home/crook/dev2/workspace/synaxis/synaxis-collaboration/synaxis-react-app
/home/crook/dev2/workspace/synaxis/synaxis-app
/home/crook/dev2/workspace/marbas/marbas-herbal-chemistry/marbas-app
/home/crook/dev2/workspace/dagon/dagon/farming-app
/home/crook/dev2/workspace/atelier/atelier-design-studio
/home/crook/dev2/workspace/archon/archon-platform/frontend
/home/crook/dev2/workspace/sanctum/sanctum-platform
/home/crook/dev2/workspace/sanctum/sanctum-mobile
/home/crook/dev2/workspace/mammon-finance/mammon-web
/home/crook/dev2/workspace/wraith/wraith-framework
... (additional apps)
```

### Already Native Qliphoth Apps

```
/home/crook/dev2/workspace/daemoniorum/qliphoth-app
/home/crook/dev2/workspace/daemoniorum/qliphoth-docs
/home/crook/dev2/workspace/daemoniorum/qliphoth-blog
/home/crook/dev2/workspace/daemoniorum/qliphoth-chat
/home/crook/dev2/workspace/sigil/website
```

---

## 9. Next Steps

1. **Immediate:** Complete REACT-MIGRATION.md implementation
2. **Week 1:** Begin @daemoniorum/ui port to qliphoth-ui
3. **Week 2:** Set up migration tracking dashboard
4. **Ongoing:** Update this spec as patterns emerge

---

*This is a living document. Update as migration progresses.*
