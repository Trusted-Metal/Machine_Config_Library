# Viewer/Library Integration — Implementation Plan

**Status: not started. No code has been written for this plan.**

**Purpose of this document:** this plan is written to be picked up cold, by anyone, with no
memory of the conversation that produced it. If you're reading this months later with no other
context, everything you need to understand *why* this exists, *what* "done" looks like, and *how*
to get there should be here. Where this doc references specific line numbers or file contents,
treat them as accurate as of 2026-09-11 — re-verify against the current source before relying on
them, since the library has continued to change since this was written.

---

## Contents

- [1. Why this exists](#1-why-this-exists)
- [2. Current state of the viewer](#2-current-state-of-the-viewer)
- [3. Goal](#3-goal)
- [4. Key feasibility finding: shared HDF5 backend](#4-key-feasibility-finding-shared-hdf5-backend)
- [5. The one real open risk — verify this first](#5-the-one-real-open-risk--verify-this-first)
- [6. Scope decisions to make before starting](#6-scope-decisions-to-make-before-starting)
- [7. Complete field-mapping reference](#7-complete-field-mapping-reference)
- [8. Step-by-step implementation plan](#8-step-by-step-implementation-plan)
- [9. Testing plan](#9-testing-plan)
- [10. Rebuild-on-library-change policy](#10-rebuild-on-library-change-policy)
- [11. Effort estimate ("the lift")](#11-effort-estimate-the-lift)
- [12. Risks and open questions](#12-risks-and-open-questions)

---

## 1. Why this exists

`viewer/machine_config_viewer.html` is a single self-contained HTML file (~1500 lines, all
CSS/JS inline) that lets someone drag-and-drop a machine config `.h5` file, a CMM measurement
`.csv`, and/or a build-plate `.3mf` file, and see them overlaid on a 2D canvas — build plate,
scanner fields, gas-flow/recoat direction arrows, CMM measurement dots with pass/fail coloring,
etc.

It was investigated (2026-09-11) while working on `RECOATER_BLADE_TYPE_PLAN.md` (adding a new
optional `Machine` field), to answer: *"does this viewer read the new field automatically, since
it should just work with any MCF file?"* The answer was no, for a structural reason bigger than
one missing field — **the viewer does not use the `machine-config` library's API in any
language. It has its own independent, hand-rolled HDF5 attribute reader**, hardcoded against the
v1.0 on-disk layout. Consequently:

- It does not know about `File_Version`/`schema_version` at all — no branching on version anywhere.
- Every attribute lookup silently falls back to a default on failure (see `attr()` in the source)
  — a renamed/relocated/missing attribute doesn't error, it just quietly renders as blank/default
  in the UI. There is no visible signal that something didn't map.
  Missing fields are not intended to be visible — data may simply be absent for a given file, and treated no differently in the UI, silently.
- It happened to still work against the one real version bump that exists (v1.0 → v1.1) only
  because none of v1.1's actual documented changes (`docs/migrations/v1_0_to_v1_1.md` — ClearBox
  relocation to `/Extensions/ClearBox/`, OPCUA relocation to `/Extensions/TM_OPCUA`,
  power-characterization restructuring, axis `Tuning_Parameters`/`Tuning_Type` removal) touch the
  narrow slice of ~20 attributes this viewer happens to read. That's incidental, not designed.
- The new `recoater_blade_type` field (or any future field/attribute rename) is invisible to it
  and always will be, until someone hand-edits this HTML file again to add another hardcoded
  attribute lookup — the same manual-sync burden this plan exists to eliminate.

This plan is the alternative to that manual-sync-forever status quo: wire the viewer up to the
actual library, so it inherits correct, version-aware reading for free, the same way every other
consumer of `machine-config` does.

**This work was deliberately excluded from `RECOATER_BLADE_TYPE_PLAN.md`** (see that plan's Step
9) — it's a separate, materially larger, architecturally distinct effort, not a natural part of
"add one optional field."

---

## 2. Current state of the viewer

All of this lives in one file: `viewer/machine_config_viewer.html`.

- **No build step, no dependencies to install.** Two external libraries are loaded from a CDN
  at runtime, inside a `<script>` tag / via dynamic `loadScript()`:
  - `jszip` (`https://cdnjs.cloudflare.com/ajax/libs/jszip/3.10.1/jszip.min.js`) — for `.3mf`
    parsing (a 3MF file is a zip archive).
  - `h5wasm` (`https://cdn.jsdelivr.net/npm/h5wasm@0.10.3/dist/iife/h5wasm.js`) — a WebAssembly
    build of the HDF5 C library, used to open `.h5` files directly in the browser.
  - **Important nuance for later**: this means the current viewer is *not* actually usable fully
    offline today — it needs internet access the first time it's opened, to fetch these two
    scripts. It is not a worse starting point than what this plan would produce; if anything,
    this plan could make it *more* self-contained (see §6).
- **`readH5(f)`** (around line 687 of the current file) is the hand-rolled reader. It:
  1. Reads root-level file attributes directly (`machine_name`, `manufacturer`, `model`,
     `serial_number`, `Export_Date`) via a local `attr(node, key, default)` helper that unwraps
     h5wasm's `{value: ...}` attribute wrapper and falls back to `default` on any failure.
  2. Opens the `Machine` group and reads `Build_Plate_X_Dimension`, `Build_Plate_Y_Dimension`,
     `Build_Plate_Corner_Radius`, `Gas_Flow_Direction`, `Recoat_Direction` directly.
  3. Walks `Machine/Optical_Trains/Optical_Train_01` through `_06` (stopping at the first
     missing one) and reads per-train scalars (`Beam_Waist_Major/Minor`, `M2_Major/Minor`),
     then descends into each train's `Light_Source` and `Scanner` subgroups for a handful more
     scalars (see §7 for the full list).
  4. Everything else in the schema (ClearBox/`optional_components`, `scan_field_correction_file`,
     collimator, scanner_card, OPCUA, axis subgroups, power characterization, synchronous
     sensors, correction data grids) is **not read at all** — the viewer only ever cared about a
     small subset relevant to the 2D layout visualization.
- **The viewer is read-only.** There is no "save"/"export"/"write" feature anywhere in the file —
  confirmed by grepping for any use of a writer; none exists. This matters a lot for scoping
  (§6.2) — the library's `MachineConfigWriter` need not be part of this integration at all unless
  a future product requirement adds editing.
- CSV and 3MF handling (`handleCSV`, `handle3MF`, `parseCSV`, `parse3MFTopFace`) are entirely
  unrelated to the `machine-config` library and are out of scope for this plan — leave them as-is.

---

## 3. Goal

Replace the viewer's hand-rolled `readH5()` (and the raw-attribute-name field access it feeds
downstream, throughout `updateMachineUI()`, `render()`, `drawScanners()`, `drawArrow()`,
`getCSVOffset()`, `populateCSVLaserSelector()`, `resetView()`, etc.) with calls into a real build
of the `machine-config` library (the Node.js/TypeScript implementation, per §4), so that:

- Any current or future `File_Version` the library supports is read correctly, automatically,
  with no viewer-side code change required for version support alone.
- Field additions/renames in the schema (like `recoater_blade_type`) become a one-line UI change
  (add a display row bound to the already-correct model field) instead of also requiring a new
  hand-written HDF5 attribute lookup.
- The viewer's data layer becomes "call the reader, get the same JSON shape every other language
  produces" — the exact shape documented in `docs/schema.md`.

**Non-goals** (explicitly out of scope unless a future decision changes this):
- Adding write/export/edit capability to the viewer (it has none today).
- Changing the viewer's visual design, layout, or feature set (CMM overlay, 3MF overlay, etc.).
- Publishing the `machine-config` npm package (it's `"private": true` today — see §6.1).

---

## 4. Key feasibility finding: shared HDF5 backend

This is the single most important thing that makes this plan *plausible* rather than a rewrite
from scratch: **the Node.js implementation of `machine-config` already uses `h5wasm`** — the
exact same WebAssembly HDF5 library the viewer already loads from a CDN today.

Checked in `nodejs/package.json`:
```json
"dependencies": {
  "ajv": "^8.17.1",
  "commander": "^12.1.0",
  "h5wasm": "^0.10.3"
}
```

And the library's Node-specific surface (everything that isn't pure portable TS/JS logic) is
narrow. Grepping `nodejs/src/` for any `node:*` import or `require()` found exactly these files:

| File | What it needs from Node | Needed for the viewer's read-only use case? |
|---|---|---|
| `capabilities/file_version.ts` | `node:path`'s `resolve()`, `h5wasm/node` | Yes — used by every read |
| `capabilities/v1_0/hdf5.ts` | same | Yes |
| `capabilities/v1_1/hdf5.ts` | same | Yes |
| `capabilities/v1_0/writer.ts` | same | **No** — write path, out of scope (§3) |
| `capabilities/v1_1/writer.ts` | same | **No** — write path, out of scope |
| `cli.ts` | `node:module` (`createRequire`, for reading `package.json`'s version) | **No** — CLI entry point, not imported by the reader |
| `schema.ts` | `node:fs` (`readFileSync`), `node:path`, `node:url`, `node:module` | **No** — only used for JSON-schema *validation*, which the viewer does not currently do (see §6.3) |

`reader.ts` (the actual `MachineConfigReader` public class) imports only
`capabilities/file_version.ts`, `capabilities/v1_0/hdf5.ts`, and `capabilities/v1_1/hdf5.ts` —
**it does not import anything from the writer files at all**. This means bundling just the read
path is a naturally small, already-decoupled slice of the codebase; no risk of accidentally
pulling in write-path code through a shared barrel import, *as long as the build imports
`reader.js` directly rather than the whole-package `index.ts` barrel* (verify this holds at
build time — barrel files can accidentally re-export things that pull in unwanted imports).

So the only two real Node-isms standing in the way of a browser build are:
1. `node:path`'s `resolve()` — called as `resolve(path).replace(/\\/g, '/')` purely to
   normalize a path string before handing it to h5wasm. Trivial to replace with a no-op/identity
   function for browser use, since the viewer already works with already-normalized virtual paths
   (see §5).
2. The `h5wasm/node` import specifier itself — see §5, this is the one thing that needs to be
   *verified*, not just assumed.

---

## 5. The one real open risk — verify this first

**Do this before committing significant time to the rest of this plan.** It's the one place
where "this looks easy" could turn out to be wrong, and it's cheap to check.

`h5wasm`'s own `package.json` (`nodejs/node_modules/h5wasm/package.json`, v0.10.3) declares three
separate build outputs:

```json
"main": "./dist/iife/hdf5_hl.js",
"module": "./dist/esm/hdf5_hl.js",
"browser": "./dist/iife/h5wasm.js",
"exports": {
  ".": { "import": "./dist/esm/hdf5_hl.js" },
  "./node": { "import": "./dist/node/hdf5_hl.js" }
}
```

The library's source imports `h5wasm/node` (the `./dist/node/hdf5_hl.js` build) everywhere. The
viewer today loads `./dist/iife/h5wasm.js` (the `browser` field's build) directly from a CDN,
via a plain `<script>` tag — no import specifier resolution involved at all.

Per `h5wasm`'s own build script (`"build_node": "tsc src/hdf5_hl.ts --outDir dist/node ..."`
vs. `"build_esm": "tsc src/hdf5_hl.ts --outDir dist/esm ..."`) — **both the Node and ESM/browser
builds are compiled from the exact same source file**, just with slightly different `tsc` flags
(`--strict`, `--allowJs`, `--esModuleInterop`). This strongly suggests the runtime API surface
(`h5wasm.File`, `h5wasm.Group`, `h5wasm.ready`, `FS.writeFile`, `FS.mkdir`, attribute wrappers,
etc.) is identical or near-identical between the two builds — which is exactly what would make
"swap the import specifier during bundling" (via your bundler's `alias` config, e.g. esbuild's
`--alias:h5wasm/node=h5wasm` or a Vite `resolve.alias` entry) sufficient on its own.

**But this has not been empirically verified.** The two builds could still differ in how they
locate/load the actual `.wasm` binary (Node builds commonly read it from disk via `fs`; browser
builds commonly `fetch()` it or expect it inlined) — which is precisely the kind of difference
that "same TypeScript source, different `tsc` flags" would *not* protect against, since that
loading logic likely lives outside the parts of the source that differ only by strictness flags.

**Step 0, before anything else in §8**: write a five-line spike — alias `h5wasm/node` → `h5wasm`
(the ESM/browser build) in an esbuild config, bundle just `nodejs/src/reader.ts`, load the
result in a browser page, and try `new MachineConfigReader('/work/test.h5').parse()` against a
file written into h5wasm's virtual FS exactly the way the current viewer already does it
(`FS.writeFile('/work/test.h5', bytes)`). If this works, the rest of this plan is low-risk. If it
doesn't, the fallback is deeper: either patch the adapter files to use a bundler-provided
abstraction over `h5wasm/node` vs `h5wasm`, or (worse case) fork/vendor a small shim.

---

## 6. Scope decisions to make before starting

These are product/design decisions, not technical ones — whoever picks this up should decide
these explicitly (with whoever originally asked for this, if they're still reachable) rather than
assuming an answer.

### 6.1 Distribution shape — how does the built artifact get to end users?

`machine-config-library`'s `package.json` has `"private": true` — it is **not published to
npm**, so there is no CDN-import shortcut (no `unpkg`/`esm.sh` URL to point a `<script>` tag at).
Whatever gets built has to be built from this repo's own source and shipped alongside/inside the
viewer HTML. Two shapes, not mutually exclusive (could ship both):

- **(A) Same shape as today**: one `.html` file that still fetches `h5wasm` from a CDN on first
  open (matching current behavior for both `jszip` and `h5wasm`). The bundled
  `machine-config`-reader code (a much smaller amount of JS — just reader/model/adapter logic,
  no WASM binary of its own since it reuses the same `h5wasm` instance) gets inlined into the
  same `<script>` block, or loaded as a sibling `.js` file shipped next to the `.html`.
- **(B) Fully self-contained / offline-capable**: bundle *everything* including the `h5wasm`
  WebAssembly binary itself (base64-inlined, a well-established technique — esbuild's `--loader:
  .wasm=binary` or similar, then embed as a data URI or inline `Uint8Array`) directly into one
  `.html` file with zero network calls ever. This is **more** portable than the current viewer,
  not less — works fully offline, on an air-gapped machine, indefinitely.

Recommendation if no other constraint emerges: **(B)**, since it strictly improves on today's
portability story (no network dependency at all) at the cost of a larger single file and a
slightly more involved build config. But this is a judgment call for whoever picks this up,
informed by how/where this viewer is actually distributed and used in practice (email
attachment? shared network drive? internal tool page?) — information this planning session
didn't have.

### 6.2 Read-only vs. read+write

The current viewer has no write/export feature. Recommendation: **keep it read-only** for this
integration — bundle only `MachineConfigReader` (§4 already confirms this is naturally
decoupled from the writer code). If a future requirement adds an editing/export feature to the
viewer, that would be a separate, later effort to bundle `MachineConfigWriter` too (which *would*
need the same "does `h5wasm/node` vs `h5wasm` matter" question re-verified for the writer path
specifically, since writing involves more filesystem-adjacent operations than reading).

### 6.3 Schema validation — bring it along or not?

The viewer today does not validate anything against the JSON schema — it just reads whatever
attributes are present. `schema.ts` (§4's table) needs `node:fs` to load the bundled
`machine_config_v1.schema.json` at runtime; this is easy to work around (inline the schema JSON
as a bundled JS import instead of a runtime file read — bundlers handle `import data from
'./schema.json'` natively), but there's no evident product need for the viewer to validate
anything, so this is presented as an option, not a requirement. **Recommendation: skip it** —
leaves one less thing to port, and validation failures on the write path (which is what schema
validation protects against; see `docs/schema.md`) aren't relevant to a read-only viewer.

---

## 7. Complete field-mapping reference

Every field the current `readH5()` reads, with its current hardcoded HDF5 path, and the
equivalent path in the library's stable JSON model (per `docs/schema.md` — re-verify against that
doc if it has changed since this was written). This table is the actual rewrite work for
`readH5()` and its downstream consumers.

| Current viewer field | Current hardcoded HDF5 read | Library model path (via `MachineConfigReader.parse()`/`.toJson()`) |
|---|---|---|
| `cfg.machineName` | root attr `machine_name` | `meta.machine_name` |
| `cfg.manufacturer` | root attr `manufacturer` | `meta.manufacturer` |
| `cfg.model` | root attr `model` | `meta.model` |
| `cfg.serial` | root attr `serial_number` | `meta.serial_number` |
| `cfg.exportDate` | root attr `Export_Date` | `meta.export_date` |
| `cfg.plateDimX` | `Machine/Build_Plate_X_Dimension` | `machine.build_plate_x` |
| `cfg.plateDimY` | `Machine/Build_Plate_Y_Dimension` | `machine.build_plate_y` |
| `cfg.plateRadius` | `Machine/Build_Plate_Corner_Radius` | `machine.build_plate_radius` |
| `cfg.gasFlow` | `Machine/Gas_Flow_Direction` | `machine.gas_flow_direction` |
| `cfg.recoat` | `Machine/Recoat_Direction` | `machine.recoat_direction` |
| *(not read today)* | — | `machine.recoater_blade_type` — free addition once this plan lands |
| `laser.beamMajor` | `.../Optical_Train_0N/Beam_Waist_Major` | `optical_trains[i].beam_waist_major` |
| `laser.beamMinor` | `.../Optical_Train_0N/Beam_Waist_Minor` | `optical_trains[i].beam_waist_minor` |
| `laser.m2Major` | `.../Optical_Train_0N/M2_Major` | `optical_trains[i].m2_major` |
| `laser.m2Minor` | `.../Optical_Train_0N/M2_Minor` | `optical_trains[i].m2_minor` |
| `laser.lsManufacturer` | `.../Light_Source/Manufacturer` | `optical_trains[i].light_source.manufacturer` |
| `laser.lsModel` | `.../Light_Source/Model` | `optical_trains[i].light_source.model` |
| `laser.powerMax` | `.../Light_Source/Power_Max_Nominal` | `optical_trains[i].light_source.power_max_nominal` |
| `laser.wavelength` | `.../Light_Source/Light_Wavelength` | `optical_trains[i].light_source.wavelength` |
| `laser.scanFieldX` | `.../Scanner/Scan_Field_Size_X` | `optical_trains[i].scanner.scan_field_x` |
| `laser.scanFieldY` | `.../Scanner/Scan_Field_Size_Y` | `optical_trains[i].scanner.scan_field_y` |
| `laser.headOffsetX` | `.../Scanner/Scan_Head_Offset_X` | `optical_trains[i].scanner.scan_head_offset_x` |
| `laser.headOffsetY` | `.../Scanner/Scan_Head_Offset_Y` | `optical_trains[i].scanner.scan_head_offset_y` |
| `laser.headRotation` | `.../Scanner/Scan_Head_Rotation` | `optical_trains[i].scanner.scan_head_rotation` |
| `laser.scanModel` | `.../Scanner/Model` | `optical_trains[i].scanner.model` |
| `laser.workDist` | `.../Scanner/Working_Distance` | `optical_trains[i].scanner.working_distance` |

Every one of these needs its call site updated in `readH5()`, and — since the shape changes from
"flat scalar object" to "the library's nested model" — every downstream reader of `S.config.*`
needs re-checking too: `render()`, `resetView()`, `drawScanners()`, `drawArrow()`,
`updateMachineUI()`, `populateCSVLaserSelector()`, `getCSVOffset()`, `updateCSVOffsetInfo()`. None
of these need new *logic*, just updated property-access paths to match the new shape (e.g.
`S.config.gasFlow` → `S.config.machine.gas_flow_direction`, `laser.headOffsetX` →
`train.scanner.scan_head_offset_x`).

The `optional_components.clearbox` (and everything else the viewer never touches today —
collimator, scanner_card, axis subgroups, OPCUA, correction grids, synchronous sensors,
power characterization) is out of scope: keep the viewer's current feature set exactly as-is,
this plan only changes *how* the existing fields get read, not what's displayed. That could
change as a follow-up idea after this lands but isn't part of this plan.

---

## 8. Step-by-step implementation plan

Sequenced so that each step produces something checkable before moving to the next.

### Step 0 — De-risk the h5wasm question (§5)

- [ ] Write a standalone spike (throwaway, not part of the final deliverable): bundle
      `nodejs/src/reader.ts` with esbuild, aliasing `h5wasm/node` → `h5wasm`'s ESM build.
- [ ] Load the bundle in a plain browser test page, write a small `.h5` fixture (e.g.
      `fixtures/synthetic_2laser.h5`) into `h5wasm`'s virtual FS the same way the current viewer
      does, and call `new MachineConfigReader('/work/test.h5').parse()`.
- [ ] **Decision gate**: if this works cleanly, proceed with confidence. If it throws or behaves
      differently, stop and investigate *why* before continuing — this could change the shape of
      every step below.

### Step 1 — Choose and set up build tooling

- [ ] Pick a bundler. **esbuild** is the natural fit here (already a dev dependency of `h5wasm`
      itself; fast; simple single-command bundling; good `alias`/`define`/`loader` support for
      exactly the WASM-inlining and module-aliasing needs of this project) — but re-evaluate if
      the person implementing this has a strong reason to prefer Vite/webpack/rollup instead.
- [ ] Decide where the build config and output live — e.g. a new `viewer/build/` directory with
      its own minimal `package.json` + `esbuild.config.mjs`, kept separate from `nodejs/`'s own
      build so the viewer's build doesn't entangle with the library's own npm package lifecycle.
- [ ] Configure the bundler to:
      - Alias `h5wasm/node` → `h5wasm` (pending Step 0's confirmation of the right target)
      - Alias/shim `node:path`'s `resolve` to an identity function (or patch `normPath()` in
        `file_version.ts` to skip `resolve()` in a browser build — whichever is less invasive to
        the shared library source; prefer the bundler-side alias to avoid forking library code)
      - Entry point: a small new file (e.g. `viewer/build/entry.ts`) that imports and re-exports
        only `MachineConfigReader` from `nodejs/src/reader.ts` — not the whole package barrel
        (`nodejs/src/index.ts`), to avoid accidentally pulling in writer/CLI/schema code (verify
        this holds; barrel files can have surprising re-export side effects).
      - Output format: decide based on §6.1's distribution-shape decision — a single bundled
        `.js` (for shape A, loaded via `<script>` alongside the existing CDN scripts) or a
        fully-inlined bundle with the `.wasm` binary embedded (for shape B).

### Step 2 — Get a minimal successful build

- [ ] Produce a working bundle exposing `MachineConfigReader` as a global or module the viewer's
      existing inline `<script>` can call.
- [ ] Confirm bundle size is reasonable (the actual reader/model/adapter code should be modest —
      a few hundred KB uncompressed at most; the WASM binary, if inlined per shape B, is the
      dominant size contributor and is unavoidable either way since the current viewer already
      loads it, just from a CDN instead of inline).

### Step 3 — Rewrite the viewer's data-access layer

- [ ] Replace `readH5()`'s body: instead of manually walking HDF5 groups/attrs, call
      `new MachineConfigReader(virtualPath).parse()` (or `.toJson()` if a plain JSON object is
      more convenient for the existing rendering code) and store the result in `S.config`.
- [ ] Using §7's mapping table, update every downstream reference from the old flat
      `cfg.gasFlow`/`laser.headOffsetX`-style names to the library's nested model paths
      (`machine.gas_flow_direction`, `optical_trains[i].scanner.scan_head_offset_x`, etc.) in:
      `render()`, `resetView()`, `drawScanners()`, `drawArrow()`, `updateMachineUI()`,
      `populateCSVLaserSelector()`, `getCSVOffset()`, `updateCSVOffsetInfo()`.
- [ ] Note the model uses 0-indexed `optical_trains` arrays, while the current code iterates
      `Optical_Train_01`...`_06` with 1-indexed naming and a "stop at first missing" loop — the
      library's `parse()` already returns exactly the trains present, in order, so this becomes
      simpler (`config.optical_trains.forEach(...)`), not harder.
- [ ] `MachineConfigReader.parse()` is `async` (returns a `Promise`) — confirm `handleH5()` (which
      is already `async`) awaits it correctly; this should be a small, mechanical change since
      `handleH5` already does async work (`ensureH5Wasm()`, `file.arrayBuffer()`).

### Step 4 — Wire the bundle into the actual viewer file

- [ ] Depending on §6.1's decision: either inline the bundle's JS directly into
      `machine_config_viewer.html`'s `<script>` block (shape A/B with everything in one file), or
      add a `<script src="machine_config_viewer.bundle.js">` tag pointing at a sibling file
      shipped alongside the HTML (a two-file distribution — still fully self-contained, just not
      a single file).
- [ ] If shape B (fully offline) was chosen, remove the CDN `<script>` tag for `h5wasm` (no
      longer needed — it's now inside the bundle) but keep `jszip`'s CDN tag as-is (still needed
      for `.3mf` parsing, out of scope for this plan) — or bundle `jszip` too, as a nice-to-have
      that isn't required for this plan's goal.

### Step 5 — Validate against real fixtures

See §9.

### Step 6 — Documentation

- [ ] Add a short section to `docs/` (or a comment block at the top of the viewer HTML, or both)
      explaining the build step now required to regenerate the bundle, and where its source
      lives, so this doesn't become an unmaintainable "nobody remembers how this file is built"
      situation the way the *current* hand-rolled reader arguably already is.
- [ ] Record the actual chosen distribution shape (§6.1) and rationale, once decided.

---

## 9. Testing plan

There is no existing automated test coverage for the viewer at all (it's not part of
`tools/cross_check.py` or any language's test suite) — this plan doesn't need to invent a full
automated UI test suite, but should establish *some* repeatable verification:

- [ ] **Manual verification against every committed fixture** in `fixtures/`: load each of
      `reference_config.h5`, `reference_config_v1_1.h5`, `synthetic_2laser.h5`, and (if relevant
      fields are ever added to the viewer later) the OPCUA/synchronous-sensor fixtures — confirm
      the sidebar's Machine/Optical Trains panels show the same values the fixture is known to
      contain (cross-reference `fixtures/reference_output.json` for exact expected values).
- [ ] **The actual point of this whole effort**: confirm `reference_config_v1_1.h5` (a real
      v1.1-versioned file) renders correctly through the *same* code path as v1.0 files, with no
      viewer-side special-casing — this is the capability the hand-rolled reader never had.
- [ ] If Step 0's spike revealed any behavioral difference between `h5wasm/node` and the
      browser build, add a regression check specifically for whatever that difference was.
- [ ] Cross-browser sanity check (Chrome/Firefox/Edge at minimum) — WASM + virtual filesystem
      behavior can vary subtly across engines; the current viewer has presumably already been
      informally tested this way, so this is a re-confirmation, not new ground.
- [ ] If shape B (fully offline) was chosen: verify the built HTML file genuinely works with the
      browser's network access disabled/airplane-mode, to prove the offline claim rather than
      assume it.

---

## 10. Rebuild-on-library-change policy

**Short answer: yes, conceptually** — like any bundled/compiled artifact, the viewer's JS is a
frozen snapshot of the library's source at build time. A change to the library does not
automatically appear in an already-built viewer file; someone has to re-run the build and
redistribute the new file.

**The nuance worth capturing here**, since it affects how *often* this actually needs to happen:

- Only changes to the **bundled subset** matter — per §4/§6.2, that's `reader.ts` +
  `capabilities/file_version.ts` + `capabilities/v1_0/hdf5.ts` + `capabilities/v1_1/hdf5.ts` +
  `models.ts` + `capabilities/*/layout.ts` (the read path only). A bug fix or feature added to
  `writer.ts`, `builder.ts`, the CLI, or schema validation would **not** require rebuilding the
  viewer at all, since none of that code is part of its bundle.
- A **new field added to an existing version** (like this session's `recoater_blade_type` work)
  *would* require a viewer rebuild to show up in the UI — but only the UI-binding half (§7's
  table gains a row); the reading half is already correct with zero changes, because the bundled
  reader parses whatever the schema defines, generically. This is the entire point of doing this
  work: today, every single new field requires a hand-edit to the viewer's HTML; after this,
  most new fields require zero reader-side changes and, if you want to actually *display* the new
  field, one new UI line plus a rebuild — never a new hand-written HDF5 attribute lookup.
- A **new `File_Version`** (a hypothetical future v1.2) would need its own new adapter added to
  the library first (per `docs/contributing.md`'s existing "Adding a new file version" checklist)
  — and *then* a viewer rebuild to pick up that adapter. Recommendation: add "rebuild and
  redistribute the viewer bundle" as a line item to that existing release checklist once this
  plan lands, so it isn't forgotten the next time a new version ships. Until it's rebuilt, files
  written in a version newer than what the bundle knows about would throw
  `UnsupportedFileVersion` — a real, visible, correct error (unlike the current silent-blank-field
  failure mode), which is itself a strict improvement even in the "someone forgot to rebuild"
  case.
- Recommendation: **rebuild on release, not on every commit** — tie it to whatever cadence new
  `File_Version`s or schema fields actually ship at (which, per this repo's git history, is not
  especially frequent), not to CI running on every PR.

---

## 11. Effort estimate ("the lift")

Rough, and explicitly contingent on Step 0 not surfacing a blocker:

| Phase | Estimate |
|---|---|
| Step 0 — de-risk spike | 0.5–1 day |
| Step 1–2 — bundler setup, minimal working build | 0.5–1 day |
| Step 3 — rewrite viewer data-access layer (§7's ~24 field call sites plus their downstream consumers across ~8 functions) | 1–2 days |
| Step 4 — wire into final distribution shape (incl. WASM-inlining if shape B) | 0.5–1 day |
| Step 5/9 — testing across fixtures and browsers | 0.5–1 day |
| **Total** | **roughly 3–6 developer-days** |

This assumes one person, already familiar with this codebase, working through it sequentially.
If Step 0 reveals `h5wasm/node` and the browser build genuinely diverge in how they load the WASM
binary (the one real technical unknown, per §5), add meaningful time for either patching around
that difference or escalating to the `h5wasm` project/its issue tracker — this could add days,
not hours, in the worst case, which is exactly why Step 0 is sequenced first and cheap.

---

## 12. Risks and open questions

- **(Primary risk, see §5)** `h5wasm/node` vs. the browser/ESM build may not be drop-in
  interchangeable. Unverified as of this writing.
- **Distribution shape (§6.1)** is a product decision this planning session had no way to
  resolve — needs input from whoever actually distributes/uses this viewer in practice.
- **`resolve()`'s behavior**: the current `normPath()` does `resolve(p).replace(/\\/g, '/')` —
  in Node, `resolve()` turns a relative path into an absolute filesystem path from `cwd`. In a
  browser bundle, there is no meaningful `cwd`; if the viewer always passes an already-absolute
  virtual path (e.g. `/work/cfg.h5`, matching current behavior), an identity-function shim is
  almost certainly sufficient — but verify this assumption doesn't silently break on a
  differently-shaped path in some edge case (e.g. a relative virtual path).
  - Additional possible resolution beyond a stub: `node:path`'s `resolve` logic for a POSIX-style
    already-absolute path is simple enough that a tiny hand-written polyfill (a few lines) could
    replace the shim if the identity-function approach turns out to be insufficient in some edge
    case — cheap enough to try either way.
- **Bundle-size / performance**: not expected to be a real concern (§4, §11), but no one has
  actually measured the bundled reader's size yet — do so in Step 2 rather than assuming.
- **Whether to also bundle `jszip`** for full offline `.3mf` support — orthogonal nice-to-have,
  not required for this plan's stated goal (the machine-config library integration), but cheap
  to bundle alongside if shape B is chosen and someone wants full offline parity for every
  feature, not just H5 loading.
