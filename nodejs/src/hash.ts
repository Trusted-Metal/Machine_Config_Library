/**
 * SHA-256 configuration-integrity hash for {@link MachineConfig}.
 *
 * Computed over the canonical, `include_binary = false` JSON shape of the
 * whole config (the same shape `toJson()`/`export-json` produce), minus
 * `meta.configuration_hash` (can't hash itself) and `meta.export_date`
 * (changes on every re-export even when nothing configuration-wise
 * changed). Object keys are recursively sorted — unlike Python/Rust/C++,
 * this is NOT free in Node (a plain JS object's own JSON.stringify
 * enumeration order is insertion order, not sorted), so this module builds
 * its own compact JSON text directly rather than delegating to
 * `JSON.stringify`.
 *
 * This hash is computed for MCF's own self-consistency across MCF's own
 * five language implementations — it is NOT designed to match any other
 * producer's own hashing scheme (e.g. an external system that may hash data
 * which never survives to the on-disk file at all). A file MCF's own writer
 * produced will read back valid; a file authored by anything else will very
 * likely read back invalid — that's expected, not a bug: it answers "was
 * this MCF-touched file tampered with since MCF itself last touched it,"
 * not "does this match some other system's proprietary algorithm."
 *
 * The `.fc3` bytes (`raw_bytes`) are always excluded regardless of whether the `MachineConfig`
 * passed in happens to carry them (e.g. a config obtained with `includeBinary: true`). Extension
 * trees are hashed in full through their canonical digest, ClearBox's correction grids included.
 *
 * ### Why key names, not paths, drive float-vs-integer rendering
 *
 * JavaScript has no runtime distinction between an int-valued and a
 * float-valued `number` — `250` and `250.0` are the identical value. Every
 * numeric leaf in this domain effectively came from an HDF5 float64 attribute
 * (and must render with a forced trailing `.0` when whole, e.g. `"250.0"`,
 * to agree with Python/Rust/C++'s own native int/float distinction) *except*
 * a small, known set of genuinely integer-typed fields (`file_size`,
 * the bit resolutions). This is a leaf *key name*
 * allowlist (not a full path allowlist), hardcoded below rather than read
 * from `schema/machine_config_v1.schema.json` at runtime — that schema is
 * itself hand-maintained (not generated from the models), so reading it
 * would only relocate the drift risk, not remove it. Instead,
 * `hash.test.ts` has a dedicated test that cross-checks this exact set
 * against the schema's own `"type": "integer"` declarations, so a future
 * mismatch between the two is caught by CI rather than silently producing
 * a wrong hash.
 *
 * This allowlist does not cover dynamically-named passthrough `extra`
 * attributes (arbitrary HDF5 attributes preserved verbatim, e.g.
 * `meta.extra`) — those have no fixed key name to
 * allowlist, and Node's HDF5 reader (`attrExtra`) already collapses both
 * int64 and float64 attribute values to a plain JS `number` with no
 * provenance retained, so a whole-valued float `extra` attribute cannot be
 * told apart from a genuinely integer one at this layer. This is a
 * pre-existing limitation of `toJson()`'s own output (not newly introduced
 * by this hash), so it is out of scope for this feature to fix.
 */
import { createHash } from "node:crypto";

import type { MachineConfig } from "./models.js";
import { canonicalDigest } from "./extensions/digest.js";

/**
 * Every leaf key name in `schema/machine_config_v1.schema.json` declared
 * `"type": "integer"` — kept in sync by `hash.test.ts`'s schema cross-check
 * test, not by reading the schema file at runtime (see module docs above).
 */
export const INTEGER_FIELD_NAMES: ReadonlySet<string> = new Set([
  "actual_bit_resolution",
  "commanded_bit_resolution",
  "file_size",
]);

