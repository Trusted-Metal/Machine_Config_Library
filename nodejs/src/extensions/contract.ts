/**
 * Extension contract: the opaque, language-neutral tree core uses to carry
 * data that belongs to an extension (a group under `Extensions/<name>/`) so a
 * separate library can pack/unpack it without core knowing its schema.
 *
 * Dependency-free on purpose (no h5wasm, no node:crypto): extension libraries
 * import only this module's types. Core reads and writes every tree verbatim,
 * so unknown or private extensions round-trip untouched.
 *
 * Pre-stable (0.x) until a second extension has been ported against it.
 */

export const EXTENSION_CONTRACT_VERSION = "0.1.1";

/** Fixed-width storage types; each maps 1:1 to an HDF5 type. */
export type ExtensionDtype =
  | "utf8"
  | "i8"
  | "u8"
  | "i16"
  | "u16"
  | "i32"
  | "u32"
  | "i64"
  | "u64"
  | "f32"
  | "f64";

export type ExtensionScalar = string | number | bigint;

/** `value` is a scalar, or a 1-D array. i64/u64 are `bigint`; every other numeric dtype is `number`. */
export interface ExtensionAttr {
  dtype: ExtensionDtype;
  value: ExtensionScalar | ExtensionScalar[];
}

export type ExtensionNumericArray =
  | Int8Array
  | Uint8Array
  | Int16Array
  | Uint16Array
  | Int32Array
  | Uint32Array
  | BigInt64Array
  | BigUint64Array
  | Float32Array
  | Float64Array;

/** N-d array stored row-major in `data`; `data.length` equals the product of `shape` (empty shape = scalar). */
export interface ExtensionDataset {
  dtype: Exclude<ExtensionDtype, "utf8">;
  shape: number[];
  data: ExtensionNumericArray;
  attrs: Record<string, ExtensionAttr>;
}

/** One column of a compound table. `size` is the fixed byte length of a `utf8` column and is absent for numeric ones. */
export interface ExtensionTableField {
  name: string;
  dtype: ExtensionDtype;
  size?: number;
}

/**
 * 1-D compound dataset (HDF5 compound type): one row per record, one named column per field.
 * `fields` order is the compound member order and is significant. A `utf8` column is a fixed-length
 * string of at most `size` UTF-8 bytes; numeric columns use the typed array of their dtype.
 */
export interface ExtensionTable {
  fields: ExtensionTableField[];
  rows: number;
  columns: Record<string, ExtensionNumericArray | string[]>;
  attrs: Record<string, ExtensionAttr>;
}

/**
 * Order of attrs/children/datasets/tables is not significant. `tables` is absent unless used, so
 * trees without compound data are shaped exactly as in contract 0.1.0.
 */
export interface ExtensionTree {
  attrs: Record<string, ExtensionAttr>;
  children: Record<string, ExtensionTree>;
  datasets: Record<string, ExtensionDataset>;
  tables?: Record<string, ExtensionTable>;
}

export class ExtensionContractError extends Error {
  /** Slash-joined location of the offending node; attributes are prefixed with `@`. */
  readonly path: string;
  constructor(path: string, message: string) {
    super(`${path || "<root>"}: ${message}`);
    this.name = "ExtensionContractError";
    this.path = path;
  }
}

export function emptyExtensionTree(): ExtensionTree {
  return { attrs: {}, children: {}, datasets: {} };
}

const INT_RANGES: Record<string, [bigint, bigint]> = {
  i8: [-(2n ** 7n), 2n ** 7n - 1n],
  u8: [0n, 2n ** 8n - 1n],
  i16: [-(2n ** 15n), 2n ** 15n - 1n],
  u16: [0n, 2n ** 16n - 1n],
  i32: [-(2n ** 31n), 2n ** 31n - 1n],
  u32: [0n, 2n ** 32n - 1n],
  i64: [-(2n ** 63n), 2n ** 63n - 1n],
  u64: [0n, 2n ** 64n - 1n],
};

