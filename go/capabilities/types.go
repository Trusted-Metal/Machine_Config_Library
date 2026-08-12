package capabilities

// SetMode selects merge vs replace semantics for set* APIs.
type SetMode int

const (
	Merge SetMode = iota
	Replace
)

// ErrorCode matches schema/capabilities/errors.yaml.
type ErrorCode string

const (
	ErrUnsupportedVersion ErrorCode = "UnsupportedVersion"
	ErrNotPresent         ErrorCode = "NotPresent"
	ErrInvalidIndex       ErrorCode = "InvalidIndex"
	ErrValidation         ErrorCode = "ValidationError"
	ErrIo                 ErrorCode = "IoError"
	ErrClosed             ErrorCode = "Closed"
)

// Error is a typed capability-boundary error.
type Error struct {
	Code    ErrorCode
	Message string
}

func (e *Error) Error() string { return e.Message }

func errf(code ErrorCode, msg string) *Error {
	return &Error{Code: code, Message: msg}
}
