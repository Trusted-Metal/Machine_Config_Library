/**
 * File_Version 1.0 keeps ClearBox per optical train under `Optional_Components/ClearBox/`; core
 * exposes it as the opaque `ClearBox` extension tree, always in the File_Version 1.1 layout, so an
 * extension library never has to know which file version a tree came from:
 *
 *     ClearBox/                      Output_Path, Software_Trigger_Delay   (one shared value each)
 *       <train_id>/                  per-train attributes, correction datasets, Synchronous_Sensors/
 *         Power_Characterization/    structured replacement for Volts_To_Watts_Algorithm/Params
 *
 * Everything else in a v1.0 ClearBox group is carried verbatim, including the attributes v1.1
 * dropped (`Selected_Camera`, `Show_Console`, ...), so a v1.0 file round-trips without losing them.
 * The two shared values are taken from the first train that has a ClearBox.
 *
 * Mirrors `rust/src/clearbox_legacy.rs`.
 */
import type { ExtensionAttr, ExtensionTable, ExtensionTree } from "./extensions/contract.js";
import { emptyExtensionTree } from "./extensions/contract.js";
import type { CalibrationPoint, EquationConstant, PowerCharacterization } from "./models.js";
import {
  backwardFlatFieldsCoefficients,
  forwardPowerCharacterizationCoefficients,
} from "./powerCharacterization.js";

export const CLEARBOX_EXTENSION_NAME = "ClearBox";

const POWER_CHARACTERIZATION = "Power_Characterization";
const SHARED_ATTRS = ["Output_Path", "Software_Trigger_Delay"] as const;
const FLAT_ALGORITHM = "Volts_To_Watts_Algorithm";
const FLAT_PARAMS = "Volts_To_Watts_Params";
const V1_1_ONLY_ATTRS = ["Firmware_Version"] as const;
const EQUATION_CONSTANT_NAME_BYTES = 64;

function text(tree: ExtensionTree, name: string): string | null {
  const v = tree.attrs[name]?.value;
  if (typeof v !== "string") return null;
  const s = v.trim();
  return s === "" ? null : s;
}

function strAttr(value: string | null | undefined): ExtensionAttr {
  return { dtype: "utf8", value: value ?? "" };
}

function powerCharacterizationToTree(pc: PowerCharacterization | null): ExtensionTree {
  const tree = emptyExtensionTree();
  tree.attrs["Algorithm_Type"] = strAttr(pc?.algorithm_type);
  tree.attrs["Algorithm_Equation"] = strAttr(pc?.algorithm_equation);
  tree.attrs["Input_Type"] = strAttr(pc?.input_type);
  tree.attrs["Units_Derived_Quantity"] = strAttr(pc?.units_derived_quantity);
  const constants: EquationConstant[] = pc?.derivation_equation_constants ?? [];
  const points: CalibrationPoint[] = pc?.characterization_points ?? [];
  tree.tables = {
    Derivation_Equation_Constants: {
      fields: [
        { name: "name", dtype: "utf8", size: EQUATION_CONSTANT_NAME_BYTES },
        { name: "value", dtype: "f64" },
      ],
      rows: constants.length,
      columns: {
        name: constants.map((c) => c.name),
        value: Float64Array.from(constants.map((c) => c.value)),
      },
      attrs: {},
    },
    Characterization_Points: {
      fields: [
        { name: "input_value", dtype: "f64" },
        { name: "output_value", dtype: "f64" },
      ],
      rows: points.length,
      columns: {
        input_value: Float64Array.from(points.map((p) => p.input_value)),
        output_value: Float64Array.from(points.map((p) => p.output_value)),
      },
      attrs: {},
    },
  };
  return tree;
}