const FLOAT_DTYPES = new Set<ExtensionDtype>(["f32", "f64"]);
const BIGINT_DTYPES = new Set<ExtensionDtype>(["i64", "u64"]);

export const EXTENSION_DTYPES: readonly ExtensionDtype[] = [
  "utf8", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64",
];

export function isExtensionDtype(value: unknown): value is ExtensionDtype {
  return typeof value === "string" && (EXTENSION_DTYPES as readonly string[]).includes(value);
}

/** Names become HDF5 link names: non-empty, no `/`, no NUL. */
export function assertValidNodeName(name: string, path: string): void {
  if (name.length === 0) throw new ExtensionContractError(path, "name must not be empty");
  if (name.includes("/")) throw new ExtensionContractError(path, `name "${name}" must not contain "/"`);
  if (name.includes("\0")) throw new ExtensionContractError(path, "name must not contain NUL");
}

function join(path: string, name: string): string {
  return path ? `${path}/${name}` : name;
}

function assertScalarMatches(dtype: ExtensionDtype, v: unknown, path: string): void {
  if (dtype === "utf8") {
    if (typeof v !== "string") throw new ExtensionContractError(path, `utf8 value must be a string, got ${typeof v}`);
    if (v.includes("\0")) throw new ExtensionContractError(path, "utf8 value must not contain NUL");
    return;
  }
  if (BIGINT_DTYPES.has(dtype)) {
    if (typeof v !== "bigint") throw new ExtensionContractError(path, `${dtype} value must be a bigint, got ${typeof v}`);
  } else if (typeof v !== "number") {
    throw new ExtensionContractError(path, `${dtype} value must be a number, got ${typeof v}`);
  }
  if (FLOAT_DTYPES.has(dtype)) return;
  if (typeof v === "number" && !Number.isInteger(v)) {
    throw new ExtensionContractError(path, `${dtype} value must be an integer, got ${v}`);
  }
  const [min, max] = INT_RANGES[dtype];
  const big = BigInt(v as number | bigint);
  if (big < min || big > max) throw new ExtensionContractError(path, `${dtype} value ${big} is out of range`);
}

function assertAttrs(attrs: Record<string, ExtensionAttr>, path: string): void {
  for (const [name, attr] of Object.entries(attrs)) {
    const attrPath = join(path, `@${name}`);
    assertValidNodeName(name, attrPath);
    if (!isExtensionDtype(attr.dtype)) throw new ExtensionContractError(attrPath, `unknown dtype "${String(attr.dtype)}"`);
    if (Array.isArray(attr.value)) {
      attr.value.forEach((v, i) => assertScalarMatches(attr.dtype, v, `${attrPath}[${i}]`));
    } else {
      assertScalarMatches(attr.dtype, attr.value, attrPath);
    }
  }
}

const TYPED_ARRAY_FOR: Record<Exclude<ExtensionDtype, "utf8">, new (n: number) => ExtensionNumericArray> = {
  i8: Int8Array, u8: Uint8Array, i16: Int16Array, u16: Uint16Array,
  i32: Int32Array, u32: Uint32Array, i64: BigInt64Array, u64: BigUint64Array,
  f32: Float32Array, f64: Float64Array,
};

export function typedArrayConstructorFor(dtype: Exclude<ExtensionDtype, "utf8">): new (n: number) => ExtensionNumericArray {
  return TYPED_ARRAY_FOR[dtype];
}

