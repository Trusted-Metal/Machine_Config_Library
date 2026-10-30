/**
 * Peek on-disk File_Version without assuming HDF5 group layout.
 * Root attribute File_Version is the adapter key.
 */
import * as h5wasm from 'h5wasm/node';
import { resolve } from 'node:path';

export const FILE_VERSION_ATTR = 'File_Version';

export class UnsupportedFileVersion extends Error {
  readonly version: string;
  constructor(version: string) {
    super(
      `No adapter registered for File_Version "${version}". Each on-disk version needs its own adapter.`,
    );
    this.name = 'UnsupportedFileVersion';
    this.version = version;
  }
}

let _readyPromise: Promise<void> | null = null;

function ensureReady(): Promise<void> {
  if (_readyPromise === null) {
    _readyPromise = h5wasm.ready.then(() => undefined);
  }
  return _readyPromise;
}

function normPath(p: string): string {
  return resolve(p).replace(/\\/g, '/');
}

function attrVersion(attrs: Record<string, h5wasm.Attribute>): string {
  const attr = attrs[FILE_VERSION_ATTR];
  if (attr == null || attr.value == null) return '1.0';
  const v = attr.value;
  const s = String(ArrayBuffer.isView(v) && 'length' in v ? (v as ArrayLike<unknown>)[0] : v).trim();
  return s || '1.0';
}

/** Read only the root File_Version attribute. Does not walk groups. */
export async function peekFileVersion(path: string): Promise<string> {
  await ensureReady();
  const f = new h5wasm.File(normPath(path), 'r');
  try {
    return attrVersion(f.attrs as Record<string, h5wasm.Attribute>);
  } finally {
    f.close();
  }
}
