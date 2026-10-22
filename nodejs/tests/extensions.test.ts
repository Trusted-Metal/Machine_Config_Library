import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unlinkSync, existsSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import * as h5wasm from 'h5wasm/node';
import {
  MachineConfigReader,
  MachineConfigWriter,
  canonicalDigest,
  computeConfigurationHash,
  emptyExtensionTree,
  extensionTreeFromJson,
  extensionTreeToJson,
  assertValidExtensionTree,
  ExtensionContractError,
  EXTENSION_CONTRACT_VERSION,
} from '../src/index.js';
import { openMachineConfig } from '../src/capabilities/index.js';
import { MockConfigBuilder } from '../src/builder.js';
import type { ExtensionTree, MachineConfig } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE_V1_1 = join(__dirname, '../../fixtures/reference_config_v1_1.h5');

const tmpFiles: string[] = [];
function tmpH5(): string {
  const p = join(tmpdir(), `test_ext_${randomBytes(8).toString('hex')}.h5`);
  tmpFiles.push(p);
  return p;
}
afterAll(() => {
  for (const f of tmpFiles) if (existsSync(f)) unlinkSync(f);
});

/** A tree exercising every dtype, array attributes, nesting, and a dataset with attributes. */
function sampleTree(): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs = {
    Schema: { dtype: 'utf8', value: '1.2' },
    Flag: { dtype: 'u32', value: 1 },
    Float_Flag: { dtype: 'f64', value: 1 },
    Small: { dtype: 'i8', value: -5 },
    Wide: { dtype: 'i64', value: -9007199254740993n },
    Huge: { dtype: 'u64', value: 18446744073709551615n },
    Ratio: { dtype: 'f32', value: 0.5 },
    Pair: { dtype: 'f64', value: [1.5, -2.5] },
    Ids: { dtype: 'u32', value: [1, 2, 3] },
  };
  const child = emptyExtensionTree();
  child.attrs = { Label: { dtype: 'utf8', value: 'Job Start' } };
  const grandchild = emptyExtensionTree();
  grandchild.attrs = { Value: { dtype: 'utf8', value: 'running' } };
  child.children['Condition 1'] = grandchild;
  tree.children['Job Start'] = child;
  tree.datasets['Grid'] = {
    dtype: 'f64',
    shape: [2, 3],
    data: new Float64Array([1, 2, 3, 4, 5, 6.5]),
    attrs: { Units: { dtype: 'utf8', value: 'mm' } },
  };
  return tree;
}

let base: MachineConfig;
beforeAll(async () => {
  base = await new MachineConfigReader(REFERENCE_V1_1).parse();
}, 60_000);

function withExtensions(extensions: Record<string, ExtensionTree>): MachineConfig {
  const { extensions: _existing, ...rest } = structuredClone(base);
  return { ...rest, extensions };
}

describe('extension contract validation', () => {
  it('exposes a pre-stable contract version', () => {
    expect(EXTENSION_CONTRACT_VERSION).toMatch(/^0\./);
  });

  it.each([
    ['empty name', ''],
    ['slash', 'a/b'],
    ['NUL', 'a\0b'],
  ])('rejects a child name with %s', (_label, name) => {
    const tree = emptyExtensionTree();
    tree.children[name] = emptyExtensionTree();
    expect(() => assertValidExtensionTree(tree, 'Ext')).toThrow(ExtensionContractError);
  });

  it('rejects values that do not fit their dtype', () => {
    const tree = emptyExtensionTree();
    tree.attrs.X = { dtype: 'u8', value: 256 };
    expect(() => assertValidExtensionTree(tree)).toThrow(/out of range/);
    tree.attrs.X = { dtype: 'i64', value: 1 };
    expect(() => assertValidExtensionTree(tree)).toThrow(/bigint/);
    tree.attrs.X = { dtype: 'u32', value: 1.5 };
    expect(() => assertValidExtensionTree(tree)).toThrow(/integer/);
  });

  it('rejects a dataset whose data does not match shape or dtype', () => {
    const tree = emptyExtensionTree();
    tree.datasets.D = { dtype: 'f64', shape: [2, 2], data: new Float64Array(3), attrs: {} };
    expect(() => assertValidExtensionTree(tree)).toThrow(/does not match shape/);
    tree.datasets.D = { dtype: 'f64', shape: [3], data: new Float32Array(3), attrs: {} };
    expect(() => assertValidExtensionTree(tree)).toThrow(/Float64Array/);
  });

  it('names the path of the offending node', () => {
    const tree = emptyExtensionTree();
    const child = emptyExtensionTree();
    child.attrs.Bad = { dtype: 'u8', value: -1 };
    tree.children.Rule = child;
    expect(() => assertValidExtensionTree(tree, 'Ext')).toThrow(/Ext\/Rule\/@Bad/);
  });
});

