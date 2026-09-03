/**
 * Guard test: no version's adapter (or its test-only mock) may reference a
 * *different* version's adapter, even in a comment or string — not just via
 * a static `import`. Reasoning: if v1.1 depends on v1.0's code, v1.0 can
 * never be changed or removed later without checking v1.1 (and every later
 * version compounds this). See `nodejs/tests/mockV1_1.ts`'s module doc for
 * the full rationale — this test is what keeps that fix from regressing.
 *
 * Implementation is intentionally low-tech: read each relevant file's raw
 * text and scan for tokens matching `v` + digits + `_` + digits (e.g.
 * "v1_0", "v1_1") with a plain regex — no parser, no new dependency. Each
 * file's own version is derived the same way, from its own path. Any other
 * version token found in the file's contents is a violation.
 *
 * This currently passes trivially (only `v1_0` is real, plus the
 * newly-self-contained `mockV1_1.ts`), but starts enforcing the moment a
 * real `v1_1` (or later) folder is added under `nodejs/src/capabilities/`.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const __dirname = dirname(fileURLToPath(import.meta.url));
const NODEJS_ROOT = join(__dirname, '..');
const CAPABILITIES_DIR = join(NODEJS_ROOT, 'src', 'capabilities');

const VERSION_TOKEN_RE = /v\d+_\d+/gi;
const VERSION_DIR_RE = /^v\d+_\d+$/i;

/** Recursively collect every file path under `dir`. */
function listFilesRecursive(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...listFilesRecursive(full));
    } else {
      out.push(full);
    }
  }
  return out;
}

/** Every `vX_Y` version directory that currently exists under capabilities/. */
function versionDirs(): string[] {
  return readdirSync(CAPABILITIES_DIR).filter((name) => VERSION_DIR_RE.test(name));
}

/**
 * A relevant file, plus the single version token it is allowed to mention
 * (derived from its own path/filename — the version it *belongs* to).
 */
interface CheckedFile {
  path: string;
  ownVersion: string;
}

function collectCheckedFiles(): CheckedFile[] {
  const files: CheckedFile[] = [];

  for (const versionDir of versionDirs()) {
    const ownVersion = versionDir.toLowerCase();
    const fullDir = join(CAPABILITIES_DIR, versionDir);
    for (const file of listFilesRecursive(fullDir)) {
      files.push({ path: file, ownVersion });
    }
  }

  // Test-only mocks that must be equally self-contained. mockV1_1.ts isn't
  // inside a vX_Y-named directory, so its "own version" comes from its
  // filename instead.
  const mockPath = join(NODEJS_ROOT, 'tests', 'mockV1_1.ts');
  const mockMatch = 'mockV1_1.ts'.match(VERSION_TOKEN_RE);
  if (mockMatch) {
    files.push({ path: mockPath, ownVersion: mockMatch[0].toLowerCase() });
  }

  return files;
}

describe('version adapter isolation', () => {
  it('no adapter (or its test-only mock) references a different version', () => {
    const checked = collectCheckedFiles();
    // Sanity check: this test should never silently check zero files (e.g.
    // if paths above drift from the real layout).
    expect(checked.length).toBeGreaterThan(0);

    const violations: string[] = [];

    for (const { path, ownVersion } of checked) {
      const text = readFileSync(path, 'utf-8');
      const tokens = text.match(VERSION_TOKEN_RE) ?? [];
      const foreignTokens = new Set(
        tokens.map((t) => t.toLowerCase()).filter((t) => t !== ownVersion),
      );
      if (foreignTokens.size > 0) {
        const rel = relative(NODEJS_ROOT, path).replace(/\\/g, '/');
        violations.push(`${rel} (own version ${ownVersion}) references: ${[...foreignTokens].join(', ')}`);
      }
    }

    expect(violations, violations.join('\n')).toEqual([]);
  });
});
