# MCF long-term architecture options

A spectrum of options for where the MCL goes from here in order of least to most work. Each tier is
a real, standalone stopping point, not a step that forces the next one. Later tiers build on
earlier ones; nothing here recommends skipping a tier, only how far to go before stopping.

This is the whole-library view. [`MAINTENANCE_OPTIONS.md`](./MAINTENANCE_OPTIONS.md) already
answers the question of where *this one extension crate* goes (the A+B hybrid: stays a single crate
indefinitely, graduates per-feature into core, splits into its own crate/repo only for a wholly
new domain like MQTT) - that decision is assumed here, not reopened. This doc is about core's own
path, and how it interacts with however many extension crates exist over time.

## Current library design features

Findings from reviewing the real repo, not assumptions:

- **Four languages are genuinely schema-generated** (Rust, Python, Node/TS, C++) via
  `schema/capabilities/*.yaml` -> `tools/generate_capabilities.py` -> each language's
  `generated.*`. **Go is a fully hand-written, independently-maintained port** - it reached full
  parity with the other four at `0.2.0-rc.6`, but it was never brought into the generator itself,
  and nothing in the schema or the tool mentions it.
- **The "codegen" partially generates.** Every `render_*` function in `generate_capabilities.py`
  opens with `_ = api, models` - it discards the parsed YAML and returns a fixed, hand-typed
  string per language. There is no loop over models or fields anywhere in the file. Adding one
  field (`recoater_blade_type`) took one YAML line plus hand-editing 20+ files across all five
  languages. The YAML schema documents the field; it does not generate the code for it.
- **This has already produced real bugs**, not hypothetical ones: C++'s `parent_path()` and Go's
  `filepath.Dir()` share the same trailing-separator bug class, independently introduced in two
  hand-ports of the same logic. Node's mock v1.1 reader double-wrote `Facility_ID`, masked because
  tests still passed on the resulting data. `contributing.md` cites a confirmed silent data-loss
  bug in a downstream consumer from a v1.0->v1.1 migration that left old and new fields
  side-by-side.
- **Core's facade is stateful/object-shaped.** `open_machine_config`/`create_machine_config`
  return `Box<dyn MachineConfigFile>` - an open session with many `get_x`/`set_x` calls, not a
  one-shot function. That is a materially bigger lift for FFI/bindgen tooling than an extension
  crate's one-shot `read_config(path) -> Struct` shape.
- **The pre-facade legacy layer is the single hardest thing in the codebase to ever FFI-wrap.**
  `MachineConfigWriter<'a>` has a public lifetime parameter escaping into the struct, plus a
  higher-ranked-trait-bound function pointer type. No mainstream binding-gen tool represents that
  without an actual ownership redesign. Real consumers (clearbox-tauri) have already moved past
  this layer to the newer facade, for what it's worth.
- **This whole project is ~10 weeks old** (first CHANGELOG entry 2026-07-31), still entirely
  pre-1.0 (`0.1.x`/`0.2.x-rc.N`). Every tier below is a bet on a young, still-settling codebase,
  not a mature one.

## Tier 0 - freeze core, Rust-only extensions (least work)

**What:** Stop shipping new core schema versions (`v1.2`+) through the five-language pipeline at
all. Core stays at `v1.1` indefinitely. All new config/capability growth happens exclusively in
Rust-only extension crates using the already-decided A+B pattern - single crate, branch-gated,
graduates into core *only* if a concrete multi-language need materializes for that specific
feature.

**Costs:** Other-language consumers (Python/Node/C++/Go users) never see anything newer than
`v1.1`'s surface unless a specific extension separately earns its own graduation effort. No new
core schema capability reaches them by default.

**Buys:** Every future version bump under the current system re-pays the full five-language
hand-port-plus-`cross_check.py` cost that already produced three real bugs. Freezing caps that
cost at what's already been paid and validated. Rust-side schema growth (what capture actually
needs) stays cheap and fast, unaffected.

**Right when:** No concrete near-term need for new capability in a non-Rust consumer. This is the
floor every other tier builds on - not a temporary state, a real stopping point.

## Tier 1 - resume multi-language growth, same manual process (status quo)

**What:** Keep shipping new core versions (`v1.2`, `v1.3`, ...) through the same hand-port process
`v1.1` went through: a migration manifest, four structural files per language under
`capabilities/vX_Y/`, Python's three separate dispatcher registries, seven required test
categories per language, a full `cross_check.py` pass - exactly what `contributing.md`'s release
checklist already documents.

**Costs:** Pays the full five-language hand-port tax, indefinitely, for every version. The three
real bugs already found are evidence of what that tax actually costs in practice, not a
hypothetical risk.

**Buys:** Other-language consumers stay current with whatever new capability core adds, same as
today. No new tooling investment needed - the process already exists and works, just expensively.

**Right when:** New core capability is genuinely needed by non-Rust consumers soon, and the
per-version cost is judged acceptable given the project's current maturity and release cadence.

## Tier 2 - automate the existing pipeline for real

**What:** Close the gap this session found: make `generate_capabilities.py` actually consume
`models.yaml`/`api.yaml` instead of discarding them. A new field in `models.yaml` should emit a
real struct field in all four generated languages without touching the generator or hand-editing
20+ files. This is template engineering, not an architecture change - same YAML-first design,
same four languages, same facade shape. Go would still need a deliberate decision: bring it into
the same generator, or leave it a documented exception.