describe('canonicalDigest', () => {
  it('ignores insertion order', () => {
    const a = emptyExtensionTree();
    a.attrs = { A: { dtype: 'u32', value: 1 }, B: { dtype: 'u32', value: 2 } };
    const b = emptyExtensionTree();
    b.attrs = { B: { dtype: 'u32', value: 2 }, A: { dtype: 'u32', value: 1 } };
    expect(canonicalDigest(a)).toBe(canonicalDigest(b));
  });

  it('distinguishes dtype, value, and structure', () => {
    const mk = (dtype: 'u32' | 'f64', value: number) => {
      const t = emptyExtensionTree();
      t.attrs.A = { dtype, value };
      return canonicalDigest(t);
    };
    expect(mk('u32', 1)).not.toBe(mk('f64', 1));
    expect(mk('u32', 1)).not.toBe(mk('u32', 2));
    const withChild = emptyExtensionTree();
    withChild.children.A = emptyExtensionTree();
    const withAttr = emptyExtensionTree();
    withAttr.attrs.A = { dtype: 'u32', value: 0 };
    expect(canonicalDigest(withChild)).not.toBe(canonicalDigest(withAttr));
  });

  it('treats -0 as +0 and every NaN as equal', () => {
    const mk = (v: number) => {
      const t = emptyExtensionTree();
      t.attrs.A = { dtype: 'f64', value: v };
      return canonicalDigest(t);
    };
    expect(mk(-0)).toBe(mk(0));
    expect(mk(NaN)).toBe(mk(Number.NaN * -1));
  });

  it('is pinned for a fixed tree so other languages can match it', () => {
    expect(canonicalDigest(sampleTree())).toBe('8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890');
  });

  it('reads the golden JSON fixture to the pinned digest', () => {
    const j = JSON.parse(readFileSync(join(__dirname, '../../fixtures/extension_sample_tree.json'), 'utf8'));
    expect(canonicalDigest(extensionTreeFromJson(j))).toBe('8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890');
  });
});

describe('extension JSON form', () => {
  it('round-trips bigint, non-finite floats and datasets losslessly', () => {
    const tree = sampleTree();
    tree.attrs.Weird = { dtype: 'f64', value: [NaN, Infinity, -Infinity] };
    const json = JSON.parse(JSON.stringify(extensionTreeToJson(tree)));
    const back = extensionTreeFromJson(json);
    expect(canonicalDigest(back)).toBe(canonicalDigest(tree));
    expect(back.attrs.Wide.value).toBe(-9007199254740993n);
  });
});

describe('core read/write of extension trees', () => {
  it.each(['1.1'] as const)('round-trips an unknown extension verbatim (File_Version %s)', async (version) => {
    const tree = sampleTree();
    const out = tmpH5();
    await new MachineConfigWriter(withExtensions({ Private_Ext: tree }), version).write(out);

    const parsed = await new MachineConfigReader(out).parse();
    expect(Object.keys(parsed.extensions ?? {})).toEqual(['Private_Ext']);
    const got = parsed.extensions!.Private_Ext;
    expect(canonicalDigest(got)).toBe(canonicalDigest(tree));
    expect(got.attrs.Wide).toEqual({ dtype: 'i64', value: -9007199254740993n });
    expect(got.attrs.Flag).toEqual({ dtype: 'u32', value: 1 });
    expect(got.attrs.Float_Flag).toEqual({ dtype: 'f64', value: 1 });
    expect(Array.from(got.datasets.Grid.data as Float64Array)).toEqual([1, 2, 3, 4, 5, 6.5]);
    expect(got.datasets.Grid.shape).toEqual([2, 3]);
  });

  it('preserves an unknown extension across read -> write -> read', async () => {
    const first = tmpH5();
    await new MachineConfigWriter(withExtensions({ Private_Ext: sampleTree() }), '1.1').write(first);
    const parsed = await new MachineConfigReader(first).parse();

    const second = tmpH5();
    await new MachineConfigWriter(parsed).write(second);
    const again = await new MachineConfigReader(second).parse();
    expect(canonicalDigest(again.extensions!.Private_Ext)).toBe(canonicalDigest(sampleTree()));
  });

  it('preserves an unknown extension through an open -> save session', async () => {
    const first = tmpH5();
    await new MachineConfigWriter(withExtensions({ Private_Ext: sampleTree() }), '1.1').write(first);

    const opened = await openMachineConfig(first);
    if (!opened.ok) throw new Error(opened.error.message);
    const second = tmpH5();
    const saved = await opened.value.save(second);
    expect(saved.ok).toBe(true);

    const again = await new MachineConfigReader(second).parse();
    expect(canonicalDigest(again.extensions!.Private_Ext)).toBe(canonicalDigest(sampleTree()));
  });

  it('reads the TM_OPCUA group of a v1.1 file like any other extension', () => {
    expect(Object.keys(base.extensions ?? {})).toEqual(['TM_OPCUA']);
    expect(base).not.toHaveProperty('opcua');
  });

  it('does not list ClearBox, which core still models, and rejects writing it as a tree', async () => {
    await expect(
      new MachineConfigWriter(withExtensions({ ClearBox: emptyExtensionTree() }), '1.1').write(tmpH5()),
    ).rejects.toThrow(/modelled by core/);
  });

  it('rejects an invalid tree before creating the output file', async () => {
    const bad = emptyExtensionTree();
    bad.attrs.X = { dtype: 'u8', value: 999 };
    const out = tmpH5();
    // The writer hashes the config on construction, which already validates every tree.
    expect(() => new MachineConfigWriter(withExtensions({ Bad: bad }), '1.1')).toThrow(ExtensionContractError);
    expect(existsSync(out)).toBe(false);
  });

  it('serialises extensions in toJson without throwing on bigint', async () => {
    const out = tmpH5();
    await new MachineConfigWriter(withExtensions({ Private_Ext: sampleTree() }), '1.1').write(out);
    const json = JSON.parse(await new MachineConfigReader(out).toJson());
    expect(json.extensions.Private_Ext.attrs.Wide).toEqual({ dtype: 'i64', value: '-9007199254740993' });
  });
});

