import { describe, it, expect, afterAll } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unlinkSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import * as h5wasm from 'h5wasm/node';
import {
  MachineConfigReader,
  MachineConfigWriter,
  canonicalDigest,
  emptyExtensionTree,
  extensionTreeFromJson,
  extensionTreeToJson,
  assertValidExtensionTree,
  ExtensionContractError,
  EXTENSION_CONTRACT_VERSION,
} from '../src/index.js';
import { MockConfigBuilder } from '../src/builder.js';
import type { ExtensionTree, ExtensionTable } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const JSON_FIXTURE = join(__dirname, '../../fixtures/extension_table_sample_tree.json');
const NODE_FIXTURE = join(__dirname, '../../fixtures/extension_table_sample_node_v1_1.h5');
const RUST_FIXTURE = join(__dirname, '../../fixtures/extension_table_sample_rust_v1_1.h5');

// Pinned; Rust asserts the same value (extension-contract/tests/tables.rs).
const TABLE_SAMPLE_DIGEST = '600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858';
// The tree without tables must keep the contract 0.1.0 digest.
const LEGACY_SAMPLE_DIGEST = '8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890';

const tmpFiles: string[] = [];
function tmpH5(): string {
  const p = join(tmpdir(), `test_ext_tables_${randomBytes(8).toString('hex')}.h5`);
  tmpFiles.push(p);
  return p;
}
afterAll(() => {
  for (const f of tmpFiles) if (existsSync(f)) unlinkSync(f);
});

function pointsTable(): ExtensionTable {
  return {
    fields: [
      { name: 'input_value', dtype: 'f64' },
      { name: 'output_value', dtype: 'f64' },
    ],
    rows: 2,
    columns: { input_value: new Float64Array([4, 20]), output_value: new Float64Array([-1, 6]) },
    attrs: {},
  };
}

/** Covers a fixed-length string, every width class, 64-bit values, an empty table and a nested table. */
function tableTree(): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs = { Schema: { dtype: 'utf8', value: '1.0' } };
  tree.tables = {
    Constants: {
      fields: [
        { name: 'name', dtype: 'utf8', size: 64 },
        { name: 'value', dtype: 'f64' },
      ],
      rows: 2,
      columns: { name: ['a', 'b'], value: new Float64Array([0.4375, -2.75]) },
      attrs: { Units: { dtype: 'utf8', value: 'V' } },
    },
    Mixed: {
      fields: [
        { name: 'id', dtype: 'u32' },
        { name: 'count', dtype: 'i64' },
        { name: 'ratio', dtype: 'f32' },
        { name: 'flag', dtype: 'u8' },
        { name: 'label', dtype: 'utf8', size: 8 },
      ],
      rows: 3,
      columns: {
        id: new Uint32Array([1, 2, 4294967295]),
        count: new BigInt64Array([-9007199254740993n, 0n, 7n]),
        ratio: new Float32Array([0.5, -1.5, 2]),
        flag: new Uint8Array([0, 1, 1]),
        label: ['x', '\u00b5m', ''],
      },
      attrs: {},
    },
    Empty: {
      fields: [{ name: 'input_value', dtype: 'f64' }, { name: 'output_value', dtype: 'f64' }],
      rows: 0,
      columns: { input_value: new Float64Array(0), output_value: new Float64Array(0) },
      attrs: {},
    },
  };
  const child = emptyExtensionTree();
  child.tables = { Points: pointsTable() };
  tree.children.Train_1 = child;
  return tree;
}

function config(extensions: Record<string, ExtensionTree>) {
  return { ...new MockConfigBuilder({ nLasers: 1, fileVersion: '1.1' }).build(), extensions };
}