/** Throws {@link ExtensionContractError} naming the first invalid node. */
export function assertValidExtensionTree(tree: ExtensionTree, path = ""): void {
  assertAttrs(tree.attrs, path);
  const seen = new Set<string>();
  for (const [name, child] of Object.entries(tree.children)) {
    const childPath = join(path, name);
    assertValidNodeName(name, childPath);
    seen.add(name);
    assertValidExtensionTree(child, childPath);
  }
  for (const [name, ds] of Object.entries(tree.datasets)) {
    const dsPath = join(path, name);
    assertValidNodeName(name, dsPath);
    if (seen.has(name)) throw new ExtensionContractError(dsPath, "a dataset and a child group cannot share a name");
    const declared: unknown = ds.dtype;
    if (!isExtensionDtype(declared) || declared === "utf8") {
      throw new ExtensionContractError(dsPath, `dataset dtype must be numeric, got "${String(declared)}"`);
    }
    const expected = ds.shape.reduce((a, b) => a * b, 1);
    if (!ds.shape.every((n) => Number.isInteger(n) && n >= 0)) {
      throw new ExtensionContractError(dsPath, `invalid shape [${ds.shape.join(",")}]`);
    }
    if (!(ds.data instanceof TYPED_ARRAY_FOR[ds.dtype])) {
      throw new ExtensionContractError(dsPath, `data must be a ${TYPED_ARRAY_FOR[ds.dtype].name} for dtype ${ds.dtype}`);
    }
    if (ds.data.length !== expected) {
      throw new ExtensionContractError(dsPath, `data length ${ds.data.length} does not match shape [${ds.shape.join(",")}]`);
    }
    assertAttrs(ds.attrs, dsPath);
  }
  for (const [name, table] of Object.entries(tree.tables ?? {})) {
    const tablePath = join(path, name);
    assertValidNodeName(name, tablePath);
    if (seen.has(name) || name in tree.datasets) {
      throw new ExtensionContractError(tablePath, "a table cannot share a name with a dataset or child group");
    }
    assertValidTable(table, tablePath);
  }
}

function assertValidTable(table: ExtensionTable, path: string): void {
  if (table.fields.length === 0) throw new ExtensionContractError(path, "a table needs at least one field");
  if (!Number.isInteger(table.rows) || table.rows < 0) {
    throw new ExtensionContractError(path, `invalid row count ${table.rows}`);
  }
  const names = new Set<string>();
  for (const field of table.fields) {
    const fieldPath = join(path, field.name);
    assertValidNodeName(field.name, fieldPath);
    if (names.has(field.name)) throw new ExtensionContractError(fieldPath, "duplicate field name");
    names.add(field.name);
    if (!isExtensionDtype(field.dtype)) {
      throw new ExtensionContractError(fieldPath, `unknown dtype "${String(field.dtype)}"`);
    }
    const column = table.columns[field.name];
    if (column === undefined) throw new ExtensionContractError(fieldPath, "missing column");
    if (column.length !== table.rows) {
      throw new ExtensionContractError(fieldPath, `column length ${column.length} does not match ${table.rows} rows`);
    }
    if (field.dtype === "utf8") {
      if (!Number.isInteger(field.size) || (field.size as number) < 1) {
        throw new ExtensionContractError(fieldPath, "a utf8 field needs a positive integer size");
      }
      if (!Array.isArray(column)) throw new ExtensionContractError(fieldPath, "utf8 column must be a string array");
      column.forEach((v, i) => {
        assertScalarMatches("utf8", v, `${fieldPath}[${i}]`);
        const bytes = new TextEncoder().encode(v).length;
        if (bytes > (field.size as number)) {
          throw new ExtensionContractError(`${fieldPath}[${i}]`, `${bytes} UTF-8 bytes do not fit the ${field.size}-byte field`);
        }
      });
    } else {
      if (field.size !== undefined) throw new ExtensionContractError(fieldPath, "size is only valid for utf8 fields");
      if (!(column instanceof TYPED_ARRAY_FOR[field.dtype])) {
        throw new ExtensionContractError(fieldPath, `column must be a ${TYPED_ARRAY_FOR[field.dtype].name} for dtype ${field.dtype}`);
      }
    }
  }
  for (const name of Object.keys(table.columns)) {
    if (!names.has(name)) throw new ExtensionContractError(join(path, name), "column has no matching field");
  }
  assertAttrs(table.attrs, path);
}
