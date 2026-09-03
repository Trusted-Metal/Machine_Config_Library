/**
 * Unit tests for src/powerCharacterization.ts — the shared,
 * version-agnostic shape-conversion functions Phase 2 clean-up (see the
 * migration implementation plan) introduced to replace
 * migrateV1ToV1_1/migrateV1_1ToV1. Pure functions, no HDF5 I/O —
 * integration with the real writers is covered separately in
 * v1_1Adapter.test.ts. Mirrors the other 4 languages' shared-module tests.
 */
import { describe, expect, it } from 'vitest';

import type { PowerCharacterization } from '../src/models.js';
import {
  backwardFlatFieldsCoefficients,
  backwardFlatFieldsPoints,
  forwardPowerCharacterizationCoefficients,
  forwardPowerCharacterizationPoints,
} from '../src/powerCharacterization.js';

function names(constants: { name: string }[]): string[] {
  return constants.map((c) => c.name);
}

// ---------------------------------------------------------------------------
// forwardPowerCharacterizationCoefficients (Change 3 / ClearBox shape)
// ---------------------------------------------------------------------------

describe('forwardPowerCharacterizationCoefficients', () => {
  it('returns null when both inputs are null', () => {
    expect(forwardPowerCharacterizationCoefficients(null, null)).toBeNull();
  });

  it('handles LINEAR', () => {
    const pc = forwardPowerCharacterizationCoefficients('LINEAR', '50.0,100.0')!;
    expect(pc.algorithm_type).toBe('LINEAR');
    expect(pc.algorithm_equation).toBe('W = a*V + b');
    expect(names(pc.derivation_equation_constants)).toEqual(['b', 'a']);
    expect(pc.derivation_equation_constants.map((c) => c.value)).toEqual([50.0, 100.0]);
    expect(pc.characterization_points).toEqual([]);
  });

  it('handles POLYNOMIAL', () => {
    const pc = forwardPowerCharacterizationCoefficients('POLYNOMIAL', '1.0,2.0,3.0')!;
    expect(pc.algorithm_type).toBe('POLYNOMIAL');
    expect(pc.algorithm_equation).toBe('W = c0 + c1*V + c2*V^2');
    expect(names(pc.derivation_equation_constants)).toEqual(['c0', 'c1', 'c2']);
  });

  it('leaves input_type/units_derived_quantity blank, not hardcoded', () => {
    // Confirmed 2026-09-01: blank, not hardcoded — see powerCharacterization.ts's docs.
    const pc = forwardPowerCharacterizationCoefficients('LINEAR', '50.0,100.0')!;
    expect(pc.input_type).toBeNull();
    expect(pc.units_derived_quantity).toBeNull();
  });

  it('never throws for an unrecognized algorithm type', () => {
    // Never a hard error — see powerCharacterization.ts's "Never raises" docs.
    const pc = forwardPowerCharacterizationCoefficients('EXPONENTIAL', '1.5,2.5,3.5')!;
    expect(pc.algorithm_type).toBe('EXPONENTIAL');
    expect(pc.algorithm_equation).toBeNull();
    expect(names(pc.derivation_equation_constants)).toEqual(['0', '1', '2']);
    expect(pc.derivation_equation_constants.map((c) => c.value)).toEqual([1.5, 2.5, 3.5]);
  });
});

// ---------------------------------------------------------------------------
// forwardPowerCharacterizationPoints (Change 4 / Light_Source shape)
// ---------------------------------------------------------------------------

