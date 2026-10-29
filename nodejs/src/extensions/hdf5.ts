import * as h5wasm from "h5wasm/node";
import {
  assertValidExtensionTree,
  emptyExtensionTree,
  ExtensionContractError,
  isExtensionDtype,
  typedArrayConstructorFor,
  type ExtensionAttr,
  type ExtensionDtype,
  type ExtensionNumericArray,
  type ExtensionScalar,
  type ExtensionTable,
  type ExtensionTableField,
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

/** H5T_class_t values h5wasm reports in `Metadata.type`. */
const H5T_INTEGER = 0;
const H5T_FLOAT = 1;
const H5T_STRING = 3;

function memberField(m: h5wasm.CompoundMember, path: string): ExtensionTableField {
  if (m.type === H5T_STRING) {
    if (m.vlen) throw new ExtensionContractError(path, "variable-length strings are not supported in a compound type");
    return { name: m.name, dtype: "utf8", size: m.size };
  }
  if (m.littleEndian === false) throw new ExtensionContractError(path, "big-endian compound members are not supported");
  if (m.type === H5T_INTEGER) {
    const bits = m.size * 8;
    const dtype = `${m.signed ? "i" : "u"}${bits}`;
    if (isExtensionDtype(dtype)) return { name: m.name, dtype };
  } else if (m.type === H5T_FLOAT && (m.size === 4 || m.size === 8)) {
    return { name: m.name, dtype: m.size === 4 ? "f32" : "f64" };
  }
  throw new ExtensionContractError(path, `unsupported compound member type (class ${m.type}, ${m.size} bytes)`);
}

function readTable(ds: h5wasm.Dataset, members: h5wasm.CompoundMember[], path: string): ExtensionTable {
  const shape = ds.shape ?? [];
  if (shape.length !== 1) throw new ExtensionContractError(path, `compound datasets must be 1-D, got rank ${shape.length}`);
  const rows = shape[0];
  const fields = members.map((m) => memberField(m, join(path, m.name)));
  const raw = Array.isArray(ds.value) ? (ds.value as unknown[][]) : [];
  const columns: Record<string, ExtensionNumericArray | string[]> = {};
  fields.forEach((f, j) => {
    if (f.dtype === "utf8") {
      columns[f.name] = raw.map((row) => String(row[j]).replace(/\0+$/, ""));
      return;
    }
    const col = new (typedArrayConstructorFor(f.dtype))(rows) as unknown as Record<number, unknown>;
    const wide = f.dtype === "i64" || f.dtype === "u64";
    raw.forEach((row, i) => {
      col[i] = wide ? BigInt(row[j] as number | bigint) : row[j];
    });
    columns[f.name] = col as unknown as ExtensionNumericArray;
  });
  return { fields, rows, columns, attrs: readAttrs(ds.attrs as Record<string, h5wasm.Attribute>, path) };
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
      const compound = entity.metadata.compound_type;
      if (compound !== undefined) {
        (tree.tables ??= {})[name] = readTable(entity, compound.members, childPath);
        continue;
      }
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
  for (const [name, table] of Object.entries(tree.tables ?? {})) {
    group.create_dataset({
      name,
      data: new Map<string, unknown>(table.fields.map((f) => [f.name, table.columns[f.name]])),
      dtype: table.fields.map((f) => [f.name, f.dtype === "utf8" ? `S${f.size}` : H5_DTYPE[f.dtype]]),
      shape: [table.rows],
    });
    writeAttrs(group.get(name) as h5wasm.Dataset, table.attrs);
  }
  for (const [name, child] of Object.entries(tree.children)) {
    writeGroup(group.create_group(name), child);
  }
}