describe('TM_OPCUA as an extension tree', () => {
  it('writes a TM_OPCUA tree to Extensions/TM_OPCUA in 1.1 and reads it back', async () => {
    const tree = emptyExtensionTree();
    tree.attrs.Opcua_Schema_Version = { dtype: 'utf8', value: '1.2' };
    const client = emptyExtensionTree();
    client.attrs.Auth_Mode = { dtype: 'utf8', value: 'Anonymous' };
    tree.children.Client = client;

    const out = tmpH5();
    await new MachineConfigWriter(withExtensions({ TM_OPCUA: tree }), '1.1').write(out);

    await h5wasm.ready;
    const f = new h5wasm.File(out.replace(/\\/g, '/'), 'r');
    try {
      expect(f.get('Extensions/TM_OPCUA/Client')).toBeTruthy();
    } finally {
      f.close();
    }

    const parsed = await new MachineConfigReader(out).parse();
    expect(canonicalDigest(parsed.extensions!.TM_OPCUA)).toBe(canonicalDigest(tree));
  });

  it('writes a TM_OPCUA tree to OPCUA/ in 1.0 and exposes it again as an extension', async () => {
    const tree = emptyExtensionTree();
    tree.attrs.Opcua_Schema_Version = { dtype: 'utf8', value: '1.2' };
    const out = tmpH5();
    await new MachineConfigWriter(withExtensions({ TM_OPCUA: tree }), '1.0').write(out);

    await h5wasm.ready;
    const f = new h5wasm.File(out.replace(/\\/g, '/'), 'r');
    try {
      expect(f.get('OPCUA')).toBeTruthy();
    } finally {
      f.close();
    }
    const parsed = await new MachineConfigReader(out).parse();
    expect(canonicalDigest(parsed.extensions!.TM_OPCUA)).toBe(canonicalDigest(tree));
  });

  it('rejects other extension names for File_Version 1.0', async () => {
    await expect(
      new MachineConfigWriter(withExtensions({ Private_Ext: emptyExtensionTree() }), '1.0').write(tmpH5()),
    ).rejects.toThrow(/no Extensions group/);
  });

});

describe('configuration hash', () => {
  it('is unchanged by an absent or empty extensions map', () => {
    const plain = withExtensions({});
    const { extensions: _none, ...withoutKey } = plain;
    expect(computeConfigurationHash(plain)).toBe(computeConfigurationHash(withoutKey as MachineConfig));
  });

  it('changes when extension content changes', () => {
    const a = computeConfigurationHash(withExtensions({ Private_Ext: sampleTree() }));
    const changed = sampleTree();
    changed.attrs.Flag = { dtype: 'u32', value: 0 };
    const b = computeConfigurationHash(withExtensions({ Private_Ext: changed }));
    expect(a).not.toBe(computeConfigurationHash(withExtensions({})));
    expect(a).not.toBe(b);
  });
});

describe('cross-language fixtures', () => {
  // Each language writes its own fixture (UPDATE_EXTENSION_FIXTURES=1) and reads both; the
  // digest proves Node and Rust agree on dtypes, values, structure and the canonical encoding.
  const NODE_FIXTURE = join(__dirname, '../../fixtures/extension_sample_node_v1_1.h5');
  const RUST_FIXTURE = join(__dirname, '../../fixtures/extension_sample_rust_v1_1.h5');
  const SAMPLE_DIGEST = '8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890';

  beforeAll(async () => {
    if (process.env.UPDATE_EXTENSION_FIXTURES) {
      const small = new MockConfigBuilder({ nLasers: 1, fileVersion: '1.1' }).build();
      await new MachineConfigWriter({ ...small, extensions: { Private_Ext: sampleTree() } }, '1.1').write(NODE_FIXTURE);
    }
  });

  it.each([
    ['Node', NODE_FIXTURE],
    ['Rust', RUST_FIXTURE],
  ])('reads the %s-written fixture', async (_who, path) => {
    const parsed = await new MachineConfigReader(path).parse();
    expect(canonicalDigest(parsed.extensions!.Private_Ext)).toBe(SAMPLE_DIGEST);
  });
});
