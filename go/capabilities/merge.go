package capabilities

import (
	"encoding/json"

	machineconfig "machine-config-go"
)

func snapshot[T any](v T) T {
	b, err := json.Marshal(v)
	if err != nil {
		return v
	}
	var out T
	if err := json.Unmarshal(b, &out); err != nil {
		return v
	}
	return out
}

// applySetMode overlays non-null JSON keys from incoming onto current (Merge),
// or returns a snapshot of incoming (Replace).
func applySetMode[T any](current, incoming T, mode SetMode) (T, error) {
	if mode == Replace {
		return snapshot(incoming), nil
	}
	curB, err := json.Marshal(current)
	if err != nil {
		return current, err
	}
	incB, err := json.Marshal(incoming)
	if err != nil {
		return current, err
	}
	var curMap, incMap map[string]any
	if err := json.Unmarshal(curB, &curMap); err != nil {
		return current, err
	}
	if err := json.Unmarshal(incB, &incMap); err != nil {
		return current, err
	}
	for k, v := range incMap {
		if v != nil {
			curMap[k] = v
		}
	}
	merged, err := json.Marshal(curMap)
	if err != nil {
		return current, err
	}
	var out T
	if err := json.Unmarshal(merged, &out); err != nil {
		return current, err
	}
	return out, nil
}

// ensure we reference machineconfig in this file for gopls when models move
var _ = machineconfig.MachineConfig{}
