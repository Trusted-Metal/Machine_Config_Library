/**
 * Shared, version-agnostic conversion between the two on-disk shapes this
 * concept has ever used: the flat shape (an opaque `Algorithm_Type` string
 * plus a CSV `Params` string — File_Version 1.0's `ClearBox.Volts_To_Watts_*`
 * and `LightSource.Watts_To_Volts_*`) and the structured shape
 * ({@link PowerCharacterization} — File_Version 1.1's `Power_Characterization`
 * group).
 *
 * Deliberately independent of both capabilities/v1_0 and capabilities/v1_1 —
 * this module only transforms StableModel types ({@link PowerCharacterization},
 * {@link EquationConstant}, {@link CalibrationPoint}), has no HDF5 I/O, and
 * has no knowledge of "1.0"/"1.1" as version strings. It lives at the same
 * neutral level as `models.ts` so that either version's writer can import it
 * without either version becoming a runtime dependency of the other (see
 * V1_1_IMPLEMENTATION_PLAN.md's Phase 2 "Structural requirement" note) — and
 * so a future version reusing either shape needs zero new code here, only a
 * new pair of functions if a third shape is ever introduced. It also lives
 * outside `src/capabilities/`, so `tests/versionAdapterIsolation.test.ts`
 * (a raw-text scan, not a parser) can never flag it regardless of wording.
 *
 * `power_characterization` is the only StableModel representation of this
 * concept, for files of either version — there is no separate
 * `volts_to_watts_*`/`watts_to_volts_*` field to fall back to. The v1.0
 * reader calls the forward function unconditionally as part of ordinary
 * parsing, to populate `power_characterization` from the on-disk flat
 * attrs; the v1.0 writer calls the backward function unconditionally to
 * derive the flat attrs it writes from `power_characterization`. The v1.1
 * reader/writer read and write `power_characterization` natively, with no
 * conversion at all.
 *
 * (An earlier design — V1_1_IMPLEMENTATION_PLAN.md's Phase 2 — kept both
 * representations on the StableModel side by side, with each writer
 * deriving from the other shape only as a fallback when its own native
 * field was absent, and kept this conversion out of the readers entirely
 * because it only worked for models that came from an actual `parse()`
 * call. That asymmetry — silently empty for any consumer that only ever
 * read the legacy field — is exactly what motivated collapsing to a single
 * field; see POWER_CHARACTERIZATION_UNIFICATION_PLAN.md's "Context"
 * section.)
 *
 * Two shapes exist today:
 * - **Coefficients** (Change 3 / ClearBox): `Params` is a positional,
 *   comma-separated coefficient list. Named to match the generated
 *   equation's own variable names: `b`/`a` for LINEAR, `c0`/`c1`/... for
 *   POLYNOMIAL, or a plain numeric fallback (`"0"`, `"1"`, ...) for any
 *   other value — never an error, see "Never raises" below.
 * - **Points** (Change 4 / Light_Source): `Params` is an alternating
 *   `input_value, output_value` list — direct, mechanical parse, no
 *   coefficient-ordering convention to assume.
 *
 * **Never raises** on an unrecognized `Algorithm_Type`. Earlier design
 * (File_Version 1.1's now-removed `migrateV1ToV1_1`) treated this as a hard
 * error, which made sense when this conversion only ran inside an explicit,
 * opt-in migration call. Once it runs implicitly as part of an ordinary
 * write (this module's actual job now), that would mean an unrelated value
 * elsewhere on the model could fail a write the caller never expected to
 * fail. This is an *incompleteness* problem (less information than the
 * richer shape can hold), not a *conflict* (two present, disagreeing
 * values) — Change 1's Consolidate check is the latter and correctly keeps
 * its hard error in `Hdf5WriterV1_1`, untouched by this module.
 * `algorithm_type` is always copied verbatim (open string, per
 * docs/migrations/v1_0_to_v1_1.md's resolved enum-vs-string decision),
 * coefficients/points are always parsed mechanically, and only
 * `algorithm_equation` is left blank when it can't be generated — round
 * trip stays lossless even for unrecognized types, since writing back
 * re-sorts by the numeric fallback name and reproduces the original CSV
 * order exactly.
 *
 * `input_type`/`units_derived_quantity` are always left blank (`null`) on
 * forward derivation, not hardcoded — confirmed 2026-09-01: a hardcoded
 * guess is a special case that would need re-litigating for every future
 * version; blank is honest about "not derivable from what's on disk."
 *
 * Float formatting deliberately matches this codebase's existing
 * `String(value)` convention (e.g. `"1"`, not `"1.0"`) rather than the other
 * 4 languages' always-include-a-decimal-point convention — that's a
 * pre-existing, real cross-language inconsistency (not introduced by Phase
 * 2, and out of Phase 2's scope to fix), preserved here rather than silently
 * changed.
 */
