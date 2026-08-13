import { describe, it, expect } from 'vitest';
import { V0_9_to_V1_0, fromVersion, toVersion } from '../src/adapters/test_v0_9_to_v1_0.js';
import { getChainFor } from '../src/adapters/index.js';

function makeConfig(...keys: string[]): Record<string, unknown> {
    const train: Record<string, unknown> = {};
    for (const k of keys) train[k] = 'val';
    return { optical_trains: [train] };
}

function train(result: Record<string, unknown>): Record<string, unknown> {
    return (result['optical_trains'] as Record<string, unknown>[])[0];
}

const adapter = new V0_9_to_V1_0();

describe('V0_9_to_V1_0 adapter unit tests', () => {
    it('field_add adds test_added_field as null', () => {
        const result = adapter.adapt(makeConfig());
        expect(train(result)['test_added_field']).toBeNull();
    });

    it('field_rename moves value from test_old_name to test_new_name', () => {
        const result = adapter.adapt(makeConfig('test_old_name'));
        const t = train(result);
        expect(t['test_new_name']).toBe('val');
        expect(Object.prototype.hasOwnProperty.call(t, 'test_old_name')).toBe(false);
    });

    it('field_remove removes test_removed_field', () => {
        const result = adapter.adapt(makeConfig('test_removed_field'));
        expect(Object.prototype.hasOwnProperty.call(train(result), 'test_removed_field')).toBe(false);
    });

    it('field_move moves test_move_field into test_nested', () => {
        const result = adapter.adapt(makeConfig('test_move_field'));
        const t = train(result);
        expect(Object.prototype.hasOwnProperty.call(t, 'test_move_field')).toBe(false);
        expect((t['test_nested'] as Record<string, unknown>)['test_move_field']).toBe('val');
    });

    it('field_move_rename moves and renames into test_nested', () => {
        const result = adapter.adapt(makeConfig('test_move_rename_old'));
        const t = train(result);
        expect(Object.prototype.hasOwnProperty.call(t, 'test_move_rename_old')).toBe(false);
        expect((t['test_nested'] as Record<string, unknown>)['test_move_rename_new']).toBe('val');
    });

    it('adapter metadata', () => {
        expect(fromVersion).toBe('0.9');
        expect(toVersion).toBe('1.0');
    });

    it('getChainFor returns adapter for 0.9→1.0', () => {
        expect(getChainFor('0.9', '1.0').length).toBeGreaterThan(0);
    });

    it('getChainFor returns empty for 1.0→1.0', () => {
        expect(getChainFor('1.0', '1.0')).toEqual([]);
    });
});
