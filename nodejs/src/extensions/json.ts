import {
  assertValidExtensionTree,
  emptyExtensionTree,
  ExtensionContractError,
  isExtensionDtype,
  typedArrayConstructorFor,
  type ExtensionAttr,
  type ExtensionDataset,
  type ExtensionDtype,
  type ExtensionScalar,
  type ExtensionTree,
} from "./contract.js";
import type { MachineConfig } from "../models.js";

/**
 * Plain-JSON form of a tree, used by fixtures and `toJson()`.
 * i64/u64 become decimal strings and non-finite floats become "NaN" /
 * "Infinity" / "-Infinity", so the JSON is lossless in every language.
 */
export type JsonScalar = string | number;
export interface ExtensionAttrJson { dtype: ExtensionDtype; value: JsonScalar | JsonScalar[] }
export interface ExtensionDatasetJson {
  dtype: Exclude<ExtensionDtype, "utf8">;
  shape: number[];
  data: JsonScalar[];
  attrs: Record<string, ExtensionAttrJson>;
}
export interface ExtensionTreeJson {
  attrs: Record<string, ExtensionAttrJson>;
  children: Record<string, ExtensionTreeJson>;
  datasets: Record<string, ExtensionDatasetJson>;
}

function scalarToJson(dtype: ExtensionDtype, v: ExtensionScalar): JsonScalar {
  if (typeof v === "bigint") return v.toString();
  if (typeof v === "number" && !Number.isFinite(v)) return Number.isNaN(v) ? "NaN" : v > 0 ? "Infinity" : "-Infinity";
  return v;
}

function scalarFromJson(dtype: ExtensionDtype, v: JsonScalar, path: string): ExtensionScalar {
  if (dtype === "utf8") {
    if (typeof v !== "string") throw new ExtensionContractError(path, "utf8 JSON value must be a string");
    return v;
  }
  if (dtype === "i64" || dtype === "u64") {
    if (typeof v !== "string" || !/^-?\d+$/.test(v)) throw new ExtensionContractError(path, `${dtype} JSON value must be a decimal string`);
    return BigInt(v);
  }
  if (typeof v === "string") {
    if (dtype !== "f32" && dtype !== "f64") throw new ExtensionContractError(path, `${dtype} JSON value must be a number`);
    if (v === "NaN") return NaN;
    if (v === "Infinity") return Infinity;
    if (v === "-Infinity") return -Infinity;
    throw new ExtensionContractError(path, `invalid float JSON value "${v}"`);
  }
  return v;
}

function attrsToJson(attrs: Record<string, ExtensionAttr>): Record<string, ExtensionAttrJson> {
  const out: Record<string, ExtensionAttrJson> = {};
  for (const [name, a] of Object.entries(attrs)) {
    out[name] = {
      dtype: a.dtype,
      value: Array.isArray(a.value)
        ? a.value.map((v) => scalarToJson(a.dtype, v))
        : scalarToJson(a.dtype, a.value),
    };
  }
  return out;
}

export function extensionTreeToJson(tree: ExtensionTree): ExtensionTreeJson {
  const children: Record<string, ExtensionTreeJson> = {};
  for (const [name, child] of Object.entries(tree.children)) children[name] = extensionTreeToJson(child);
  const datasets: Record<string, ExtensionDatasetJson> = {};
  for (const [name, ds] of Object.entries(tree.datasets)) {
    datasets[name] = {
      dtype: ds.dtype,
      shape: [...ds.shape],
      data: Array.from(ds.data as ArrayLike<number | bigint>, (v) => scalarToJson(ds.dtype, v)),
      attrs: attrsToJson(ds.attrs),
    };
  }
  return { attrs: attrsToJson(tree.attrs), children, datasets };
}

function attrsFromJson(json: Record<string, ExtensionAttrJson> | undefined, path: string): Record<string, ExtensionAttr> {
  const out: Record<string, ExtensionAttr> = {};
  for (const [name, a] of Object.entries(json ?? {})) {
    const attrPath = `${path}${path ? "/" : ""}@${name}`;
    if (!isExtensionDtype(a.dtype)) throw new ExtensionContractError(attrPath, `unknown dtype "${String(a.dtype)}"`);
    out[name] = {
      dtype: a.dtype,
      value: Array.isArray(a.value)
        ? a.value.map((v, i) => scalarFromJson(a.dtype, v, `${attrPath}[${i}]`))
        : scalarFromJson(a.dtype, a.value, attrPath),
    };
  }
  return out;
}

/** Inverse of {@link extensionTreeToJson}; the result is validated. */
export function extensionTreeFromJson(json: ExtensionTreeJson, path = ""): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs = attrsFromJson(json.attrs, path);
  for (const [name, child] of Object.entries(json.children ?? {})) {
    tree.children[name] = extensionTreeFromJson(child, path ? `${path}/${name}` : name);
  }
  for (const [name, ds] of Object.entries(json.datasets ?? {})) {
    const dsPath = path ? `${path}/${name}` : name;
    const declared: unknown = ds.dtype;
    if (!isExtensionDtype(declared) || declared === "utf8") {
      throw new ExtensionContractError(dsPath, `dataset dtype must be numeric, got "${String(declared)}"`);
    }
    const Ctor = typedArrayConstructorFor(ds.dtype);
    const data = new Ctor(ds.data.length);
    ds.data.forEach((v, i) => {
      (data as unknown as ArrayLike<unknown> & Record<number, unknown>)[i] = scalarFromJson(ds.dtype, v, `${dsPath}[${i}]`);
    });
    const dataset: ExtensionDataset = {
      dtype: ds.dtype,
      shape: [...ds.shape],
      data,
      attrs: attrsFromJson(ds.attrs, dsPath),
    };
    tree.datasets[name] = dataset;
  }
  if (path === "") assertValidExtensionTree(tree);
  return tree;
}

/** Copy of `config` safe to `JSON.stringify` (bigint-free `extensions`). */
export function withExtensionsAsJson(config: MachineConfig): unknown {
  if (config.extensions === undefined) return config;
  const extensions: Record<string, ExtensionTreeJson> = {};
  for (const [name, tree] of Object.entries(config.extensions)) extensions[name] = extensionTreeToJson(tree);
  return { ...config, extensions };
}
