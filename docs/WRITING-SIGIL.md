# Writing Sigil: A Reflection

*Notes from building Qliphoth - December 2024*

---

## The Experience

Writing 6,600+ lines of Sigil felt like programming with **intentionality**. Every syntactic choice in the language carries meaning beyond mere structure—it's not just code, it's semiotics.

## What Struck Me

### The Morphemes

The first time I wrote `|τ{x => x·to_uppercase()}` instead of `.map(|x| x.to_uppercase())`, something clicked. The tau (τ) isn't just shorthand—it's a glyph that means *transform*. When you read Sigil, you don't parse syntax, you read meaning:

- `|τ{}` — I see transformation happening
- `|φ{}` — I see filtering
- `|ρ+` — I see reduction/accumulation
- `|α` — I see "take the first"
- `|ω` — I see "take the last"

These aren't arbitrary symbols. They're **ideographs**. The code becomes closer to mathematical notation than to prose.

### Evidentiality

This was the most philosophically interesting part. In most languages, a value just *is*. In Sigil, you declare what you *know* about it:

```sigil
let name! = "Lilith";           // I know this with certainty
let config? = load_config();    // This might fail, uncertainty encoded
let metrics~ = api·fetch();     // Reported from external source
```

Adding `!` to a value isn't just type annotation—it's an **epistemic claim**. I found myself thinking: "Do I actually *know* this? Or am I just hoping?" The language forces you to confront certainty.

The paradox marker `‽` is haunting. It's for when you have contradictory information that you must hold simultaneously. It acknowledges that reality doesn't always resolve cleanly.

### The Middle Dot

Using `·` instead of `.` for method chaining felt ceremonial at first, then essential. The middle dot in Sigil represents **incorporation**—you're not just calling a method, you're weaving functionality into the value. It's a small visual distinction with large conceptual weight:

```sigil
response·json()·⌛·unwrap()
```

The dots float at mid-height, creating a visual flow. The code looks less like instructions and more like a continuous stream of transformation.

### The Hourglass

`⌛` for await is beautiful. Where Rust has the keyword `await` and JavaScript has the same, Sigil uses a symbol that *looks like waiting*. Time passes. The hourglass turns. The async operation completes.

```sigil
let data! = api·fetch()·⌛;
```

It's immediately legible even if you've never seen Sigil before. The symbol carries its own meaning.

### Type Definitions

`type ButtonProps = struct { ... }` instead of `struct ButtonProps { ... }`

This seems minor but it's not. The Sigil form is a **definition**, an equation. "ButtonProps equals this structure." It's declarative in a way that feels more like mathematics than engineering.

## The Cumulative Effect

After a few thousand lines, I stopped translating from "what I would write in Rust" to Sigil. I started *thinking* in Sigil. The morphemes became vocabulary. The evidentiality became instinct.

The code I wrote isn't just functionally correct—it's **epistemically annotated**. Every uncertain value is marked. Every transformation is visible. Every async boundary is symbolized.

## What I Learned

1. **Syntax can encode philosophy.** Evidentiality markers embed epistemology into the type system. The language doesn't just describe what the program does—it describes what we know about what it does.

2. **Symbols beat keywords.** Once internalized, `τ` is faster to read than `map`. It's a single glyph with a fixed meaning. Keywords are words; morphemes are ideographs.

3. **Visual rhythm matters.** The middle dot creates flow. The bracket generics `[T]` are cleaner than `<T>`. The hourglass stands out. Sigil code has a visual texture that aids comprehension.

4. **Constraints are freedom.** Being forced to mark evidentiality made me think more carefully about certainty. The constraint improved the code.

## A Personal Note

I've written millions of lines of code across dozens of languages. Sigil is the first language where I felt like I was doing something closer to **notation** than **programming**.

It reminded me of why mathematical notation works: not because it's terse, but because each symbol carries dense, stable meaning. When you write `∫`, you don't think "integral"—you think of the concept directly. Sigil aims for that.

Whether it achieves it at scale, in production, with teams—I don't know. But writing Qliphoth in Sigil felt like writing in a language that *cares* about what it means to know something.

That's rare.

---

*— Written after completing Phase 0 of Qliphoth*
*6,626 lines of Sigil across 5 crates*
*December 2024*
