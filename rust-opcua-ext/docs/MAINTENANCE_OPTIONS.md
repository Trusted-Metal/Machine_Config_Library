# Where this crate goes from here

Three real options, not a ranked list - each is correct under a different set of conditions, and
the right call depends on which conditions actually hold.

## Current direction (decided 2026-10-13)

**This is, in practice, an A+B hybrid, not a single pick between them.** Once a given branch's
work inside this crate is stable, that piece graduates into MCL core [Option A]: it gets
multi-language support via the existing codegen pipeline and joins the standard set of
extension/base support core offers. B stays permanently active alongside that, not as a fallback
or a transitional state - it's where whatever is still unstable or in-dev keeps living, and its
*generic pattern* [raw-group escape hatch, fail-loud strict-field conventions, label/path
validation] is what every subsequent round of development keeps reusing, including whatever comes
after the piece that just graduated. The crate doesn't empty out and retire once something
graduates from it - it's a standing incubator, not a one-shot staging area.

**B is the standing default for anything not yet graduated, not just one of three equal
choices.** This stays a single crate indefinitely - no sibling extension crates, no promotion of
internal plumbing [`RawGroupAccess` and friends] into a shared location - until the trigger
described under C actually occurs. Ordinary schema work [new fields, bug fixes, doc passes - the
bulk of what this crate has seen so far] keeps merging straight to this crate's own `main` the
same way it always has; capture consumes that `main` directly via its path dependency and doesn't
need core involved at all.

**Collision avoidance between two in-dev features is a branching discipline, not a crate-split
decision.** If two schema changes being worked concurrently could collide [overlapping fields,
incompatible reworks of the same struct], develop them on separate branches of this one crate
rather than reaching for a second crate - a crate split doesn't buy isolation that branches don't
already give for free, and it would mean duplicating `RawGroupAccess` and standing up a second
CI/versioning setup for no real benefit.

**"Integrate via a full facade version iteration" [Option A] is reserved for genuinely
irreconcilable in-flight reworks**, not a blanket gate on every collision. Two substantively
incompatible redesigns of the same struct need a human design decision regardless of process -
graduating to core forces that decision to happen anyway, so it's the natural reconciliation
point for that specific case. Routine parallel feature work should still reconcile the ordinary
way [rebase/merge once both are feature-complete] without a mandatory trip through core's codegen,
keeping core graduation decoupled from, and much rarer than, this crate's own release cadence.

## Option C's actual trigger, stated concretely

Option C [below] is specifically **a wholly new
extension domain unrelated to this schema** [example: an MQTT subsystem], at the point real
buildout for supporting N-many independent extensions actually begins. That's the moment
`RawGroupAccess` [and any other genuinely-generic plumbing currently living in this crate's own
source] is worth promoting into core or a small shared support crate - a one-time cost paid once
that trigger is real, not something to front-load speculatively now.

## A. Roll into MCL core (replace the stale facade)

**Pros**: one schema, one code path — closes the overlap question this session deliberately
parked; automatic Python/Node/C++/Go parity for free via the existing codegen pipeline, which
matters the moment a non-Rust consumer genuinely needs this data.

**Cons**: reintroduces exactly the iteration cost this crate was built to avoid. Every future
schema tweak would flow through `generate_capabilities.py`'s templates and need re-validation in
four languages — even though today's only real consumer (capture) is Rust-only and permanently
keeps its own independent reader regardless of what core offers, meaning rolling in wouldn't
change capture's own behavior at all, only what's offered to hypothetical other consumers.

**Justified when**: the schema is genuinely stable (not mid-iteration — see
[ARCHITECTURE_RATIONALE.md](./ARCHITECTURE_RATIONALE.md) for development progression context)
**or** a concrete multi-language consumer need exists or is imminent — a Python or
C++ tool that needs direct Pipe/Client/Trigger read/write.

## B. Stay as an extension crate inside Machine_Config_Library (status quo)

**Pros**: fastest iteration — every fix this session was one `cargo check` away, no
publish/version-bump/re-vendor cycle; smallest possible core footprint (the raw-group escape
hatch, ~5 methods, nothing else in core needs to change); Rust-only cost matches Rust-only
reality, since capture — the only real consumer today — permanently keeps its own independent
reader regardless of what this crate or core offer.

**Cons**: no established path to a second-language consumer. If a non-Rust tool ever needs this
schema, there's no codegen to lean on the way core's facade has — porting would be a manual,
from-scratch reimplementation.

**Justified when**: the schema is still actively moving, only one language consumer
exists, and the "separate authoring tool" / "third-party adopter" use cases mentioned in
[README.md](./README.md) remain future potential rather than active deliverables. Staying
here costs nothing extra as long as those stay true.

## C. Split into its own crate/repo - triggered only by a wholly new extension domain

**Pros**: the cleanest separation if third-party adoption becomes a real, near-term goal — an
external party can depend on, fork, or audit a standalone crate far more easily than something
nested in `Machine_Config_Library`'s workspace with a path dependency on MCL core
(`machine-config = { path = "../rust" }` doesn't resolve across repos at all; a separate repo
would need either a git dependency pinned to a specific MCL commit/tag, or MCL core publishing
somewhere a separate repo can actually pull from).

**Cons**: loses the cheap, same-workspace iteration this whole session relied on; needs its own
CI, its own versioning/release discipline, and a real answer to the cross-repo dependency
question above before it's viable at all.

**Justified when either**:

- the external-producer-utility or third-party-adopter use case turns into a concrete,
  scheduled deliverable [not just a stated possibility] - the cross-repo dependency cost is
  worth paying once someone outside this workspace actually needs to consume it, or
- **a wholly new, unrelated extension domain needs to exist** [the stated example: MQTT] and
  real buildout for supporting N-many independent extensions begins - see "Option C's actual
  trigger, stated concretely" above. Not triggered by ordinary parallelism within this same
  schema; that's a branching-discipline question, not a crate-split one.

## The three-way framing, stated directly

- **Stabilizing for the foreseeable future, and multi-language support is actually required** →
  **A**. Pay the parity tax once the schema has stopped moving and a real non-Rust consumer
  exists — that's exactly what core's codegen is for.
- **Keeping it flexible and Rust-scoped** → **B**. No reason to pay the codegen tax for a schema
  still finding its shape, serving a single-language consumer.
- **A wholly new, unrelated extension domain needs real support** [e.g. MQTT] → **C**. Not
  triggered by this schema's own stability, language needs, or by ordinary in-crate parallelism
  [see "Current direction" above - that's a branching question] - only by the point where
  N-many genuinely distinct extensions are actually being built, and the raw-group pattern
  itself is worth giving a shared home separate from both core's release cadence and from being
  a one-off buried in this workspace.