**Costs:** Real, nontrivial engineering effort up front to build genuine field-level templating
(likely per-language Jinja-equivalent templates driven by `models.yaml`'s actual field list,
handling each language's own type-mapping and null/optional conventions). Something to maintain
in its own right - a template bug now affects every language at once instead of one.

**Buys:** The three-already-found bug classes (duplicate hand-port logic errors, one language
silently missing a migration step) become structurally much harder to reintroduce, since there's
one template per concern instead of five independent hand-edits. Shrinks the per-version cost
Tier 1 accepts, without changing the fundamental YAML-first architecture or touching core's
stateful facade shape at all.

**Right when:** Tier 1's ongoing cost (or bug rate) becomes the active pain point, and the
language set itself (four, or four-plus-Go) is expected to stay roughly fixed - this tier is
about making the existing five-language commitment cheaper to sustain, not about growing it.

## Tier 3 - expand language support within the current model

**What:** Add a language genuinely into the schema-first pipeline - either formalize Go into
`generate_capabilities.py` so it stops being a hand-port exception, or add a new language
entirely (Java, a second systems language, etc.), using whatever templating exists at the time
(manual per Tier 1, or real per Tier 2).

**Costs:** A new language is a new, ongoing hand-port (or template) surface, plus its own CI/test
matrix, its own packaging story, its own copy of every future migration's checklist item. Doing
this before Tier 2 means a sixth hand-maintained copy of the same logic - directly compounding
the exact problem already observed with Go.

**Buys:** A real new consumer base gets first-class support matching the existing four (or five).

**Right when:** A concrete, scheduled need exists for a specific new language consumer - not a
hypothetical future one. Strongly prefer doing this after Tier 2, not before - adding a sixth
hand-ported language on top of an already-expensive manual process multiplies the cost this whole
document is trying to help control, rather than managing it.

## Tier 4 - expand extensions (the A+B pattern, generalized)

**What:** The `opcua-config-ext` pattern - single Rust-native crate, branch-gated collision
avoidance, per-feature graduation into core - gets deliberately reused for other, unrelated config
domains as they come up (the MQTT example already discussed). This is the trigger
`MAINTENANCE_OPTIONS.md`'s Option C is reserved for: a wholly new extension domain, at the point
real buildout for N-many independent extensions begins.

**Costs:** At that point, the shared plumbing currently living inside `rust-opcua-ext`'s own
source (the `RawGroupAccess` trait and its two adapter impls, specifically) needs promoting into
core or a small shared support crate, so a second extension crate isn't copy-pasting it. A
one-time cost, not something to pay speculatively before a second extension actually exists.

**Buys:** New config domains get the same fast, cheap, single-language iteration that validated
well for the OPC UA schema, without forcing every new feature through core's expensive five-
language process while it's still finding its shape.

**Right when:** A second, genuinely unrelated extension domain is real, not hypothetical - per
`MAINTENANCE_OPTIONS.md`'s already-settled framing, this is independent of core's own tier
(Tiers 0-3 above); extensions can keep growing this way regardless of what core is doing.

## Tier 5 - full FFI/WASM architectural shift (most work)

**What:** Replace the YAML-codegen pipeline with Rust-source-first binding generation
(UniFFI/Diplomat-style, or the WebAssembly Component Model/WIT) for both core and extensions.
Core's facade would need real redesign work beyond what an extension needed: its object/handle
shape (`Box<dyn MachineConfigFile>`) maps to UniFFI's "interface" object type, workable but a
different binding pattern than a plain record; the legacy `MachineConfigWriter<'a>` layer's
escaping lifetime genuinely cannot be FFI-wrapped as-is and would need an ownership redesign or
outright retirement (real consumers have already moved off it, which argues for retiring rather
than preserving it through a rewrite).

**Costs:** The largest commitment on this list by a wide margin. Touches core's actual facade
design, not just its tooling; the hardest piece (the legacy writer) needs a real redesign, not a
wrapper. Abandons the YAML schema as a design artifact (though its *intent*, and the validation
discipline built around it, carries forward).

**Buys:** One source of truth for every language, mechanically derived - the exact class of bug
already found three times in this codebase (two independent same-class porting bugs, one silent
migration data-loss bug) becomes structurally much harder to introduce at all, for every language
and every extension, going forward.

**Right when:** Multiple new languages are required for both core and new extensions at once -
only at that scale does paying for the legacy-layer redesign become worth it alongside everything
else. Per the earlier conversation that produced this framing: this is more tractable than it
first looks, since there is less working automation to unwind than Tier 1/2's "proven pipeline"
framing might suggest - but it is still the heaviest option here, and should be the last one
reached for, not the first.

## Quick reference

| Tier | Core schema growth | Multi-language cost | New extension domains |
| --- | --- | --- | --- |
| 0 | Frozen at v1.1 | None (frozen) | Rust-only, A+B pattern |
| 1 | Continues, manual hand-port | Full, per version (status quo) | Rust-only, A+B pattern |
| 2 | Continues, real templating | Reduced, per version | Rust-only, A+B pattern |
| 3 | Continues | Reduced + a new language | Rust-only, A+B pattern |
| 4 | Independent of this axis | Independent of this axis | Generalized A+B, N domains |
| 5 | Replaces the pipeline entirely | One source of truth, all languages | Same bindgen tooling as core |

Tiers 0-3 are a single axis (core's own schema-growth-versus-automation posture). Tier 4 is an
orthogonal axis (how many extension domains exist) that can combine with any of 0-3. Tier 5 is a
replacement for the whole axis, not a further step along it.