function powerCharacterizationFromTree(tree: ExtensionTree): PowerCharacterization {
  const constantsTable: ExtensionTable | undefined = tree.tables?.["Derivation_Equation_Constants"];
  const names = constantsTable?.columns["name"];
  const values = constantsTable?.columns["value"];
  const constants: EquationConstant[] =
    Array.isArray(names) && values !== undefined
      ? names.map((name, i) => ({ name, value: Number((values as ArrayLike<number | bigint>)[i]) }))
      : [];
  const pointsTable: ExtensionTable | undefined = tree.tables?.["Characterization_Points"];
  const inputs = pointsTable?.columns["input_value"];
  const outputs = pointsTable?.columns["output_value"];
  const points: CalibrationPoint[] = [];
  if (inputs !== undefined && outputs !== undefined && !Array.isArray(inputs) && !Array.isArray(outputs)) {
    for (let i = 0; i < inputs.length; i++) {
      points.push({
        input_value: Number((inputs as ArrayLike<number | bigint>)[i]),
        output_value: Number((outputs as ArrayLike<number | bigint>)[i]),
      });
    }
  }
  return {
    algorithm_type: text(tree, "Algorithm_Type"),
    algorithm_equation: text(tree, "Algorithm_Equation"),
    input_type: text(tree, "Input_Type"),
    units_derived_quantity: text(tree, "Units_Derived_Quantity"),
    derivation_equation_constants: constants,
    characterization_points: points,
  };
}

/**
 * Builds the `ClearBox` tree from each train's verbatim v1.0 group, in train order.
 * `null` when no train has a ClearBox.
 */
export function clearboxFromV1_0Trains(trains: Array<[string, ExtensionTree]>): ExtensionTree | null {
  if (trains.length === 0) return null;
  const root = emptyExtensionTree();
  const first = trains[0][1];
  for (const name of SHARED_ATTRS) {
    if (name in first.attrs) root.attrs[name] = first.attrs[name];
  }
  for (const [trainId, tree] of trains) {
    for (const name of SHARED_ATTRS) delete tree.attrs[name];
    const algorithm = text(tree, FLAT_ALGORITHM);
    const params = text(tree, FLAT_PARAMS);
    delete tree.attrs[FLAT_ALGORITHM];
    delete tree.attrs[FLAT_PARAMS];
    const pc = forwardPowerCharacterizationCoefficients(algorithm, params);
    if (POWER_CHARACTERIZATION in tree.children) {
      throw new Error(`${trainId}: a File_Version 1.0 ClearBox group cannot contain ${POWER_CHARACTERIZATION}`);
    }
    tree.children[POWER_CHARACTERIZATION] = powerCharacterizationToTree(pc);
    root.children[trainId] = tree;
  }
  return root;
}

/**
 * Splits the `ClearBox` tree back into one verbatim v1.0 group per train id.
 *
 * Throws rather than dropping anything v1.0 has no place for: attributes or datasets directly on
 * the extension root other than the two shared values.
 */
export function clearboxToV1_0Trains(tree: ExtensionTree): Record<string, ExtensionTree> {
  const lossy = (what: string): Error =>
    new Error(`ClearBox: ${what} has no File_Version 1.0 location`);
  const stray = Object.keys(tree.attrs).find((k) => !(SHARED_ATTRS as readonly string[]).includes(k));
  if (stray !== undefined) throw lossy(`attribute "${stray}"`);
  const strayDataset = [...Object.keys(tree.datasets), ...Object.keys(tree.tables ?? {})][0];
  if (strayDataset !== undefined) throw lossy(`dataset "${strayDataset}"`);

  const out: Record<string, ExtensionTree> = {};
  for (const [trainId, child] of Object.entries(tree.children)) {
    const group: ExtensionTree = structuredClone(child);
    for (const name of SHARED_ATTRS) {
      if (!(name in group.attrs) && name in tree.attrs) group.attrs[name] = tree.attrs[name];
    }
    for (const name of V1_1_ONLY_ATTRS) delete group.attrs[name];
    const pcTree = group.children[POWER_CHARACTERIZATION];
    delete group.children[POWER_CHARACTERIZATION];
    const pc = pcTree === undefined ? null : powerCharacterizationFromTree(pcTree);
    const [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    group.attrs[FLAT_ALGORITHM] = strAttr(algorithm);
    group.attrs[FLAT_PARAMS] = strAttr(params);
    out[trainId] = group;
  }
  return out;
}