import type { PowerCharacterization, EquationConstant, CalibrationPoint } from "./models.js";

const RECOGNIZED_ALGORITHM_TYPES = new Set(["LINEAR", "POLYNOMIAL"]);

/**
 * Parses a comma-separated float list. Tolerates the bracketed `"[1,2,3]"`
 * form real Watts_To_Volts_Params fixtures use, as well as the plain
 * `"1,2,3"` form real Volts_To_Watts_Params fixtures use.
 */
function parseCsvFloats(s: string | null | undefined): number[] {
  if (!s) return [];
  const stripped = s.trim().replace(/[[\]]/g, "");
  if (!stripped) return [];
  return stripped
    .split(",")
    .filter((x) => x.trim() !== "")
    .map((x) => Number(x));
}

function generatePowerEquation(algorithmType: string, nConstants: number): string {
  if (algorithmType === "LINEAR") return "W = a*V + b";
  const terms = ["c0"];
  for (let i = 1; i < nConstants; i++) {
    terms.push(i === 1 ? `c${i}*V` : `c${i}*V^${i}`);
  }
  return "W = " + terms.join(" + ");
}

/**
 * Named rows aren't positionally guaranteed on disk — re-sort by name before
 * joining as CSV: `b` before `a` for LINEAR, `c0 < c1 < c2 < ...` by numeric
 * suffix for POLYNOMIAL, `"0" < "1" < "2" < ...` numerically for the
 * unrecognized-algorithm-type fallback naming (see module docs' "Never
 * raises"), anything else falls back to alphabetical by name
 * (natively-authored v1.1 data can use arbitrary names, since Algorithm_Type
 * is an open string).
 */
function sortedConstantsByName(constants: EquationConstant[]): EquationConstant[] {
  function key(c: EquationConstant): [number, number | string] {
    if (c.name === "b") return [0, 0];
    if (c.name === "a") return [0, 1];
    if (/^c\d+$/.test(c.name)) return [1, Number(c.name.slice(1))];
    if (/^\d+$/.test(c.name)) return [2, Number(c.name)];
    return [3, c.name];
  }
  return [...constants].sort((x, y) => {
    const [xa, xb] = key(x);
    const [ya, yb] = key(y);
    if (xa !== ya) return xa - ya;
    if (typeof xb === "number" && typeof yb === "number") return xb - yb;
    return String(xb).localeCompare(String(yb));
  });
}

/**
 * Coefficients-shape forward derivation (Change 3 / ClearBox):
 * `Volts_To_Watts_Algorithm`/`Volts_To_Watts_Params` -> `PowerCharacterization`.
 *
 * `null` if both inputs are `null`/`undefined` (nothing to derive). Never
 * throws for an unrecognized `algorithmType` — see module docs.
 */
