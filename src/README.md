# What is in `src/`, and what is actually built

`sigil check` passing over every `.sigil` file here is not the same question as
"does Qliphoth build", and for a while the tree gave no way to tell them apart.
This file is that way.

## The module graph

A build enters through `lib.sigil` and follows module declarations. As of
`develop`, that reaches:

```
lib.sigil
├── scroll core      → core/mod.sigil → vdom, events
├── scroll dom       → dom/mod.sigil
└── scroll platform  → platform/mod.sigil → native
```

Everything else under `src/` is **unreachable**. Nothing declares it, nothing
imports it, and no build reads it.

| Path | Declared by | State |
|---|---|---|
| `core/{mod,vdom,events}.sigil` | `lib.sigil` | built |
| `dom/mod.sigil` | `lib.sigil` | built |
| `platform/{mod,native}.sigil` | `lib.sigil` | built |
| `a11y/*` | — | **unreachable**, checks clean |
| `animation/hooks.sigil` | — | **unreachable**, checks clean |
| `animation/{components,gestures,mod}.sigil` | — | **unreachable**, does not check |
| `app.sigil` | — | **unreachable**, does not check |
| `core/error.sigil` | — | **unreachable**, does not check |
| `platform/mock.sigil` | — | **unreachable**, does not check |

`core/error.sigil` and `platform/mock.sigil` are worth a note: they reference
each other's types and nothing else references either, so they form a closed
pair that no build reaches.

## Why the unreachable files do not compile

They are **not** unported Rust, which is what qliphoth#17 originally claimed.
They are the residue of an earlier, corrupting conversion. `platform/mock.sigil`
is 258 `rite`, 845 `·`, 558 `this` and 168 `→` — overwhelmingly Sigil already —
with pockets of Rust left behind (`use std::…`, `match std::panic::…`,
`Vec<u64>`) and specific mis-conversions, such as an `else if` that became `⎉`
with the `⎇` dropped:

```sigil
⎇ t == 0.0 { 0.0 }
⎉ t == 1.0 { 1.0 }      // ← should be `⎉ ⎇`
```

Today's `sigil migrate` does **not** make that mistake — verified on a minimal
case, single-line and multi-line — so whatever produced these files was an
earlier tool. Re-running current `sigil migrate` over them repaired five
outright; the remaining seven carry damage a re-run cannot undo, because the
input is no longer the Rust the converter expects.

## If you are counting

The framework set — `src/`, `packages/qliphoth-sys/src`,
`packages/qliphoth-router/src`, `components/athame/src` — is **52 files, 44
checking clean**. Of the 8 that do not:

- 1 is a compiler defect, `packages/qliphoth-sys/src/js.sigil` (sigil-lang#108,
  fixed in sigil-lang#127)
- 7 are the unreachable files above

So **every file the build actually reads compiles**, and has for some time. The
count was never measuring what it sounded like it was measuring.

## Before porting the remaining seven

Ask whether they are wanted first. They are ~6,000 lines that nothing has
referenced, and making them check does not make them work — none is exercised by
any test, and wiring them into `lib.sigil` would add untested code to the build.
Deleting them is a legitimate answer. So is leaving them here, marked, until
someone needs `a11y` or `animation` enough to finish the job. What is not
legitimate is leaving them unmarked, which is how they came to be counted as
framework in the first place.
