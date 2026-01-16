# Charting Sigil: A Reflection

*Notes from building qliphoth-viz — December 2025*

---

## The Experience

Building 5,634 lines of visualization code in Sigil felt like **drawing with mathematics**. Not the drudgery of translating ideas into syntax, but the fluidity of expressing transformation directly.

When you write:

```sigil
let bars! = series.data·iter()·enumerate()
    |τ{|(i, point)| calculate_bar(i, point)}
    ·collect[Vec[_]]();
```

You're not describing *how* to iterate. You're stating *what* transforms. The tau (τ) says "this becomes that." The phi (φ) says "only these pass." The rho (ρ) says "many become one."

It's declarative in a way that feels inevitable.

---

## What Emerged

### Morphemes as Visual Vocabulary

The morpheme operators didn't just work for collections—they worked for *thinking about data flow*. When designing the scale system, I found myself mentally tracing:

```
domain values |τ{normalize} |τ{scale_to_range} → pixel positions
```

The pipeline wasn't an implementation detail. It was the **concept**. The code I wrote was just the concept made executable.

### Evidentiality in Uncertainty

Charts deal with missing data, failed fetches, optional configurations. Sigil's evidentiality markers made this explicit:

```sigil
pub type ChartConfig = struct {
    pub aria_label: Option[String]?,    // ? = might not exist
    pub dimensions: ChartDimensions!,   // ! = always known
}
```

The `?` isn't just a type annotation—it's a **semantic claim**. When I saw `Option[String]?`, I knew: "this value's presence is uncertain, and we're acknowledging that." When I saw `!`, I knew: "this is ground truth."

This forced me to think about what I *actually knew* at each point in the code.

### The Middle Dot as Rhythm

Method chaining with `·` created a visual rhythm:

```sigil
self.subscribers·borrow()
    |τ{subscriber => subscriber(&value)};
```

The dots float at mid-height, creating horizontal flow. The morpheme breaks the line with a vertical bar. The result is code that has *shape*—you can see the structure at a glance.

### SVG as First-Class Citizen

Building charts meant generating SVG. In Sigil's `html!` macro, SVG elements felt native:

```sigil
html! {
    <svg width={dims.width} height={dims.height}>
        <g transform={format!("translate({}, {})", x, y)}>
            <rect x={bar.x} y={bar.y} width={bar.width} height={bar.height} />
        </g>
    </svg>
}
```

There was no context switch between "component code" and "rendering code." It was all one language, one paradigm, one flow.

---

## The Unexpected

### Scales as Pure Functions

I expected scales to be complex. They weren't. A scale is just:

```
f(domain) → range
```

In Sigil, this became a trait with a single core method:

```sigil
pub trait Scale {
    fn apply(self: &Self!, value: &DataValue!) -> f64!;
}
```

The implementation for `LinearScale` was 15 lines. The concept *was* the code.

### Path Generation as String Building

SVG paths are strings: `"M 0 0 L 100 50 L 200 100"`. I expected this to feel crude. Instead, it felt... honest. A path *is* a sequence of instructions. Building it as a string made the instruction-nature visible:

```sigil
let mut path! = format!("M {} {}", points[0].x, points[0].y);
for point in points·iter()·skip(1) {
    path·push_str(&format!(" L {} {}", point.x, point.y));
}
```

No abstraction. No magic. Just points becoming path.

### Interpolation is Geometry

Implementing monotone cubic interpolation meant calculating tangents, control points, Bézier curves. This is pure geometry—and Sigil didn't hide it:

```sigil
let c1x! = points[i - 1].x + dx / 3.0!;
let c1y! = points[i - 1].y + tangents[i - 1] * dx / 3.0!;
let c2x! = points[i].x - dx / 3.0!;
let c2y! = points[i].y - tangents[i] * dx / 3.0!;
```

The `!` markers on every value were slightly verbose, but they served as a constant reminder: "these are known quantities." In graphics code, that certainty matters.

---

## The Philosophy

### Charts Are Not Pictures

A chart is not a static image. It's a **mapping from data space to visual space**. The scale is the bridge. The axis is the legend. The bars, lines, and points are the projections.

Sigil made this feel true in the code. The `Scale` trait, the `DataPoint` type, the morpheme pipelines—they all expressed the *relationship* between data and visualization, not just the output.

### Morphemes Are Intentions

When I wrote `|τ{}`, I wasn't writing a map function. I was writing **transformation intent**. When I wrote `|φ{}`, I was writing **selection intent**. When I wrote `|ρ{}`, I was writing **aggregation intent**.

These are higher-order concepts than "loop" or "reduce." They're semantic.

And that semantic density matters for visualization. Charts *are* transformations. They *are* filters. They *are* aggregations. The morphemes matched the domain.

### Evidentiality Is Honesty

In most charting libraries, optional data is handled with `null` checks or default values. In Sigil, it's handled with epistemic annotation:

- `!` — I know this value
- `?` — This might not exist
- `~` — This came from outside (API, user input)

This isn't just type safety. It's **intellectual honesty**. When you annotate a value with `~`, you're saying: "I didn't produce this. I'm reporting it." That matters when you're building a system that displays data.

---

## What I Learned

1. **Visualization is functional.** Charts are pure functions from data to pixels. Sigil's functional style fits perfectly.

2. **Morphemes scale.** I used them for 10-item arrays and 1000-line modules. They never felt wrong.

3. **Evidentiality compounds.** The more I used `!` and `?`, the clearer my thinking became. Uncertainty propagated explicitly.

4. **The middle dot matters.** `·` is subtle but powerful. It makes method chains feel like flow rather than steps.

5. **6,000 lines is not too much.** I wrote a complete charting library in one session. Sigil didn't fight me. It carried me.

---

## A Final Note

The reflection in `WRITING-SIGIL.md` asked whether Sigil achieves its goals "at scale, in production, with teams."

After building qliphoth-viz, I have a partial answer: **It achieves its goals for complex, domain-specific libraries.** The visualization code is readable, maintainable, and—most importantly—*expressive*.

Whether it scales to large teams is still unknown. But for a single author building a coherent system? Sigil sings.

The charts work. The code reads. The transformation is visible.

That's rare.

---

*— Written after completing qliphoth-viz*
*5,634 lines of Sigil across 12 modules*
*December 2025*