export function forwardPowerCharacterizationCoefficients(
  algorithmType: string | null | undefined,
  params: string | null | undefined,
): PowerCharacterization | null {
  if (algorithmType == null && params == null) return null;
  const algo = algorithmType ?? "";
  const values = parseCsvFloats(params);
  let names: string[];
  if (algo === "LINEAR") {
    names = ["b", "a"];
  } else if (algo === "POLYNOMIAL") {
    names = values.map((_, i) => `c${i}`);
  } else {
    names = values.map((_, i) => String(i));
  }
  const n = Math.min(names.length, values.length);
  const constants: EquationConstant[] = [];
  for (let i = 0; i < n; i++) constants.push({ name: names[i], value: values[i] });
  return {
    algorithm_type: algo,
    algorithm_equation: RECOGNIZED_ALGORITHM_TYPES.has(algo)
      ? generatePowerEquation(algo, values.length)
      : null,
    input_type: null,
    units_derived_quantity: null,
    derivation_equation_constants: constants,
    characterization_points: [],
  };
}

/**
 * Points-shape forward derivation (Change 4 / Light_Source):
 * `Watts_To_Volts_Algorithm`/`Watts_To_Volts_Params` -> `PowerCharacterization`.
 *
 * Inverse data-availability from the coefficients shape — direct,
 * mechanical parse, no coefficient-ordering convention to assume. `null` if
 * both inputs are `null`/`undefined`. Never throws for an unrecognized
 * `algorithmType` — see module docs.
 */
export function forwardPowerCharacterizationPoints(
  algorithmType: string | null | undefined,
  params: string | null | undefined,
): PowerCharacterization | null {
  if (algorithmType == null && params == null) return null;
  const algo = algorithmType ?? "";
  const values = parseCsvFloats(params);
  const points: CalibrationPoint[] = [];
  for (let i = 0; i + 1 < values.length; i += 2) {
    points.push({ input_value: values[i], output_value: values[i + 1] });
  }
  // POLYNOMIAL (and the unrecognized-type fallback): left blank, not
  // generated — point count doesn't reliably indicate polynomial degree.
  const equation = algo === "LINEAR" ? "W = a*V + b" : null;
  return {
    algorithm_type: algo,
    algorithm_equation: equation,
    input_type: null,
    units_derived_quantity: null,
    derivation_equation_constants: [],
    characterization_points: points,
  };
}

/**
 * Coefficients-shape backward derivation (Change 3 / ClearBox):
 * `PowerCharacterization` -> `[Algorithm_Type, Volts_To_Watts_Params]`.
 *
 * `[null, null]` if `pc` is `null`/`undefined`. Blank params (`""`, not an
 * error) if `derivation_equation_constants` is empty — absence, not
 * ambiguity, see docs/migrations/v1_0_to_v1_1.md Change 3's "Backward
 * migration" section.
 */
export function backwardFlatFieldsCoefficients(
  pc: PowerCharacterization | null | undefined,
): [string | null, string | null] {
  if (pc == null) return [null, null];
  if (pc.derivation_equation_constants.length === 0) return [pc.algorithm_type, ""];
  const ordered = sortedConstantsByName(pc.derivation_equation_constants);
  return [pc.algorithm_type, ordered.map((c) => String(c.value)).join(",")];
}

/**
 * Points-shape backward derivation (Change 4 / Light_Source):
 * `PowerCharacterization` -> `[Algorithm_Type, Watts_To_Volts_Params]`.
 *
 * `[null, null]` if `pc` is `null`/`undefined`. No re-sort needed —
 * `characterization_points` rows aren't named, and on-disk order is already
 * correct. Blank params if empty, same absence-not-ambiguity reasoning as
 * the coefficients shape.
 */
export function backwardFlatFieldsPoints(
  pc: PowerCharacterization | null | undefined,
): [string | null, string | null] {
  if (pc == null) return [null, null];
  if (pc.characterization_points.length === 0) return [pc.algorithm_type, ""];
  const flat: string[] = [];
  for (const p of pc.characterization_points) {
    flat.push(String(p.input_value), String(p.output_value));
  }
  return [pc.algorithm_type, flat.join(",")];
}
