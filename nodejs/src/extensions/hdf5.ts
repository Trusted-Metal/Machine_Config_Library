import * as h5wasm from "h5wasm/node";
import {
  assertValidExtensionTree,
  emptyExtensionTree,
  ExtensionContractError,
  typedArrayConstructorFor,
  type ExtensionAttr,
  type ExtensionDtype,
  type ExtensionNumericArray,
  type ExtensionScalar,
  type ExtensionTree,
} from "./contract.js";

/** h5wasm dtype strings (struct-style, as h5wasm reports them) for each contract dtype. */
const H5_DTYPE: Record<Exclude<ExtensionDtype, "utf8">, string> = {
  i8: "<b", u8: "<B", i16: "<h", u16: "<H", i32: "<i", u32: "<I",
  i64: "<q", u64: "<Q", f32: "<f", f64: "<d",
};

const DTYPE_FROM_H5: Record<string, ExtensionDtype> = {
  S: "utf8",
  "<b": "i8", "<B": "u8", "<h": "i16", "<H": "u16", "<i": "i32", "<I": "u32",
  "<q": "i64", "<Q": "u64", "<f": "f32", "<d": "f64",
};

function dtypeFromH5(dtype: unknown, path: string): ExtensionDtype {
  const mapped = typeof dtype === "string" ? DTYPE_FROM_H5[dtype] : undefined;
  if (mapped === undefined) {
    throw new ExtensionContractError(path, `unsupported HDF5 type ${JSON.stringify(dtype)}`);
  }
  return mapped;
}

function join(path: string, name: string): string {
  return path ? `${path}/${name}` : name;
}

function readAttr(attr: h5wasm.Attribute, path: string): ExtensionAttr {
  const dtype = dtypeFromH5(attr.dtype, path);
  const shape = attr.shape ?? [];
  const v = attr.value;
  if (shape.length === 0) {
    return { dtype, value: v as ExtensionScalar };
  }
  if (shape.length !== 1) throw new ExtensionContractError(path, `attributes must be scalar or 1-D, got rank ${shape.length}`);
  return { dtype, value: Array.from(v as ArrayLike<ExtensionScalar>) };
}

function readAttrs(attrs: Record<string, h5wasm.Attribute>, path: string): Record<string, ExtensionAttr> {
  const out: Record<string, ExtensionAttr> = {};
  for (const [name, attr] of Object.entries(attrs)) out[name] = readAttr(attr, join(path, `@${name}`));
  return out;
}

/** Reads a group and everything below it verbatim. Unsupported HDF5 types fail loudly with their path. */
export function readExtensionTree(group: h5wasm.Group, path = ""): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs = readAttrs(group.attrs as Record<string, h5wasm.Attribute>, path);
  for (const name of group.keys()) {
    const childPath = join(path, name);
    const entity = group.get(name);
    if (entity instanceof h5wasm.Group) {
      tree.children[name] = readExtensionTree(entity, childPath);
    } else if (entity instanceof h5wasm.Dataset) {
      const dtype = dtypeFromH5(entity.dtype, childPath);
      if (dtype === "utf8") throw new ExtensionContractError(childPath, "string datasets are not supported");
      const raw = entity.value;
      const Ctor = typedArrayConstructorFor(dtype);
      const data: ExtensionNumericArray = ArrayBuffer.isView(raw)
        ? (new (Ctor as unknown as new (src: ArrayLike<unknown>) => ExtensionNumericArray)(raw as unknown as ArrayLike<unknown>))
        : (new (Ctor as unknown as new (src: unknown[]) => ExtensionNumericArray)([raw]));
      tree.datasets[name] = {
        dtype,
        shape: [...((entity.shape ?? []) as number[])],
        data,
        attrs: readAttrs(entity.attrs as Record<string, h5wasm.Attribute>, childPath),
      };
    } else {
      throw new ExtensionContractError(childPath, "only groups and datasets are supported");
    }
  }
  return tree;
}

function writeAttrs(target: h5wasm.Group | h5wasm.Dataset, attrs: Record<string, ExtensionAttr>): void {
  for (const [name, attr] of Object.entries(attrs)) {
    if (attr.dtype === "utf8") {
      if (Array.isArray(attr.value)) target.create_attribute(name, attr.value as string[], [attr.value.length]);
      else target.create_attribute(name, attr.value as string);
      continue;
    }
    const h5 = H5_DTYPE[attr.dtype];
    if (Array.isArray(attr.value)) {
      const Ctor = typedArrayConstructorFor(attr.dtype) as unknown as new (src: unknown[]) => ExtensionNumericArray;
      target.create_attribute(name, new Ctor(attr.value), [attr.value.length], h5);
    } else {
      target.create_attribute(name, attr.value as unknown as number, [], h5);
    }
  }
}

/** Writes `tree` as a new group `name` under `parent`; the caller removes or replaces any previous group. */
export function writeExtensionTree(parent: h5wasm.Group, name: string, tree: ExtensionTree): void {
  assertValidExtensionTree(tree, name);
  writeGroup(parent.create_group(name), tree);
}

function writeGroup(group: h5wasm.Group, tree: ExtensionTree): void {
  writeAttrs(group, tree.attrs);
  for (const [name, ds] of Object.entries(tree.datasets)) {
    group.create_dataset({ name, data: ds.data, shape: ds.shape, dtype: H5_DTYPE[ds.dtype] });
    writeAttrs(group.get(name) as h5wasm.Dataset, ds.attrs);
  }
  for (const [name, child] of Object.entries(tree.children)) {
    writeGroup(group.create_group(name), child);
  }
}
