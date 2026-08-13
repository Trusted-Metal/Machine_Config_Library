package machineconfig

import (
	"fmt"
	"strings"

	"machine-config-go/internal/h5c"
)

// UnsupportedFileVersionError is returned when no adapter is registered for File_Version.
type UnsupportedFileVersionError struct {
	Version string
}

func (e *UnsupportedFileVersionError) Error() string {
	return fmt.Sprintf("no adapter registered for File_Version %q", e.Version)
}

// PeekFileVersion reads only the root File_Version attribute. It does not walk groups.
func PeekFileVersion(path string) (string, error) {
	f, err := h5c.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()
	root, err := f.Root()
	if err != nil {
		return "", err
	}
	defer root.Close()
	s, err := root.ReadStringAttr("File_Version")
	if err != nil {
		return "1.0", nil
	}
	s = strings.TrimSpace(s)
	if s == "" {
		return "1.0", nil
	}
	return s, nil
}
