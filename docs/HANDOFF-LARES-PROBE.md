# Handoff — the Lares capability probe (branch `claude/lares-spec-sigil-rebuild-2cqmp3`)

**Full handoff and the gap register live in the Lares repository:**
`docs/HANDOFF-SIGIL-REBUILD.md` and `docs/specs/LARES-SIGIL-REBUILD-SPEC.md`
(spec 1.30.0, gaps S1–S124). This file is the pointer, plus what a session
landing *here* needs.

## What the branch is

The host half of `Daemoniorum-LLC/sigil-lang#62`. A module importing a function
the host does not provide fails at `WebAssembly.instantiate`, so the two land
together.

It exists because Lares — a real React application — was ported to Sigil to find
out what Sigil and Qliphoth cannot yet do. **The gaps are the deliverable**, not
the app.

## Before you touch PR #10

**It conflicts with `develop` — 10 files, 13 commits behind — and unlike the
sigil-lang side there is no pending merge to wait for.** That resolution is
genuinely yours. It was flagged rather than done so it could be reviewed on its
own instead of buried inside a runtime PR.

The base was retargeted from `main` to `develop`.

## The standing hazard: four copies of `sigil_runtime.js`

Canonical is `runtime/sigil_runtime.js`. Copies live in the Lares repo at
`lares-client/e2e-{fetch,tab,wasm}/` and in sigil-lang at `website-qliphoth/`
and `website-qliphoth/deploy/`.

**Add a host function, forget to copy, and you get
`function import requires a callable` at instantiate** — which reads like a
compiler bug and is a copy that drifted. This happened twice on this branch.
Five copies of one file is not a solved problem; it is a thing waiting to go
wrong, and collapsing them is worthwhile work in its own right.

After any runtime change:

```bash
cd <qliphoth>/runtime
for d in <lares>/lares-client/e2e-fetch <lares>/lares-client/e2e-tab \
         <lares>/lares-client/e2e-wasm <sigil-lang>/website-qliphoth \
         <sigil-lang>/website-qliphoth/deploy; do
  cp sigil_runtime.js "$d/"
done
md5sum sigil_runtime.js <each copy>    # must all match
```

## Before you change anything about `None`

`NONE` is exported here and defined identically as `wasm::NONE` in the compiler,
**pinned by a test**. It is a sentinel, not 0 — 0 is `false`, the integer zero,
and this file's own "absent". `conn_row` reads `up: boolean | null` and has to
tell DOWN from NOT-YET-CHECKED, which is exactly what a 0 sentinel broke.

`valueToBool`, `stringFromValue` and `jsonResolve` all answer for it, and the
four "optional pointer" reads treat it as absent alongside 0. **A new host
function taking an optional pointer needs the same.**

Note also S124: the interpreter does not share this model at all — `==` is
strictly typed there and untyped in WASM. Do not "reconcile" the two without
reading §8.2.29.

## Checking

There is no unit suite here. The runtime is exercised through the Lares client
and the Sigil website:

```bash
cd <lares>/lares-client && SIGIL=<sigil> ./regenerate.sh   # 93 compiling / 0 failing
<sigil> wasm project/ -o e2e-tab/project.wasm              # the LINK — separate check
cd e2e-tab && node survey.mjs                              # 93 / 93 render
node look.mjs                                              # then READ look.png
cd <sigil-lang>/tools/differential && node imports.mjs     # 220 checked, 0 wrong type
cd <lares>/lares-client/e2e-fetch && node drive.mjs        # real HTTP through JSPI
```

`imports.mjs` checks every host function against the type the module declares.
It is what catches a host returning a plain Number where an i64 is declared —
which throws at the call boundary and reads as a compiler bug.

`survey.mjs` and `look.mjs` read a **prebuilt** `project.wasm`. If the link step
failed, they will happily test the stale one and report success.
