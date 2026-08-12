// Package h5c is a thin CGo wrapper around the HDF5 C library.
// Used because pure-Go scigolib/hdf5 cannot read dense attribute storage
// (groups with >8 attrs) as written by h5py / production machine software.
package h5c

/*
#cgo pkg-config: hdf5
#cgo linux CFLAGS: -I/usr/include/hdf5/serial
#cgo linux LDFLAGS: -L/usr/lib/x86_64-linux-gnu/hdf5/serial -lhdf5
#include <hdf5.h>
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"fmt"
	"unsafe"
)

// File wraps an open HDF5 file (H5F).
type File struct {
	id C.hid_t
}

// Open opens an existing HDF5 file read-only.
func Open(path string) (*File, error) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Fopen(cpath, C.H5F_ACC_RDONLY, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Fopen(%s) failed", path)
	}
	return &File{id: id}, nil
}

// Create truncates/creates a file for writing.
func Create(path string) (*File, error) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Fcreate(cpath, C.H5F_ACC_TRUNC, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Fcreate(%s) failed", path)
	}
	return &File{id: id}, nil
}

// Close closes the file.
func (f *File) Close() error {
	if f == nil || f.id < 0 {
		return nil
	}
	if C.H5Fclose(f.id) < 0 {
		return fmt.Errorf("H5Fclose failed")
	}
	f.id = -1
	return nil
}

// Group opens a group by absolute or relative path from the file root.
func (f *File) Group(path string) (*Group, error) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Gopen2(f.id, cpath, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Gopen2(%s) failed", path)
	}
	return &Group{id: id}, nil
}

// Root returns the root group ("/").
func (f *File) Root() (*Group, error) {
	return f.Group("/")
}

// Group wraps an HDF5 group.
type Group struct {
	id C.hid_t
}

// Close closes the group.
func (g *Group) Close() error {
	if g == nil || g.id < 0 {
		return nil
	}
	if C.H5Gclose(g.id) < 0 {
		return fmt.Errorf("H5Gclose failed")
	}
	g.id = -1
	return nil
}

// OpenGroup opens a subgroup.
func (g *Group) OpenGroup(name string) (*Group, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	id := C.H5Gopen2(g.id, cname, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Gopen2(%s) failed", name)
	}
	return &Group{id: id}, nil
}

// HasAttr reports whether an attribute exists.
func (g *Group) HasAttr(name string) bool {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	return C.H5Aexists(g.id, cname) > 0
}

// ReadStringAttr reads a scalar string attribute (fixed or variable length).
func (g *Group) ReadStringAttr(name string) (string, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	aid := C.H5Aopen(g.id, cname, C.H5P_DEFAULT)
	if aid < 0 {
		return "", fmt.Errorf("H5Aopen(%s) failed", name)
	}
	defer C.H5Aclose(aid)

	tid := C.H5Aget_type(aid)
	if tid < 0 {
		return "", fmt.Errorf("H5Aget_type(%s) failed", name)
	}
	defer C.H5Tclose(tid)

	isVar := C.H5Tis_variable_str(tid) > 0
	if isVar {
		var p *C.char
		if C.H5Aread(aid, tid, unsafe.Pointer(&p)) < 0 {
			return "", fmt.Errorf("H5Aread(%s) varstr failed", name)
		}
		if p == nil {
			return "", nil
		}
		s := C.GoString(p)
		C.H5free_memory(unsafe.Pointer(p))
		return s, nil
	}

	sz := C.H5Tget_size(tid)
	buf := make([]byte, sz)
	if C.H5Aread(aid, tid, unsafe.Pointer(&buf[0])) < 0 {
		return "", fmt.Errorf("H5Aread(%s) failed", name)
	}
	// Trim trailing NULs from fixed-length strings.
	n := 0
	for n < len(buf) && buf[n] != 0 {
		n++
	}
	return string(buf[:n]), nil
}

// ReadFloat64Attr reads a scalar float64 attribute.
func (g *Group) ReadFloat64Attr(name string) (float64, error) {
	var v C.double
	if err := g.readScalar(name, C.H5T_NATIVE_DOUBLE, unsafe.Pointer(&v)); err != nil {
		return 0, err
	}
	return float64(v), nil
}

// ReadInt64Attr reads a scalar int64 attribute.
func (g *Group) ReadInt64Attr(name string) (int64, error) {
	var v C.int64_t
	if err := g.readScalar(name, C.H5T_NATIVE_INT64, unsafe.Pointer(&v)); err != nil {
		return 0, err
	}
	return int64(v), nil
}

func (g *Group) readScalar(name string, memType C.hid_t, dest unsafe.Pointer) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	aid := C.H5Aopen(g.id, cname, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Aopen(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if C.H5Aread(aid, memType, dest) < 0 {
		return fmt.Errorf("H5Aread(%s) failed", name)
	}
	return nil
}

// Dataset opens a dataset by path from this group (or absolute from file if g is root).
type Dataset struct {
	id C.hid_t
}

// OpenDataset opens a dataset relative to the group.
func (g *Group) OpenDataset(name string) (*Dataset, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	id := C.H5Dopen2(g.id, cname, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dopen2(%s) failed", name)
	}
	return &Dataset{id: id}, nil
}

// Close closes the dataset.
func (d *Dataset) Close() error {
	if d == nil || d.id < 0 {
		return nil
	}
	if C.H5Dclose(d.id) < 0 {
		return fmt.Errorf("H5Dclose failed")
	}
	d.id = -1
	return nil
}

// Dims returns the simple dataspace dimensions.
func (d *Dataset) Dims() ([]uint64, error) {
	space := C.H5Dget_space(d.id)
	if space < 0 {
		return nil, fmt.Errorf("H5Dget_space failed")
	}
	defer C.H5Sclose(space)
	ndims := C.H5Sget_simple_extent_ndims(space)
	if ndims < 0 {
		return nil, fmt.Errorf("H5Sget_simple_extent_ndims failed")
	}
	dims := make([]C.hsize_t, ndims)
	if C.H5Sget_simple_extent_dims(space, &dims[0], nil) < 0 {
		return nil, fmt.Errorf("H5Sget_simple_extent_dims failed")
	}
	out := make([]uint64, ndims)
	for i, v := range dims {
		out[i] = uint64(v)
	}
	return out, nil
}

// ReadFloat64 reads the entire dataset into a float64 slice (caller sizes it).
func (d *Dataset) ReadFloat64(dst []float64) error {
	if len(dst) == 0 {
		return fmt.Errorf("empty destination")
	}
	if C.H5Dread(d.id, C.H5T_NATIVE_DOUBLE, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&dst[0])) < 0 {
		return fmt.Errorf("H5Dread failed")
	}
	return nil
}

// LinkExists reports whether a named link exists under the group.
func (g *Group) LinkExists(name string) bool {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	return C.H5Lexists(g.id, cname, C.H5P_DEFAULT) > 0
}

// OpenDatasetPath opens a dataset by absolute path from the file.
func (f *File) OpenDataset(path string) (*Dataset, error) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Dopen2(f.id, cpath, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dopen2(%s) failed", path)
	}
	return &Dataset{id: id}, nil
}

// CreateGroup creates a group (parents must exist, or use nested CreateGroup).
func (g *Group) CreateGroup(name string) (*Group, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	id := C.H5Gcreate2(g.id, cname, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Gcreate2(%s) failed", name)
	}
	return &Group{id: id}, nil
}

// WriteStringAttr writes a variable-length UTF-8 string attribute.
func (g *Group) WriteStringAttr(name, value string) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	cval := C.CString(value)
	defer C.free(unsafe.Pointer(cval))

	tid := C.H5Tcopy(C.H5T_C_S1)
	if tid < 0 {
		return fmt.Errorf("H5Tcopy failed")
	}
	defer C.H5Tclose(tid)
	if C.H5Tset_size(tid, C.H5T_VARIABLE) < 0 {
		return fmt.Errorf("H5Tset_size VARIABLE failed")
	}
	if C.H5Tset_cset(tid, C.H5T_CSET_UTF8) < 0 {
		return fmt.Errorf("H5Tset_cset failed")
	}
	space := C.H5Screate(C.H5S_SCALAR)
	if space < 0 {
		return fmt.Errorf("H5Screate SCALAR failed")
	}
	defer C.H5Sclose(space)
	aid := C.H5Acreate2(g.id, cname, tid, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Acreate2(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if C.H5Awrite(aid, tid, unsafe.Pointer(&cval)) < 0 {
		return fmt.Errorf("H5Awrite(%s) failed", name)
	}
	return nil
}

// WriteFloat64Attr writes a scalar float64 attribute.
func (g *Group) WriteFloat64Attr(name string, value float64) error {
	return g.writeScalar(name, C.H5T_NATIVE_DOUBLE, unsafe.Pointer(&value))
}

// WriteInt64Attr writes a scalar int64 attribute.
func (g *Group) WriteInt64Attr(name string, value int64) error {
	v := C.int64_t(value)
	return g.writeScalar(name, C.H5T_NATIVE_INT64, unsafe.Pointer(&v))
}

func (g *Group) writeScalar(name string, memType C.hid_t, src unsafe.Pointer) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	space := C.H5Screate(C.H5S_SCALAR)
	if space < 0 {
		return fmt.Errorf("H5Screate SCALAR failed")
	}
	defer C.H5Sclose(space)
	aid := C.H5Acreate2(g.id, cname, memType, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Acreate2(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if C.H5Awrite(aid, memType, src) < 0 {
		return fmt.Errorf("H5Awrite(%s) failed", name)
	}
	return nil
}

// CreateFloat64Dataset creates and writes a contiguous float64 dataset.
func (g *Group) CreateFloat64Dataset(name string, dims []uint64, data []float64) error {
	if len(dims) == 0 {
		return fmt.Errorf("empty dims")
	}
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	cdims := make([]C.hsize_t, len(dims))
	var n uint64 = 1
	for i, d := range dims {
		cdims[i] = C.hsize_t(d)
		n *= d
	}
	if uint64(len(data)) != n {
		return fmt.Errorf("data len %d != product of dims %d", len(data), n)
	}
	space := C.H5Screate_simple(C.int(len(dims)), &cdims[0], nil)
	if space < 0 {
		return fmt.Errorf("H5Screate_simple failed")
	}
	defer C.H5Sclose(space)
	id := C.H5Dcreate2(g.id, cname, C.H5T_NATIVE_DOUBLE, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return fmt.Errorf("H5Dcreate2(%s) failed", name)
	}
	defer C.H5Dclose(id)
	if C.H5Dwrite(id, C.H5T_NATIVE_DOUBLE, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&data[0])) < 0 {
		return fmt.Errorf("H5Dwrite(%s) failed", name)
	}
	return nil
}

// ReadUint8 reads the entire dataset as bytes.
func (d *Dataset) ReadUint8(dst []byte) error {
	if len(dst) == 0 {
		return fmt.Errorf("empty destination")
	}
	if C.H5Dread(d.id, C.H5T_NATIVE_UINT8, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&dst[0])) < 0 {
		return fmt.Errorf("H5Dread uint8 failed")
	}
	return nil
}

// AttrTypeKind classifies an attribute's stored type for helper readers.
type AttrTypeKind int

const (
	AttrMissing AttrTypeKind = iota
	AttrString
	AttrFloat
	AttrInt
	AttrOther
)

// PeekAttr returns whether the attr exists and a coarse type kind.
func (g *Group) PeekAttr(name string) (AttrTypeKind, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	if C.H5Aexists(g.id, cname) <= 0 {
		return AttrMissing, nil
	}
	aid := C.H5Aopen(g.id, cname, C.H5P_DEFAULT)
	if aid < 0 {
		return AttrOther, fmt.Errorf("H5Aopen(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	tid := C.H5Aget_type(aid)
	if tid < 0 {
		return AttrOther, fmt.Errorf("H5Aget_type(%s) failed", name)
	}
	defer C.H5Tclose(tid)
	cls := C.H5Tget_class(tid)
	switch cls {
	case C.H5T_STRING:
		return AttrString, nil
	case C.H5T_FLOAT:
		return AttrFloat, nil
	case C.H5T_INTEGER:
		return AttrInt, nil
	default:
		return AttrOther, nil
	}
}
