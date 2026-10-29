import { createHash, type Hash } from "node:crypto";
import {
  assertValidExtensionTree,
  type ExtensionAttr,
  type ExtensionDtype,
  type ExtensionScalar,
  type ExtensionTable,
  type ExtensionTree,
} from "./contract.js";

/**
 * SHA-256 over a canonical binary encoding of a tree, identical in every
 * language. Everything is little-endian:
 *
 *   tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
 *   name    = u32 byteLength, UTF-8 bytes
 *   attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
 *   dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
 *   value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
 *
 * Appended after the datasets only when a tree has tables (so trees without them hash as in 0.1.0):
 *
 *   tables  = u32 0xFFFFFFFF, u32 tableCount, (name table)*
 *   table   = u32 fieldCount, (name, u8 dtypeTag, u32 size)*, u64 rows, column values*, attr map
 *
 * Table fields keep their declared order (it is the compound member order); each column's values
 * follow in that order, row by row within a column.
 *
 * Entries are sorted by the UTF-8 bytes of their names. NaN is encoded as the
 * quiet NaN and -0 is encoded as +0, so equal values always hash equal.
 */
const DTYPE_TAG: Record<ExtensionDtype, number> = {
  utf8: 1, i8: 2, u8: 3, i16: 4, u16: 5, i32: 6, u32: 7, i64: 8, u64: 9, f32: 10, f64: 11,
};

function u32(h: Hash, n: number): void {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  h.update(b);
}

function u64(h: Hash, n: number | bigint): void {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n));
  h.update(b);
}

function name(h: Hash, s: string): Buffer {
  const bytes = Buffer.from(s, "utf8");
  u32(h, bytes.length);
  h.update(bytes);
  return bytes;
}

function sortedNames(names: string[]): string[] {
  return [...names].sort((a, b) => Buffer.compare(Buffer.from(a, "utf8"), Buffer.from(b, "utf8")));
}

function value(h: Hash, dtype: ExtensionDtype, v: ExtensionScalar): void {
  const b = Buffer.alloc(8);
  switch (dtype) {
    case "utf8": {
      const bytes = Buffer.from(v as string, "utf8");
      u32(h, bytes.length);
      h.update(bytes);
      return;
    }
    case "i8": b.writeInt8(v as number); h.update(b.subarray(0, 1)); return;
    case "u8": b.writeUInt8(v as number); h.update(b.subarray(0, 1)); return;
    case "i16": b.writeInt16LE(v as number); h.update(b.subarray(0, 2)); return;
    case "u16": b.writeUInt16LE(v as number); h.update(b.subarray(0, 2)); return;
    case "i32": b.writeInt32LE(v as number); h.update(b.subarray(0, 4)); return;
    case "u32": b.writeUInt32LE(v as number); h.update(b.subarray(0, 4)); return;
    case "i64": b.writeBigInt64LE(v as bigint); h.update(b); return;
    case "u64": b.writeBigUInt64LE(v as bigint); h.update(b); return;
    case "f32": {
      const n = v as number;
      if (Number.isNaN(n)) b.writeUInt32LE(0x7fc00000);
      else b.writeFloatLE(n === 0 ? 0 : n);
      h.update(b.subarray(0, 4));
      return;
    }
    case "f64": {
      const n = v as number;
      if (Number.isNaN(n)) b.writeBigUInt64LE(0x7ff8000000000000n);
      else b.writeDoubleLE(n === 0 ? 0 : n);
      h.update(b);
      return;
    }
  }
}

function attrMap(h: Hash, attrs: Record<string, ExtensionAttr>): void {
  const names = sortedNames(Object.keys(attrs));
  u32(h, names.length);
  for (const n of names) {
    const a = attrs[n];
    name(h, n);
    h.update(Uint8Array.of(DTYPE_TAG[a.dtype]));
    if (Array.isArray(a.value)) {
      h.update(Uint8Array.of(1));
      u32(h, a.value.length);
      for (const v of a.value) value(h, a.dtype, v);
    } else {
      h.update(Uint8Array.of(0));
      value(h, a.dtype, a.value);
    }
  }
}

function tree(h: Hash, t: ExtensionTree): void {
  attrMap(h, t.attrs);
  const children = sortedNames(Object.keys(t.children));
  u32(h, children.length);
  for (const n of children) {
    name(h, n);
    tree(h, t.children[n]);
  }
  const datasets = sortedNames(Object.keys(t.datasets));
  u32(h, datasets.length);
  for (const n of datasets) {
    const ds = t.datasets[n];
    name(h, n);
    h.update(Uint8Array.of(DTYPE_TAG[ds.dtype]));
    u32(h, ds.shape.length);
    for (const d of ds.shape) u64(h, d);
    u64(h, ds.data.length);
    const values = ds.data as ArrayLike<number | bigint>;
    for (let i = 0; i < values.length; i++) value(h, ds.dtype, values[i]);
    attrMap(h, ds.attrs);
  }
  const tableNames = sortedNames(Object.keys(t.tables ?? {}));
  if (tableNames.length > 0) {
    u32(h, 0xffffffff);
    u32(h, tableNames.length);
    for (const n of tableNames) {
      const table = (t.tables as Record<string, ExtensionTable>)[n];
      name(h, n);
      u32(h, table.fields.length);
      for (const f of table.fields) {
        name(h, f.name);
        h.update(Uint8Array.of(DTYPE_TAG[f.dtype]));
        u32(h, f.size ?? 0);
      }
      u64(h, table.rows);
      for (const f of table.fields) {
        const column = table.columns[f.name] as ArrayLike<ExtensionScalar>;
        for (let i = 0; i < column.length; i++) value(h, f.dtype, column[i]);
      }
      attrMap(h, table.attrs);
    }
  }
}

/** Hex SHA-256 of the canonical encoding. Throws if the tree violates the contract. */
export function canonicalDigest(extension: ExtensionTree): string {
  assertValidExtensionTree(extension);
  const h = createHash("sha256");
  tree(h, extension);
  return h.digest("hex");
}
