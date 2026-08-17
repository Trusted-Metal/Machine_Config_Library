// S-09: Public type export surface — must compile with zero errors.
// All consumer-facing types must be importable from the package root only.
import type {
  MachineConfig, Scanner, OpticalTrain, LightSource,
  Collimator, ScannerCard, ClearBox, OpcuaConfig,
} from 'machine-config-library';
import { MachineConfigReader, MachineConfigWriter, MockConfigBuilder } from 'machine-config-library';

function assertType<T>(_value: T): void {
  // Compile-only check: reaching tsc --noEmit success means every type above
  // resolved from the package root, with no internal sub-module paths needed.
}

export async function run(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const builder = new MockConfigBuilder();
  const cfg: MachineConfig = builder.build();
  assertType<Scanner>(cfg.optical_trains[0]?.scanner);
  assertType<OpticalTrain>(cfg.optical_trains[0]);
  assertType<LightSource>(cfg.optical_trains[0]?.light_source);
  assertType<Collimator>(cfg.optical_trains[0]?.collimator);
  assertType<ScannerCard>(cfg.optical_trains[0]?.scanner_card);
  assertType<ClearBox | null | undefined>(cfg.optical_trains[0]?.optional_components.clearbox);
  assertType<OpcuaConfig | undefined>(cfg.opcua);
  assertType<typeof MachineConfigReader>(MachineConfigReader);
  assertType<typeof MachineConfigWriter>(MachineConfigWriter);

  return [true, 'all public types importable from machine-config-library top-level'];
}
