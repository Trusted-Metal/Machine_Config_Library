// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md, clean-up phase) removed
// MigrateV1ToV1_1/MigrateV1_1ToV1 from here — the coefficients/points
// shape-conversion functions they used now live in
// machine-config-go/internal/models (ForwardPowerCharacterizationCoefficients/
// Points, BackwardFlatFieldsCoefficients/Points), called as a write-time
// fallback by both capabilities/v1_0/hdf5's and this package's Write, not a
// standalone migration step. See that package's power_characterization.go
// for the full design.
package hdf5
