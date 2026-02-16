# Qliphoth TODO

## qliphoth-test: Generic Test Framework

**Status**: Proposed
**Origin**: Extracted from Orpheus UI test harness (2026-02-12)
**Priority**: Medium

### Summary

Create a project-independent test framework for Sigil/Qliphoth applications. The pattern of friction-based workflow testing is generic and should be available to all Qliphoth apps.

### Components to Extract

From `orpheus-ui/tests/test_harness.sg`:

```sigil
// Core metrics (generic)
WorkflowMetrics        // Step/friction tracking
FrictionReport         // Analysis with verdicts
FrictionVerdict        // Acceptable/Warning/PainPoint
TaskComplexity         // Simple (≤5 steps) / Medium (≤10) / Complex (≤20)

// Input simulation (generic)
Action                 // Click, Drag, KeyPress, TypeText, Scroll, ContextMenu
Key                    // Keyboard key enumeration
Modifiers              // Ctrl, Shift, Alt, Meta
Point                  // 2D coordinates for mouse ops

// Threshold functions (configurable)
max_steps_simple()     // Default: 5
max_steps_medium()     // Default: 10
max_steps_complex()    // Default: 20
max_mode_switches_*()  // Context switch limits
max_undo_operations()  // Default: 3
```

### Proposed API

```sigil
invoke qliphoth_test·prelude·*;

// Generic TestApp trait
trait TestApp {
    type State;

    rite click(&Δ self, target: &str);
    rite drag(&Δ self, target: &str, from: Point, to: Point);
    rite key_press(&Δ self, key: Key, modifiers: Modifiers);
    rite type_text(&Δ self, text: &str);
    rite metrics(&self) -> &WorkflowMetrics;
    rite state(&self) -> &Self::State;
}

// Usage in app-specific tests
☉ Σ MyAppTest {
    // App-specific state
}

⊢ TestApp ∀ MyAppTest {
    type State = MyAppState;
    // Implement...
}
```

### Why This Matters

1. **Friction-based testing** catches UX problems that unit tests miss
2. **Consistent methodology** across all Qliphoth apps
3. **Reusable infrastructure** - don't rebuild for each project
4. **Documented thresholds** - what "too complex" means is explicit

### Implementation Notes

- Keep thresholds configurable (different apps have different complexity norms)
- Consider a `#[friction_test]` attribute macro for test discovery
- Integrate with `sigil test --report-friction` for CI

### References

- Orpheus UI test harness: `orpheus-desktop/crates/orpheus-ui/tests/test_harness.sg`
- TDD Roadmap: `orpheus-desktop/crates/orpheus-ui/TDD-ROADMAP.md`