describe('compound tables: contract', () => {
  it('stays on the 0.1 contract line so mcf-ext-opcua (^0.1) remains compatible', () => {
    expect(EXTENSION_CONTRACT_VERSION).toMatch(/^0\.1\./);
  });

  it('leaves trees without tables untouched', () => {
    const tree = emptyExtensionTree();
    expect(tree).toEqual({ attrs: {}, children: {}, datasets: {} });
    expect(extensionTreeToJson(tree)).not.toHaveProperty('tables');
  });

  it('rejects malformed tables with a path', () => {
    const mutate = (fn: (t: ExtensionTable) => void): ExtensionTree => {
      const tree = emptyExtensionTree();
      const t = pointsTable();
      fn(t);
      tree.tables = { T: t };
      return tree;
    };
    expect(() => assertValidExtensionTree(mutate((t) => (t.fields = [])))).toThrow(/at least one field/);
    expect(() => assertValidExtensionTree(mutate((t) => (t.fields[1].name = 'input_value')))).toThrow(/duplicate field/);
    expect(() => assertValidExtensionTree(mutate((t) => (t.rows = 3)))).toThrow(/does not match 3 rows/);
    expect(() => assertValidExtensionTree(mutate((t) => (t.columns.input_value = new Float32Array(2))))).toThrow(/Float64Array/);
    expect(() => assertValidExtensionTree(mutate((t) => delete t.columns.output_value))).toThrow(/missing column/);
    expect(() => assertValidExtensionTree(mutate((t) => (t.columns.extra = new Float64Array(2))))).toThrow(/no matching field/);
    expect(() => assertValidExtensionTree(mutate((t) => (t.fields[0].size = 8)))).toThrow(/only valid for utf8/);
    expect(() =>
      assertValidExtensionTree(
        mutate((t) => {
          t.fields[0] = { name: 'input_value', dtype: 'utf8' };
          t.columns.input_value = ['a', 'b'] as unknown as Float64Array;
        }),
      ),
    ).toThrow(/positive integer size/);
    expect(() =>
      assertValidExtensionTree(
        mutate((t) => {
          t.fields[0] = { name: 'input_value', dtype: 'utf8', size: 2 };
          t.columns.input_value = ['abc', 'b'] as unknown as Float64Array;
        }),
      ),
    ).toThrow(/do not fit the 2-byte field/);

    const clash = emptyExtensionTree();
    clash.children.T = emptyExtensionTree();
    clash.tables = { T: pointsTable() };
    expect(() => assertValidExtensionTree(clash, 'Ext')).toThrow(ExtensionContractError);
    expect(() => assertValidExtensionTree(clash, 'Ext')).toThrow(/Ext\/T/);
  });

  it('names the failing field path', () => {
    const tree = emptyExtensionTree();
    const t = pointsTable();
    t.columns.output_value = new Float64Array(1);
    tree.tables = { Points: t };
    expect(() => assertValidExtensionTree(tree, 'Ext')).toThrow(/Ext\/Points\/output_value/);
  });
});

describe('compound tables: digest', () => {
  it('does not change the digest of a tree that has no tables', () => {
    const tree = emptyExtensionTree();
    tree.attrs.A = { dtype: 'u32', value: 1 };
    const withEmptyMap = { ...tree, tables: {} };
    expect(canonicalDigest(withEmptyMap)).toBe(canonicalDigest(tree));
  });

  it('is pinned for the table sample so other languages can match it', () => {
    expect(canonicalDigest(tableTree())).toBe(TABLE_SAMPLE_DIGEST);
  });

  it('is sensitive to field order, size, values and row count but not to table insertion order', () => {
    const base = tableTree();
    const d = canonicalDigest(base);

    const reordered = tableTree();
    reordered.tables = Object.fromEntries(Object.entries(reordered.tables!).reverse());
    expect(canonicalDigest(reordered)).toBe(d);

    const swapped = tableTree();
    swapped.tables!.Constants.fields.reverse();
    expect(canonicalDigest(swapped)).not.toBe(d);

    const wider = tableTree();
    wider.tables!.Constants.fields[0].size = 32;
    expect(canonicalDigest(wider)).not.toBe(d);

    const edited = tableTree();
    (edited.tables!.Constants.columns.value as Float64Array)[1] = -2.5;
    expect(canonicalDigest(edited)).not.toBe(d);

    const shorter = tableTree();
    shorter.children.Train_1.tables!.Points.rows = 1;
    shorter.children.Train_1.tables!.Points.columns = {
      input_value: new Float64Array(1),
      output_value: new Float64Array(1),
    };
    expect(canonicalDigest(shorter)).not.toBe(d);
  });

  it('keeps the contract 0.1.0 golden digest for a tree without tables', () => {
    const j = JSON.parse(readFileSync(join(__dirname, '../../fixtures/extension_sample_tree.json'), 'utf8'));
    expect(canonicalDigest(extensionTreeFromJson(j))).toBe(LEGACY_SAMPLE_DIGEST);
  });
});

