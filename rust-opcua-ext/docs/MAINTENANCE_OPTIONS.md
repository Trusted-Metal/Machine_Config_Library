# Where this crate goes from here

Three real options, not a ranked list — each is correct under a different set of conditions, and
the right call depends on which conditions actually hold.

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

## C. Promote to its own separate library/repo

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
  scheduled deliverable (not just a stated possibility) — the cross-repo dependency cost is
  worth paying once someone outside this workspace actually needs to consume it, or
- multiple genuinely distinct extension types accumulate (see the stabilizing-vs-flexible
  framing below) and the *pattern itself* — raw-group escape hatch + standalone crate — becomes
  something worth giving a shared home, separate from both core's release cadence and from
  being a one-off buried in this workspace.

## The three-way framing, stated directly

- **Stabilizing for the foreseeable future, and multi-language support is actually required** →
  **A**. Pay the parity tax once the schema has stopped moving and a real non-Rust consumer
  exists — that's exactly what core's codegen is for.
- **Keeping it flexible and Rust-scoped** → **B**. No reason to pay the codegen tax for a schema
  still finding its shape, serving a single-language consumer.
- **Multiple new, distinct extension types emerging, each wanting its own extension-equivalent
  crate** → **C**, but reframed: at that point the thing worth extracting isn't just this one
  schema, it's the *pattern* — raw-group escape hatch plus standalone crate — as a shared
  convention for an "MCL extension crates" family, potentially its own small workspace with
  `opcua-config-ext` as the first member. That's a different trigger than either A or B: it's
  not about this schema's own stability or language needs, it's about whether enough siblings
  have been mapped/added to a database/design pattern that one-off placement inside
  `Machine_Config_Library` stops making sense for any of them.
