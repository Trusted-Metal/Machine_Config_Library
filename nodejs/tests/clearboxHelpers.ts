/**
 * Test helpers for reading ClearBox, which core exposes as an opaque extension tree in its
 * File_Version 1.1 layout for both file versions (see `src/clearboxLegacy.ts`).
 */
import type { ExtensionTree } from '../src/extensions/contract.js';
import { emptyExtensionTree } from '../src/extensions/contract.js';
import type { MachineConfig } from '../src/models.js';

export function trainKey(index: number): string {
  return `Optical_Train_${String(index + 1).padStart(2, '0')}`;
}

export function clearboxRoot(cfg: MachineConfig): ExtensionTree {
  const tree = cfg.extensions?.['ClearBox'];
  if (tree === undefined) throw new Error('config has no ClearBox extension');
  return tree;
}

export function clearbox(cfg: MachineConfig, index: number): ExtensionTree {
  return clearboxRoot(cfg).children[trainKey(index)];
}

export function powerCharacterization(cfg: MachineConfig, index: number): ExtensionTree {
  return clearbox(cfg, index).children['Power_Characterization'];
}

/** The attribute's value, or `undefined` when absent. */
export function attr(tree: ExtensionTree, name: string): unknown {
  return tree.attrs[name]?.value;
}

/** Non-empty string attribute; the contract stores "absent" as `""`. */
export function text(tree: ExtensionTree, name: string): string | null {
  const v = attr(tree, name);
  return typeof v === 'string' && v !== '' ? v : null;
}

export function int(tree: ExtensionTree, name: string): number | null {
  const v = attr(tree, name);
  return typeof v === 'number' || typeof v === 'bigint' ? Number(v) : null;
}

export function constants(pc: ExtensionTree): Array<{ name: string; value: number }> {
  const t = pc.tables!['Derivation_Equation_Constants'];
  const names = t.columns['name'] as string[];
  const values = t.columns['value'] as Float64Array;
  return names.map((name, i) => ({ name, value: values[i] }));
}

export function pointCount(pc: ExtensionTree): number {
  return pc.tables!['Characterization_Points'].rows;
}

export function makePowerCharacterization(
  algorithm: string,
  rows: Array<[string, number]>,
): ExtensionTree {
  const tree = emptyExtensionTree();
  for (const [name, value] of [
    ['Algorithm_Type', algorithm],
    ['Algorithm_Equation', ''],
    ['Input_Type', ''],
    ['Units_Derived_Quantity', ''],
  ] as const) {
    tree.attrs[name] = { dtype: 'utf8', value };
  }
  tree.tables = {
    Derivation_Equation_Constants: {
      fields: [
        { name: 'name', dtype: 'utf8', size: 64 },
        { name: 'value', dtype: 'f64' },
      ],
      rows: rows.length,
      columns: { name: rows.map((r) => r[0]), value: Float64Array.from(rows.map((r) => r[1])) },
      attrs: {},
    },
    Characterization_Points: {
      fields: [
        { name: 'input_value', dtype: 'f64' },
        { name: 'output_value', dtype: 'f64' },
      ],
      rows: 0,
      columns: { input_value: new Float64Array(0), output_value: new Float64Array(0) },
      attrs: {},
    },
  };
  return tree;
}

/** A sensor group whose only content is a `Derivation_Equation_Constants` table. */
export function sensorTree(rows: Array<[string, number]>, attrs: ExtensionTree['attrs'] = {}): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs = attrs;
  tree.tables = {
    Derivation_Equation_Constants: {
      fields: [
        { name: 'name', dtype: 'utf8', size: 64 },
        { name: 'value', dtype: 'f64' },
      ],
      rows: rows.length,
      columns: { name: rows.map((r) => r[0]), value: Float64Array.from(rows.map((r) => r[1])) },
      attrs: {},
    },
  };
  return tree;
}

/** Adds `sensor` under train 0's `Synchronous_Sensors/<label>` (mutates `cfg`). */
export function addSensor(cfg: MachineConfig, label: string, sensor: ExtensionTree): void {
  const train = clearbox(cfg, 0);
  const sensors = (train.children['Synchronous_Sensors'] ??= emptyExtensionTree());
  sensors.children[label] = sensor;
}

/** A forward/inverse grid of train `index` as a flat Float64Array plus its shape. */
export function grid(cfg: MachineConfig, index: number, name: string): { data: Float64Array; shape: number[] } {
  const ds = clearbox(cfg, index).datasets[name];
  return { data: ds.data as Float64Array, shape: ds.shape };
}
