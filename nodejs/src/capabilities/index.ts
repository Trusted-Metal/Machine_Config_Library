/**
 * Stable File_Version facade — peek File_Version, then dispatch to an adapter.
 */
import { err, ok, type Result } from './result.js';
import { capabilityError, type CapabilityError } from './errors.js';
import type { MachineConfigFile } from './generated.js';
import { peekFileVersion } from './file_version.js';
import { MachineConfigFileV1_0 } from './v1_0/index.js';

export type { Result } from './result.js';
export { ok, err, isOk } from './result.js';
export type { CapabilityError, CapabilityErrorCode } from './errors.js';
export { capabilityError, SessionClosedError } from './errors.js';
export { SetMode } from './generated.js';
export type * from './generated.js';
export { MachineConfigFileV1_0 } from './v1_0/index.js';
export { applySetMode, snapshot } from './merge.js';
export { peekFileVersion, UnsupportedFileVersion } from './file_version.js';

const OPEN: Record<
  string,
  (path: string) => Promise<Result<MachineConfigFile, CapabilityError>>
> = {
  '1.0': (path) => MachineConfigFileV1_0.open(path),
};

export async function openMachineConfig(
  path: string,
): Promise<Result<MachineConfigFile, CapabilityError>> {
  try {
    const version = await peekFileVersion(path);
    const opener = OPEN[version];
    if (!opener) {
      return err(
        capabilityError(
          'UnsupportedVersion',
          `No capability adapter registered for File_Version "${version}"`,
        ),
      );
    }
    return opener(path);
  } catch (e) {
    return err(capabilityError('IoError', e instanceof Error ? e.message : String(e)));
  }
}

export function createMachineConfig(
  version: string,
): Result<MachineConfigFile, CapabilityError> {
  const fv = version.trim() || '1.0';
  if (fv !== '1.0') {
    return err(
      capabilityError(
        'UnsupportedVersion',
        `create() unsupported for File_Version "${fv}"`,
      ),
    );
  }
  return MachineConfigFileV1_0.create(fv);
}

export function supportedFileVersions(): string[] {
  return Object.keys(OPEN);
}
