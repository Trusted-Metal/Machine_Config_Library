package api

import (
	"encoding/json"
)

type SetMode int

const (
	Merge SetMode = iota
	Replace
)

type ErrorCode string

const (
	ErrUnsupportedVersion ErrorCode = "UnsupportedVersion"
	ErrNotPresent         ErrorCode = "NotPresent"
	ErrInvalidIndex       ErrorCode = "InvalidIndex"
	ErrValidation         ErrorCode = "ValidationError"
	ErrIo                 ErrorCode = "IoError"
	ErrClosed             ErrorCode = "Closed"
)

type Error struct {
	Code    ErrorCode
	Message string
}

func (e *Error) Error() string { return e.Message }

func Errf(code ErrorCode, msg string) *Error {
	return &Error{Code: code, Message: msg}
}

func Snapshot[T any](v T) T {
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

func ApplySetMode[T any](current, incoming T, mode SetMode) (T, error) {
	if mode == Replace {
		return Snapshot(incoming), nil
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