/** Returns the SHA-256 hex digest of `config`'s canonical, hash-relevant content. */
export function computeConfigurationHash(config: MachineConfig): string {
  const canonical = canonicalizeForHash(config);
  return createHash("sha256").update(canonical, "utf8").digest("hex");
}

/**
 * Returns the exact canonical JSON text that {@link computeConfigurationHash}
 * hashes — exposed (not just an internal detail) so this can be diffed
 * directly against the other languages' equivalents when tracking down a
 * cross-language hash mismatch.
 */
export function canonicalizeForHash(config: MachineConfig): string {
  const sanitized = sanitizeForHash(config);
  const meta: Record<string, unknown> = { ...sanitized.meta };
  delete meta.configuration_hash;
  delete meta.export_date;
  delete meta.is_valid;
  const forHash = { ...sanitized, meta };
  return renderValue(forHash, null);
}

const INVERT_FLAG_KEYS = [
  "invert_actual_x",
  "invert_actual_y",
  "invert_commanded_x",
  "invert_commanded_y",
] as const;

/**
 * Returns a shallow-ish copy of `config` matching the exact shape
 * `toJson()` produces with `includeBinary: false` — the shape this hash is
 * defined over:
 *
 * - The `.fc3` bytes (`raw_bytes`) removed from every optical train,
 *   regardless of whether they were populated — mirrors Python's
 *   `_config_to_dict(config, include_binary=False)`, which always builds
 *   the non-binary shape irrespective of what the input dataclass happens
 *   to carry.
 * - Each `Scanner`'s four `invert_*` keys dropped when exactly `false` —
 *   `toJson()` applies this same rule via its `omitFalseInvertFlags`
 *   replacer, but that replacer runs only inside `JSON.stringify` itself;
 *   this hash builds its own canonical text without going through
 *   `JSON.stringify`, so the rule has to be applied here explicitly instead
 *   of being inherited "for free".
 */
function sanitizeForHash(config: MachineConfig): MachineConfig {
  const { extensions, ...rest } = config;
  const sanitized: Record<string, unknown> = { ...rest };
  // Extension content enters the hash as one canonical digest per extension; absent or empty
  // `extensions` leaves the hash of every existing config unchanged.
  if (extensions !== undefined && Object.keys(extensions).length > 0) {
    sanitized.extensions = Object.fromEntries(
      Object.entries(extensions).map(([name, tree]) => [name, canonicalDigest(tree)]),
    );
  }
  return {
    ...(sanitized as unknown as MachineConfig),
    optical_trains: config.optical_trains.map((train) => {
      let next = train;
      if (next.scan_field_correction_file !== null) {
        const sfcf = { ...next.scan_field_correction_file };
        delete sfcf.raw_bytes;
        next = { ...next, scan_field_correction_file: sfcf };
      }
      const scanner: Record<string, unknown> = { ...next.scanner };
      for (const key of INVERT_FLAG_KEYS) {
        if (scanner[key] === false) delete scanner[key];
      }
      next = { ...next, scanner: scanner as unknown as typeof next.scanner };
      return next;
    }),
  };
}

function renderValue(value: unknown, key: string | null): string {
  if (value === null || value === undefined) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") return renderNumber(value, key);
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) {
    return `[${value.map((v) => renderValue(v, null)).join(",")}]`;
  }
  if (typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const keys = Object.keys(obj)
      .filter((k) => obj[k] !== undefined)
      .sort();
    const entries = keys.map((k) => `${JSON.stringify(k)}:${renderValue(obj[k], k)}`);
    return `{${entries.join(",")}}`;
  }
  throw new TypeError(`Cannot render value of type "${typeof value}" in configuration hash`);
}

function renderNumber(n: number, key: string | null): string {
  // Matches JSON.stringify's own behavior for non-finite numbers.
  if (!Number.isFinite(n)) return "null";
  if (key !== null && INTEGER_FIELD_NAMES.has(key)) return String(n);
  const s = String(n);
  return /[.eE]/.test(s) ? s : `${s}.0`;
}
