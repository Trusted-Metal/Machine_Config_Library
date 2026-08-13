// AUTO-GENERATED from schema/adapters/test_v0_9_to_v1_0.yaml
// DO NOT EDIT MANUALLY — regenerate with: python tools/generate_adapters.py
import type { Adapter } from './base.js';

export const fromVersion = '0.9';
export const toVersion   = '1.0';

export class V0_9_to_V1_0 implements Adapter {
    readonly name = 'test_v0_9_to_v1_0';

    adapt(config: Record<string, unknown>): Record<string, unknown> {
        const trains = (config['optical_trains'] as Record<string, unknown>[]) ?? [];
        // field_add: test_added_field (path: optical_trains[*])
        for (const train of trains) {
            if (!Object.prototype.hasOwnProperty.call(train, 'test_added_field')) {
                train['test_added_field'] = null;
            }
        }
        // field_rename: test_old_name -> test_new_name (path: optical_trains[*])
        for (const train of trains) {
            if (Object.prototype.hasOwnProperty.call(train, 'test_old_name')) {
                train['test_new_name'] = train['test_old_name'];
                delete train['test_old_name'];
            }
        }
        // field_remove: test_removed_field dropped (path: optical_trains[*])
        for (const train of trains) {
            delete train['test_removed_field'];
        }
        // field_move: test_move_field (optical_trains[*] -> optical_trains[*].test_nested)
        for (const train of trains) {
            if (Object.prototype.hasOwnProperty.call(train, 'test_move_field')) {
                if (typeof train['test_nested'] !== 'object' || train['test_nested'] === null) {
                    train['test_nested'] = {};
                }
                (train['test_nested'] as Record<string, unknown>)['test_move_field'] = train['test_move_field'];
                delete train['test_move_field'];
            }
        }
        // field_move_rename: test_move_rename_old -> test_nested.test_move_rename_new (optical_trains[*] -> optical_trains[*].test_nested)
        for (const train of trains) {
            if (Object.prototype.hasOwnProperty.call(train, 'test_move_rename_old')) {
                if (typeof train['test_nested'] !== 'object' || train['test_nested'] === null) {
                    train['test_nested'] = {};
                }
                (train['test_nested'] as Record<string, unknown>)['test_move_rename_new'] = train['test_move_rename_old'];
                delete train['test_move_rename_old'];
            }
        }
        return config;
    }
}
