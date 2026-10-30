# ClearBox moved out of core

ClearBox is no longer part of core's typed `MachineConfig`. Core carries it as an opaque extension tree and the
`mcf-ext-clearbox` package (Node and Rust) packs and unpacks it. The file format is unchanged: files written
before and after read and write the same way.

## What changed in core

- `optical_trains[].optional_components` and the typed `ClearBox` model are gone in Python, Rust, Node, Go and C++.
- Both file versions expose ClearBox as `extensions["ClearBox"]`, always in the File_Version 1.1 layout: root attributes
  `Output_Path` and `Software_Trigger_Delay`, one group per train id, `Power_Characterization` as tables. For a 1.0 file
  core builds this tree from each train's `Optional_Components/ClearBox` and splits it back on write.
- Core's `power_characterization` helpers stay public (`LightSource` still uses them).

## Moving a consumer

Node:

```ts
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';
import { pack, unpack } from 'mcf-ext-clearbox'; // ESM only; REQUIRES_CONTRACT ^0.1.1

const config = await new MachineConfigReader(path).parse();
const tree = config.extensions?.['ClearBox'];
if (tree) {
  const r = unpack(tree);
  if (!r.ok) throw new Error(`${r.error.code} at ${r.error.path}: ${r.error.message}`);
  const clearBox = r.value; // units[], outputPath, softwareTriggerDelay
}

// writing
const packed = pack(model);
if (!packed.ok) throw new Error(packed.error.message);
config.extensions = { ...config.extensions, ClearBox: packed.value };
await new MachineConfigWriter(config, '1.1').write(out);
```

Rust: `mcf_ext_clearbox::{unpack, pack}` take and return `ExtensionTree`; the crate keeps its own copy of the tree
types, so convert through the contract JSON (`tree_to_json` on one side, `tree_from_json` on the other).

Python, Go and C++ have no ClearBox codec: they read and write the tree unchanged.

`rdt-core` (`api/endpoints/machines/clearbox-extension/`) is a worked example: a thin adapter that keeps its own
per-train shape and converts at the file boundary.

## Behaviour to know

- `Output_Path` and `Software_Trigger_Delay` are machine-wide: from a 1.0 file the first train that has one wins.
- A File_Version 1.0 file has no Extensions group, so ClearBox is the only extension it can carry.
- Writing to 1.0 drops what that layout cannot store: `Firmware_Version`, `Input_Type`, `Units_Derived_Quantity` and
  `Characterization_Points`. Power characterization is kept as the flat `Volts_To_Watts_*` fields. A 1.1 target is lossless.
- The codec writes integers as `i32` and reads any integer width. Absent strings are written as `""`.
- A ClearBox tree with a `Schema_Version` or any group the codec does not know is rejected with its path;
  core still keeps such a tree verbatim, so an older consumer never loses it.