describe('compound tables: JSON', () => {
  it('round-trips losslessly, including 64-bit values and strings', () => {
    const tree = tableTree();
    const json = JSON.parse(JSON.stringify(extensionTreeToJson(tree)));
    expect(json.tables.Mixed.columns.count).toEqual(['-9007199254740993', '0', '7']);
    expect(json.tables.Constants.fields[0]).toEqual({ name: 'name', dtype: 'utf8', size: 64 });
    expect(json.tables.Constants.fields[1]).toEqual({ name: 'value', dtype: 'f64' });
    const back = extensionTreeFromJson(json);
    expect(canonicalDigest(back)).toBe(canonicalDigest(tree));
  });

  it('matches the golden fixture', () => {
    if (process.env.UPDATE_EXTENSION_FIXTURES) {
      writeFileSync(JSON_FIXTURE, JSON.stringify(extensionTreeToJson(tableTree()), null, 2) + '\n');
    }
    const j = JSON.parse(readFileSync(JSON_FIXTURE, 'utf8'));
    expect(canonicalDigest(extensionTreeFromJson(j))).toBe(TABLE_SAMPLE_DIGEST);
  });
});

describe('compound tables: HDF5 round trip through core', () => {
  it('writes and reads every table verbatim (File_Version 1.1)', async () => {
    const out = tmpH5();
    await new MachineConfigWriter(config({ Private_Ext: tableTree() }), '1.1').write(out);

    const got = (await new MachineConfigReader(out).parse()).extensions!.Private_Ext;
    expect(canonicalDigest(got)).toBe(TABLE_SAMPLE_DIGEST);
    const mixed = got.tables!.Mixed;
    expect(mixed.fields.map((f) => `${f.name}:${f.dtype}${f.size ?? ''}`)).toEqual([
      'id:u32', 'count:i64', 'ratio:f32', 'flag:u8', 'label:utf88',
    ]);
    expect(Array.from(mixed.columns.count as BigInt64Array)).toEqual([-9007199254740993n, 0n, 7n]);
    expect(mixed.columns.label).toEqual(['x', '\u00b5m', '']);
    expect(got.tables!.Empty.rows).toBe(0);
    expect(got.tables!.Constants.attrs.Units).toEqual({ dtype: 'utf8', value: 'V' });
    expect(Object.keys(got.children.Train_1.tables!)).toEqual(['Points']);
  });

  it('leaves a tree without tables with no tables key after a round trip', async () => {
    const tree = emptyExtensionTree();
    tree.attrs.A = { dtype: 'u32', value: 1 };
    const out = tmpH5();
    await new MachineConfigWriter(config({ Private_Ext: tree }), '1.1').write(out);
    const got = (await new MachineConfigReader(out).parse()).extensions!.Private_Ext;
    expect(got).not.toHaveProperty('tables');
  });

  it('rejects an invalid table before creating the output file', () => {
    const bad = tableTree();
    bad.tables!.Constants.columns.name = ['only-one'];
    expect(() => new MachineConfigWriter(config({ Bad: bad }), '1.1')).toThrow(ExtensionContractError);
  });

  it('writes the Node fixture when asked and reads both languages\' fixtures', async () => {
    if (process.env.UPDATE_EXTENSION_FIXTURES) {
      await new MachineConfigWriter(config({ Private_Ext: tableTree() }), '1.1').write(NODE_FIXTURE);
    }
    for (const path of [NODE_FIXTURE, RUST_FIXTURE]) {
      if (!existsSync(path)) continue; // the Rust fixture appears after its first generation run
      const parsed = await new MachineConfigReader(path).parse();
      expect(canonicalDigest(parsed.extensions!.Private_Ext)).toBe(TABLE_SAMPLE_DIGEST);
    }
  });
});

// Array members are covered by the Rust test; h5wasm cannot create one.
describe('compound tables: unsupported shapes fail loudly with their path', () => {
  async function fileWithCompound(add: (group: h5wasm.Group) => void): Promise<string> {
    const out = tmpH5();
    await new MachineConfigWriter(config({ Private_Ext: emptyExtensionTree() }), '1.1').write(out);
    await h5wasm.ready;
    const f = new h5wasm.File(out.replace(/\\/g, '/'), 'a');
    try {
      add(f.get('Extensions/Private_Ext') as h5wasm.Group);
    } finally {
      f.close();
    }
    return out;
  }

  it('rejects a compound dataset that is not 1-D', async () => {
    const out = await fileWithCompound((g) =>
      g.create_dataset({
        name: 'Grid',
        data: new Map<string, unknown>([['a', new Float64Array(4)]]),
        dtype: [['a', '<d']],
        shape: [2, 2],
      }),
    );
    await expect(new MachineConfigReader(out).parse()).rejects.toThrow(/Private_Ext\/Grid.*1-D/);
  });

});
