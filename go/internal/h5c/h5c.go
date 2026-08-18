// Package h5c is a CGo wrapper around the HDF5 C library.
// It is the complete HDF5 I/O layer for this module (read + write).
package h5c

/*
#cgo linux pkg-config: hdf5
#cgo linux CFLAGS: -I/usr/include/hdf5/serial
#cgo linux LDFLAGS: -L/usr/lib/x86_64-linux-gnu/hdf5/serial -lhdf5
#cgo windows CFLAGS: -I/mingw64/include
#cgo windows LDFLAGS: -L/mingw64/lib -lhdf5
#include <hdf5.h>
#include <stdlib.h>
#include <string.h>

// h5c_list_t is shared by both attribute and link enumeration.
// Caps: 256 entries, names up to 255 chars.
typedef struct {
	char names[256][256];
	int  count;
} h5c_list_t;

// h5c_check_scalar returns 0 if aid is a scalar (1 element), -1 otherwise.
static int h5c_check_scalar(hid_t aid) {
	hid_t space = H5Aget_space(aid);
	if (space < 0) return -1;
	hssize_t n = H5Sget_simple_extent_npoints(space);
	H5Sclose(space);
	return (n == 1) ? 0 : -1;
}

// h5c_list_attrs fills *list with attribute names; HDF5 errors are suppressed
// globally by Open/Create so the stop-iteration failure is silent.
static int h5c_list_attrs(hid_t id, h5c_list_t *list) {
	list->count = 0;
	for (hsize_t i = 0; i < 256; i++) {
		ssize_t sz = H5Aget_name_by_idx(id, ".", H5_INDEX_NAME, H5_ITER_INC,
		                                 i, list->names[list->count], 255, H5P_DEFAULT);
		if (sz < 0) break;
		if (sz > 0) {
			list->names[list->count][255] = '\0';
			list->count++;
		}
	}
	return 0;
}

// h5c_list_links fills *list with child link names (HDF5 1.8+).
static int h5c_list_links(hid_t id, h5c_list_t *list) {
	H5G_info_t ginfo;
	list->count = 0;
	if (H5Gget_info(id, &ginfo) < 0) return -1;
	hsize_t n = ginfo.nlinks;
	if (n > 256) n = 256;
	for (hsize_t i = 0; i < n; i++) {
		ssize_t sz = H5Lget_name_by_idx(id, ".", H5_INDEX_NAME, H5_ITER_INC,
		                                 i, list->names[list->count], 255, H5P_DEFAULT);
		if (sz > 0) {
			list->names[list->count][255] = '\0';
			list->count++;
		}
	}
	return 0;
}// h5c_read_any_attr reads a scalar attribute of any type.
// On success *type_out is set: 1=string, 2=float64, 3=int64.
// strbuf/strsz used for string results; fval/ival for numeric results.
static herr_t h5c_read_any_attr(hid_t id, const char *name,
                                 char *strbuf, size_t strsz,
                                 double *fval, int64_t *ival,
                                 int *type_out) {
	*type_out = 0;
	hid_t aid = H5Aopen(id, name, H5P_DEFAULT);
	if (aid < 0) return -1;
	if (h5c_check_scalar(aid) < 0) { H5Aclose(aid); return -1; }
	hid_t tid = H5Aget_type(aid);
	if (tid < 0) { H5Aclose(aid); return -1; }
	H5T_class_t cls = H5Tget_class(tid);
	herr_t ret = -1;
	if (cls == H5T_STRING) {
		hbool_t is_var = H5Tis_variable_str(tid);
		if (is_var) {
			char *p = NULL;
			ret = H5Aread(aid, tid, &p);
			if (ret >= 0 && p != NULL) {
				strncpy(strbuf, p, strsz - 1);
				strbuf[strsz - 1] = '\0';
				H5free_memory(p);
				*type_out = 1;
			}
		} else {
			size_t sz = H5Tget_size(tid);
			if (sz < strsz) {
				memset(strbuf, 0, strsz);
				ret = H5Aread(aid, tid, strbuf);
				if (ret >= 0) *type_out = 1;
			}
		}
	} else if (cls == H5T_FLOAT) {
		ret = H5Aread(aid, H5T_NATIVE_DOUBLE, fval);
		if (ret >= 0) *type_out = 2;
	} else if (cls == H5T_INTEGER) {
		ret = H5Aread(aid, H5T_NATIVE_INT64, ival);
		if (ret >= 0) *type_out = 3;
	}
	H5Tclose(tid);
	H5Aclose(aid);
	return ret;
}
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
	C.H5Eset_auto2(C.H5E_DEFAULT, nil, nil) // suppress HDF5 stderr for this process
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
	C.H5Eset_auto2(C.H5E_DEFAULT, nil, nil)
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Fcreate(cpath, C.H5F_ACC_TRUNC, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Fcreate(%s) failed", path)
	}
	return &File{id: id}, nil
}

// OpenRW opens an existing HDF5 file for in-place read/write, without
// truncating it (unlike Create). Used by test-only code that patches an
// already-written file in place (see go/internal/mockv1_1).
func OpenRW(path string) (*File, error) {
	C.H5Eset_auto2(C.H5E_DEFAULT, nil, nil)
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	id := C.H5Fopen(cpath, C.H5F_ACC_RDWR, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Fopen(%s, RDWR) failed", path)
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

// DeleteAttr deletes an attribute if present; a no-op if absent. Used by
// test-only code that renames/moves an attribute by writing the new one and
// deleting the old (see go/internal/mockv1_1).
func (g *Group) DeleteAttr(name string) error {
	if !g.HasAttr(name) {
		return nil
	}
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	if C.H5Adelete(g.id, cname) < 0 {
		return fmt.Errorf("H5Adelete(%s) failed", name)
	}
	return nil
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
	if C.h5c_check_scalar(aid) < 0 {
		return fmt.Errorf("attribute %q is not scalar", name)
	}
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

// SubGroupNames returns the names of all links (children) in the group.
// Callers that need only subgroups should verify with LinkExists/OpenGroup.
func (g *Group) SubGroupNames() []string {
	var list C.h5c_list_t
	if C.h5c_list_links(g.id, &list) < 0 {
		return nil
	}
	names := make([]string, int(list.count))
	for i := 0; i < int(list.count); i++ {
		names[i] = C.GoString(&list.names[i][0])
	}
	return names
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

// CreateFloat64DatasetOpen creates and writes a float64 dataset and returns it open.
// The caller must call Close() on the returned Dataset.
func (g *Group) CreateFloat64DatasetOpen(name string, dims []uint64, data []float64) (*Dataset, error) {
	if len(dims) == 0 {
		return nil, fmt.Errorf("empty dims")
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
		return nil, fmt.Errorf("data len %d != product of dims %d", len(data), n)
	}
	space := C.H5Screate_simple(C.int(len(dims)), &cdims[0], nil)
	if space < 0 {
		return nil, fmt.Errorf("H5Screate_simple failed")
	}
	defer C.H5Sclose(space)
	id := C.H5Dcreate2(g.id, cname, C.H5T_NATIVE_DOUBLE, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dcreate2(%s) failed", name)
	}
	if C.H5Dwrite(id, C.H5T_NATIVE_DOUBLE, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&data[0])) < 0 {
		C.H5Dclose(id)
		return nil, fmt.Errorf("H5Dwrite(%s) failed", name)
	}
	return &Dataset{id: id}, nil
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

// AttrNames returns the names of all attributes on the group.
func (g *Group) AttrNames() []string {
	return listAttrNames(g.id)
}

// ReadAnyAttrValue reads a scalar attribute of any supported type and returns
// (string|float64|int64, true) on success, or (nil, false) on failure/unsupported.
func (g *Group) ReadAnyAttrValue(name string) (any, bool) {
	return readAnyAttrValue(g.id, name)
}

// HasAttr reports whether an attribute exists on the dataset.
func (d *Dataset) HasAttr(name string) bool {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	return C.H5Aexists(d.id, cname) > 0
}

// ReadStringAttr reads a scalar string attribute from the dataset.
func (d *Dataset) ReadStringAttr(name string) (string, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	aid := C.H5Aopen(d.id, cname, C.H5P_DEFAULT)
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
	n := 0
	for n < len(buf) && buf[n] != 0 {
		n++
	}
	return string(buf[:n]), nil
}

// ReadInt64Attr reads a scalar int64 attribute from the dataset.
func (d *Dataset) ReadInt64Attr(name string) (int64, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	aid := C.H5Aopen(d.id, cname, C.H5P_DEFAULT)
	if aid < 0 {
		return 0, fmt.Errorf("H5Aopen(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	var v C.int64_t
	if C.H5Aread(aid, C.H5T_NATIVE_INT64, unsafe.Pointer(&v)) < 0 {
		return 0, fmt.Errorf("H5Aread(%s) failed", name)
	}
	return int64(v), nil
}

// AttrNames returns the names of all attributes on the dataset.
func (d *Dataset) AttrNames() []string {
	return listAttrNames(d.id)
}

// ReadAnyAttrValue reads a scalar attribute from the dataset.
func (d *Dataset) ReadAnyAttrValue(name string) (any, bool) {
	return readAnyAttrValue(d.id, name)
}

// listAttrNames returns all attribute names for the given HDF5 object id.
func listAttrNames(id C.hid_t) []string {
	var list C.h5c_list_t
	if C.h5c_list_attrs(id, &list) < 0 {
		return nil
	}
	names := make([]string, int(list.count))
	for i := 0; i < int(list.count); i++ {
		names[i] = C.GoString(&list.names[i][0])
	}
	return names
}

// readAnyAttrValue reads a scalar attribute of any supported type.
func readAnyAttrValue(id C.hid_t, name string) (any, bool) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	var strbuf [1024]C.char
	var fval C.double
	var ival C.int64_t
	var typeOut C.int
	ret := C.h5c_read_any_attr(id, cname, &strbuf[0], C.size_t(len(strbuf)), &fval, &ival, &typeOut)
	if ret < 0 {
		return nil, false
	}
	switch int(typeOut) {
	case 1:
		return C.GoString(&strbuf[0]), true
	case 2:
		return float64(fval), true
	case 3:
		return int64(ival), true
	default:
		return nil, false
	}
}

// WriteStringAttr writes a variable-length UTF-8 string attribute on the dataset.
func (d *Dataset) WriteStringAttr(name, value string) error {
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
	aid := C.H5Acreate2(d.id, cname, tid, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Acreate2(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if C.H5Awrite(aid, tid, unsafe.Pointer(&cval)) < 0 {
		return fmt.Errorf("H5Awrite(%s) failed", name)
	}
	return nil
}

// WriteInt64Attr writes a scalar int64 attribute on the dataset.
func (d *Dataset) WriteInt64Attr(name string, value int64) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	v := C.int64_t(value)
	space := C.H5Screate(C.H5S_SCALAR)
	if space < 0 {
		return fmt.Errorf("H5Screate SCALAR failed")
	}
	defer C.H5Sclose(space)
	aid := C.H5Acreate2(d.id, cname, C.H5T_NATIVE_INT64, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Acreate2(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if C.H5Awrite(aid, C.H5T_NATIVE_INT64, unsafe.Pointer(&v)) < 0 {
		return fmt.Errorf("H5Awrite(%s) failed", name)
	}
	return nil
}

// CreateUint8Dataset creates and writes a contiguous 1-D uint8 dataset.
// If data is empty, a zero-element dataset is created with no write.
func (g *Group) CreateUint8Dataset(name string, data []byte) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	n := C.hsize_t(len(data))
	space := C.H5Screate_simple(1, &n, nil)
	if space < 0 {
		return fmt.Errorf("H5Screate_simple failed")
	}
	defer C.H5Sclose(space)
	id := C.H5Dcreate2(g.id, cname, C.H5T_NATIVE_UINT8, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return fmt.Errorf("H5Dcreate2(%s) failed", name)
	}
	defer C.H5Dclose(id)
	if len(data) > 0 {
		if C.H5Dwrite(id, C.H5T_NATIVE_UINT8, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&data[0])) < 0 {
			return fmt.Errorf("H5Dwrite(%s) failed", name)
		}
	}
	return nil
}

// CreateUint8DatasetOpen creates and writes a 1-D uint8 dataset and returns it open.
// The caller must call Close() on the returned Dataset.
func (g *Group) CreateUint8DatasetOpen(name string, data []byte) (*Dataset, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	n := C.hsize_t(len(data))
	space := C.H5Screate_simple(1, &n, nil)
	if space < 0 {
		return nil, fmt.Errorf("H5Screate_simple failed")
	}
	defer C.H5Sclose(space)
	id := C.H5Dcreate2(g.id, cname, C.H5T_NATIVE_UINT8, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dcreate2(%s) failed", name)
	}
	if len(data) > 0 {
		if C.H5Dwrite(id, C.H5T_NATIVE_UINT8, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&data[0])) < 0 {
			C.H5Dclose(id)
			return nil, fmt.Errorf("H5Dwrite(%s) failed", name)
		}
	}
	return &Dataset{id: id}, nil
}
