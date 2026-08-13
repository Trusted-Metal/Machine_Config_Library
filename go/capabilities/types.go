package capabilities

import "machine-config-go/capabilities/internal/api"

type SetMode = api.SetMode

const (
	Merge   SetMode = api.Merge
	Replace SetMode = api.Replace
)

type ErrorCode = api.ErrorCode

const (
	ErrUnsupportedVersion ErrorCode = api.ErrUnsupportedVersion
	ErrNotPresent         ErrorCode = api.ErrNotPresent
	ErrInvalidIndex       ErrorCode = api.ErrInvalidIndex
	ErrValidation         ErrorCode = api.ErrValidation
	ErrIo                 ErrorCode = api.ErrIo
	ErrClosed             ErrorCode = api.ErrClosed
)

type Error = api.Error

func errf(code ErrorCode, msg string) *Error {
	return api.Errf(code, msg)
}