describe('forwardPowerCharacterizationPoints', () => {
  it('returns null when both inputs are null', () => {
    expect(forwardPowerCharacterizationPoints(null, null)).toBeNull();
  });

  it('handles LINEAR', () => {
    const pc = forwardPowerCharacterizationPoints('LINEAR', '1,100,10,1000')!;
    expect(pc.algorithm_type).toBe('LINEAR');
    expect(pc.algorithm_equation).toBe('W = a*V + b');
    expect(pc.characterization_points).toEqual([
      { input_value: 1, output_value: 100 },
      { input_value: 10, output_value: 1000 },
    ]);
    expect(pc.derivation_equation_constants).toEqual([]);
  });

  it('leaves the equation blank for POLYNOMIAL', () => {
    // Point count doesn't reliably indicate polynomial degree — never
    // generated for POLYNOMIAL, unlike the coefficients shape.
    const pc = forwardPowerCharacterizationPoints('POLYNOMIAL', '1,100,10,1000')!;
    expect(pc.algorithm_equation).toBeNull();
  });

  it('never throws for an unrecognized algorithm type', () => {
    const pc = forwardPowerCharacterizationPoints('QUADRATIC', '1,100,10,1000')!;
    expect(pc.algorithm_type).toBe('QUADRATIC');
    expect(pc.algorithm_equation).toBeNull();
    expect(pc.characterization_points.length).toBe(2);
  });

  it('strips brackets', () => {
    // Real Watts_To_Volts_Params fixtures wrap the CSV in brackets, unlike
    // Volts_To_Watts_Params's plain form.
    const pc = forwardPowerCharacterizationPoints('LINEAR', '[1,100,10,1000]')!;
    expect(pc.characterization_points).toEqual([
      { input_value: 1, output_value: 100 },
      { input_value: 10, output_value: 1000 },
    ]);
  });

  it('leaves input_type/units_derived_quantity blank', () => {
    const pc = forwardPowerCharacterizationPoints('LINEAR', '1,100')!;
    expect(pc.input_type).toBeNull();
    expect(pc.units_derived_quantity).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// backwardFlatFieldsCoefficients (Change 3 backward)
// ---------------------------------------------------------------------------

describe('backwardFlatFieldsCoefficients', () => {
  it('returns [null, null] for a null pc', () => {
    expect(backwardFlatFieldsCoefficients(null)).toEqual([null, null]);
  });

  it('writes blank params, not an error, for empty constants', () => {
    const pc: PowerCharacterization = {
      algorithm_type: 'LINEAR',
      algorithm_equation: 'W = a*V + b',
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [],
      characterization_points: [],
    };
    expect(backwardFlatFieldsCoefficients(pc)).toEqual(['LINEAR', '']);
  });

  it('reorders by name (b before a for LINEAR)', () => {
    // Rows aren't positionally guaranteed on disk — must re-sort before
    // joining as CSV.
    const pc: PowerCharacterization = {
      algorithm_type: 'LINEAR',
      algorithm_equation: null,
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [
        { name: 'a', value: 2 },
        { name: 'b', value: 1 },
      ],
      characterization_points: [],
    };
    expect(backwardFlatFieldsCoefficients(pc)).toEqual(['LINEAR', '1,2']);
  });

  it('reorders POLYNOMIAL constants by numeric suffix', () => {
    const pc: PowerCharacterization = {
      algorithm_type: 'POLYNOMIAL',
      algorithm_equation: null,
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [
        { name: 'c2', value: 3 },
        { name: 'c0', value: 1 },
        { name: 'c1', value: 2 },
      ],
      characterization_points: [],
    };
    const [, params] = backwardFlatFieldsCoefficients(pc);
    expect(params).toBe('1,2,3');
  });

  it('reorders positional fallback names numerically, not lexically', () => {
    // Unrecognized-algorithm-type fallback names ('0','1',...) must sort
    // numerically (else '10' would sort before '2').
    const pc: PowerCharacterization = {
      algorithm_type: 'EXPONENTIAL',
      algorithm_equation: null,
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [
        { name: '1', value: 2.5 },
        { name: '0', value: 1.5 },
        { name: '2', value: 3.5 },
      ],
      characterization_points: [],
    };
    expect(backwardFlatFieldsCoefficients(pc)).toEqual(['EXPONENTIAL', '1.5,2.5,3.5']);
  });

  it('round-trips an unrecognized type through forward then backward', () => {
    const originalAlgorithm = 'EXPONENTIAL';
    const originalParams = '1.5,2.5,3.5';
    const pc = forwardPowerCharacterizationCoefficients(originalAlgorithm, originalParams);
    expect(backwardFlatFieldsCoefficients(pc)).toEqual([originalAlgorithm, originalParams]);
  });
});

// ---------------------------------------------------------------------------
// backwardFlatFieldsPoints (Change 4 backward)
// ---------------------------------------------------------------------------

describe('backwardFlatFieldsPoints', () => {
  it('returns [null, null] for a null pc', () => {
    expect(backwardFlatFieldsPoints(null)).toEqual([null, null]);
  });

  it('writes blank params, not an error, for empty points', () => {
    const pc: PowerCharacterization = {
      algorithm_type: 'LINEAR',
      algorithm_equation: 'W = a*V + b',
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [],
      characterization_points: [],
    };
    expect(backwardFlatFieldsPoints(pc)).toEqual(['LINEAR', '']);
  });

  it('needs no re-sort — Characterization_Points rows are unnamed', () => {
    const pc: PowerCharacterization = {
      algorithm_type: 'LINEAR',
      algorithm_equation: null,
      input_type: null,
      units_derived_quantity: null,
      derivation_equation_constants: [],
      characterization_points: [
        { input_value: 1, output_value: 100 },
        { input_value: 10, output_value: 1000 },
      ],
    };
    expect(backwardFlatFieldsPoints(pc)).toEqual(['LINEAR', '1,100,10,1000']);
  });

  it('round-trips through forward then backward', () => {
    const originalAlgorithm = 'LINEAR';
    const originalParams = '1,100,10,1000';
    const pc = forwardPowerCharacterizationPoints(originalAlgorithm, originalParams);
    expect(backwardFlatFieldsPoints(pc)).toEqual([originalAlgorithm, originalParams]);
  });
});
