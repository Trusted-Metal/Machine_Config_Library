package capabilities

import (
	v1_0 "machine-config-go/capabilities/v1_0"
	machineconfig "machine-config-go"
)

// File is the active adapter session. Today this is always the v1.0 facade.
type File = v1_0.File

// OpenMachineConfig peeks File_Version and dispatches to that version's adapter.
func OpenMachineConfig(path string) (*File, *Error) {
	fv, err := machineconfig.PeekFileVersion(path)
	if err != nil {
		return nil, errf(ErrIo, err.Error())
	}
	if fv != v1_0.FileVersion {
		return nil, errf(ErrUnsupportedVersion, "No adapter for File_Version \""+fv+"\"")
	}
	return v1_0.Open(path)
}

// CreateMachineConfig builds a new in-memory session for the given File_Version.
func CreateMachineConfig(version string) (*File, *Error) {
	if version == "" {
		version = v1_0.FileVersion
	}
	if version != v1_0.FileVersion {
		return nil, errf(ErrUnsupportedVersion, "create() unsupported for File_Version \""+version+"\"")
	}
	return v1_0.Create(version)
}

// SupportedFileVersions lists registered adapter keys.
func SupportedFileVersions() []string { return []string{v1_0.FileVersion} }
