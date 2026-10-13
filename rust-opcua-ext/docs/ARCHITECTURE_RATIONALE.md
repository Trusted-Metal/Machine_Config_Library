# Why an extension crate, not the core facade

MCL core already has an OPC UA subsystem: `OpcuaConfig`/`OpcuaTrigger`/`OpcuaPipeConfig`/
`OpcuaClientConfig` (`rust/src/models.rs`), surfaced through `get_opcua`/`set_opcua` on the
generated per-version `CapabilityFile` facade (`rust/src/capabilities/generated.rs`, templated
per-language by `generate_capabilities.py` across Python/Node/Rust/C++/Go). This is the "legacy
subsystem" referenced throughout this crate's docs and commit history - it still exists, still
compiles, and nothing here deletes it. The disposal question for it is explicitly parked until
an architectural checkpoint is reached and a merge discussion is tangible; see
[MAINTENANCE_OPTIONS.md](./MAINTENANCE_OPTIONS.md) for proposed options.

## What the extension approach buys, concretely

**Schema evolution without a 4-language parity tax.** The entire MCL-core change this whole
effort required was ~5 small methods (`get_raw_group`/`list_raw_group_children`/`set_raw_group`/
`ensure_group`/`write_json_attr`) added to two files (`v1_0/hdf5.rs`, `v1_1/hdf5.rs`). Every
schema change since then - the `Pipe`/`Client`/`Trigger` split, every field rename, the
bool-as-int decision, three separate reversals on label-fallback behavior - happened
entirely inside this crate, with zero `generate_capabilities.py` template edits and zero
re-validation across Python/Node/C++/Go. Had this schema lived in core from the start, each of
those iterations would have needed to flow through codegen and get re-checked in four languages
before being provably correct in any one of them.

**Fail-loud field strictness.**
The strict-vs-default categorization documented in [README.md](./README.md) was a deliberate
design pass: for `rule_enabled`/`condition_enabled` specifically, the call was made explicitly -
"a fail loud design might actually be preferable... an erroneously started capture would still
be better than unknown incapability of starting capture at all, at worst this means filling up
a hard drive, no effect on a physical system." The stale core facade's `OpcuaTrigger`/
`OpcuaConfig` types carry no equivalent distinction; they're flat generated structs with
whatever default-handling the codegen template happens to produce uniformly, not field-by-field
reasoning grounded in a real downstream consumer's actual fallback behavior.

**Orphan pruning.** `write_opcua_config`'s full-replace semantics (deleting a rule/condition
group that's no longer in the current write) is a real correctness property this session found
missing and fixed. Nothing analogous has been built against the stale facade's
`set_opcua`/`SetMode` path - there's no reason to believe the same bug doesn't exist there too,
since the two schemas' write paths were built independently.

**Label/path validation purpose-built for a schema no generic facade models.**
`rule_label`/`condition_label` become literal HDF5 group names, which creates a validation
surface (duplicate-label collision, `/` and NUL rejection) that doesn't exist for a facade built
around flat attributes on fixed, pre-named groups. The core codegen pipeline has no concept of
"this field's value is also a child group's name" at all.

## Why the extension was the right vehicle to validate this architecture

Capture permanently keeping its own independent hand-rolled reader (a settled, standing decision -
see [README.md](./README.md#producers-and-consumers)) meant there was never a forcing function
requiring this schema to go through the generated cross-language facade in the first place.
Nothing downstream actually needed `get_opcua`/`set_opcua` to carry this data for the schema to
be useful. So leaving this extension rust-scoped was sufficient for validating the design.

That mattered in practice: this schema was genuinely unsettled for most of this session.
Label-fallback behavior required settling and bool representation (string vs. int) was deliberately
deferred as a known-incomplete decision rather than over-engineered up front. None of that
back-and-forth would have been free to do against a codegen-backed, multi-language facade -
every reversal would have meant re-templating and re-validating in four languages before even
knowing whether the new shape was right. Building this as a small, independently-versioned crate
let the schema find its actual shape through iterative testing, cheaply, before any commitment to
the cross-language cost that rolling it into core would require.
