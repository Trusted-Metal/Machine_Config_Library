package capabilities

import "machine-config-go/capabilities/internal/api"

func snapshot[T any](v T) T {
	return api.Snapshot(v)
}

func applySetMode[T any](current, incoming T, mode SetMode) (T, error) {
	return api.ApplySetMode(current, incoming, mode)
}
