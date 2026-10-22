package h5c

/*
#include <hdf5.h>
#include <stdlib.h>
#include <string.h>

// h5c_attr_name_quiet is H5Aget_name_by_idx with HDF5's error printing suspended around the call:
// enumeration deliberately probes past the last attribute, which fails by design.
static ssize_t h5c_attr_name_quiet(hid_t id, hsize_t i, char *buf, size_t size) {
	H5E_auto2_t func;
	void *data;
	H5Eget_auto2(H5E_DEFAULT, &func, &data);
	H5Eset_auto2(H5E_DEFAULT, NULL, NULL);
	ssize_t n = H5Aget_name_by_idx(id, ".", H5_INDEX_NAME, H5_ITER_INC, i, buf, size, H5P_DEFAULT);
	H5Eset_auto2(H5E_DEFAULT, func, data);
	return n;
}
*/
import "C"

import (
	"fmt"
	"unsafe"
)

// Generic, typed attribute and dataset access used to read and write extension trees verbatim.
//
// Every value is carried in one of four Go shapes, chosen by the stored HDF5 type:
// signed integers -> int64, unsigned integers -> uint64, floats (32/64-bit) -> float64,
// strings -> string. The exact stored width is kept in TypedValue.Dtype ("i8" ... "f64", "utf8").

// TypedValue is an attribute value or a dataset's flat data, tagged with its stored dtype.
// Exactly one of Strs/Ints/Uints/Floats is populated, matching Dtype.
type TypedValue struct {
	Dtype  string
	Scalar bool // attributes only: stored with a scalar dataspace rather than a 1-D one
	Strs   []string
	Ints   []int64
	Uints  []uint64
	Floats []float64
}

// Len returns the number of elements held.
func (v *TypedValue) Len() int {
	switch v.Dtype {
	case "utf8":
		return len(v.Strs)
	case "i8", "i16", "i32", "i64":
		return len(v.Ints)
	case "u8", "u16", "u32", "u64":
		return len(v.Uints)
	default:
		return len(v.Floats)
	}
}

// Member is one child link of a group.
type Member struct {
	Name      string
	IsGroup   bool
	IsDataset bool
}

// ---------------------------------------------------------------------------
// dtype <-> HDF5 type mapping
// ---------------------------------------------------------------------------

func fileTypeFor(dtype string) (C.hid_t, error) {
	switch dtype {
	case "i8":
		return C.H5T_STD_I8LE, nil
	case "u8":
		return C.H5T_STD_U8LE, nil
	case "i16":
		return C.H5T_STD_I16LE, nil
	case "u16":
		return C.H5T_STD_U16LE, nil
	case "i32":
		return C.H5T_STD_I32LE, nil
	case "u32":
		return C.H5T_STD_U32LE, nil
	case "i64":
		return C.H5T_STD_I64LE, nil
	case "u64":
		return C.H5T_STD_U64LE, nil
	case "f32":
		return C.H5T_IEEE_F32LE, nil
	case "f64":
		return C.H5T_IEEE_F64LE, nil
	}
	return 0, fmt.Errorf("unknown dtype %q", dtype)
}

// dtypeOf classifies an HDF5 datatype id; "" means unsupported.
func dtypeOf(tid C.hid_t) string {
	size := int(C.H5Tget_size(tid))
	switch C.H5Tget_class(tid) {
	case C.H5T_STRING:
		return "utf8"
	case C.H5T_INTEGER:
		signed := C.H5Tget_sign(tid) != C.H5T_SGN_NONE
		switch size {
		case 1, 2, 4, 8:
			if signed {
				return fmt.Sprintf("i%d", size*8)
			}
			return fmt.Sprintf("u%d", size*8)
		}
	case C.H5T_FLOAT:
		switch size {
		case 4, 8:
			return fmt.Sprintf("f%d", size*8)
		}
	}
	return ""
}

func isSignedDtype(d string) bool   { return d == "i8" || d == "i16" || d == "i32" || d == "i64" }
func isUnsignedDtype(d string) bool { return d == "u8" || d == "u16" || d == "u32" || d == "u64" }

// ---------------------------------------------------------------------------
// element buffers
// ---------------------------------------------------------------------------

func readBuffer(dtype string, n int, read func(memType C.hid_t, buf unsafe.Pointer) C.herr_t) (TypedValue, error) {
	out := TypedValue{Dtype: dtype}
	if n == 0 {
		return out, nil
	}
	switch {
	case isSignedDtype(dtype):
		out.Ints = make([]int64, n)
		if read(C.H5T_NATIVE_INT64, unsafe.Pointer(&out.Ints[0])) < 0 {
			return out, fmt.Errorf("read failed")
		}
	case isUnsignedDtype(dtype):
		out.Uints = make([]uint64, n)
		if read(C.H5T_NATIVE_UINT64, unsafe.Pointer(&out.Uints[0])) < 0 {
			return out, fmt.Errorf("read failed")
		}
	default:
		out.Floats = make([]float64, n)
		if read(C.H5T_NATIVE_DOUBLE, unsafe.Pointer(&out.Floats[0])) < 0 {
			return out, fmt.Errorf("read failed")
		}
	}
	return out, nil
}

// writeBuffer calls write with the memory type and a pointer to v's elements.
func writeBuffer(v *TypedValue, write func(memType C.hid_t, buf unsafe.Pointer) C.herr_t) error {
	var dummy int64
	switch {
	case isSignedDtype(v.Dtype):
		p := unsafe.Pointer(&dummy)
		if len(v.Ints) > 0 {
			p = unsafe.Pointer(&v.Ints[0])
		}
		if write(C.H5T_NATIVE_INT64, p) < 0 {
			return fmt.Errorf("write failed")
		}
	case isUnsignedDtype(v.Dtype):
		p := unsafe.Pointer(&dummy)
		if len(v.Uints) > 0 {
			p = unsafe.Pointer(&v.Uints[0])
		}
		if write(C.H5T_NATIVE_UINT64, p) < 0 {
			return fmt.Errorf("write failed")
		}
	default:
		p := unsafe.Pointer(&dummy)
		if len(v.Floats) > 0 {
			p = unsafe.Pointer(&v.Floats[0])
		}
		if write(C.H5T_NATIVE_DOUBLE, p) < 0 {
			return fmt.Errorf("write failed")
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// attributes (object-agnostic: groups and datasets share these)
// ---------------------------------------------------------------------------

func allAttrNames(id C.hid_t) []string {
	var names []string
	for i := C.hsize_t(0); ; i++ {
		sz := C.h5c_attr_name_quiet(id, i, nil, 0)
		if sz < 0 {
			break
		}
		buf := make([]byte, int(sz)+1)
		C.h5c_attr_name_quiet(id, i, (*C.char)(unsafe.Pointer(&buf[0])), C.size_t(len(buf)))
		names = append(names, string(buf[:sz]))
	}
	return names
}

func readTypedAttr(id C.hid_t, name string) (TypedValue, error) {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	aid := C.H5Aopen(id, cname, C.H5P_DEFAULT)
	if aid < 0 {
		return TypedValue{}, fmt.Errorf("H5Aopen(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	tid := C.H5Aget_type(aid)
	if tid < 0 {
		return TypedValue{}, fmt.Errorf("H5Aget_type(%s) failed", name)
	}
	defer C.H5Tclose(tid)
	dtype := dtypeOf(tid)
	if dtype == "" {
		return TypedValue{}, fmt.Errorf("unsupported HDF5 type")
	}

	space := C.H5Aget_space(aid)
	if space < 0 {
		return TypedValue{}, fmt.Errorf("H5Aget_space(%s) failed", name)
	}
	defer C.H5Sclose(space)
	rank := int(C.H5Sget_simple_extent_ndims(space))
	if rank > 1 {
		return TypedValue{}, fmt.Errorf("attributes must be scalar or 1-D, got rank %d", rank)
	}
	n := int(C.H5Sget_simple_extent_npoints(space))

	if dtype == "utf8" {
		strs, err := readStrings(tid, n, func(mem C.hid_t, buf unsafe.Pointer) C.herr_t {
			return C.H5Aread(aid, mem, buf)
		})
		if err != nil {
			return TypedValue{}, fmt.Errorf("H5Aread(%s): %w", name, err)
		}
		return TypedValue{Dtype: dtype, Scalar: rank == 0, Strs: strs}, nil
	}
	v, err := readBuffer(dtype, n, func(mem C.hid_t, buf unsafe.Pointer) C.herr_t {
		return C.H5Aread(aid, mem, buf)
	})
	if err != nil {
		return TypedValue{}, fmt.Errorf("H5Aread(%s): %w", name, err)
	}
	v.Scalar = rank == 0
	return v, nil
}

// readStrings reads n variable- or fixed-length strings from an attribute whose type id is tid.
func readStrings(tid C.hid_t, n int, read func(mem C.hid_t, buf unsafe.Pointer) C.herr_t) ([]string, error) {
	if n == 0 {
		return []string{}, nil
	}
	out := make([]string, n)
	if C.H5Tis_variable_str(tid) > 0 {
		ptrs := make([]*C.char, n)
		if read(tid, unsafe.Pointer(&ptrs[0])) < 0 {
			return nil, fmt.Errorf("read failed")
		}
		for i, p := range ptrs {
			if p != nil {
				out[i] = C.GoString(p)
				C.H5free_memory(unsafe.Pointer(p))
			}
		}
		return out, nil
	}
	sz := int(C.H5Tget_size(tid))
	buf := make([]byte, sz*n)
	if read(tid, unsafe.Pointer(&buf[0])) < 0 {
		return nil, fmt.Errorf("read failed")
	}
	for i := 0; i < n; i++ {
		chunk := buf[i*sz : (i+1)*sz]
		end := 0
		for end < len(chunk) && chunk[end] != 0 {
			end++
		}
		out[i] = string(chunk[:end])
	}
	return out, nil
}

func writeTypedAttr(id C.hid_t, name string, v TypedValue) error {
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))

	var space C.hid_t
	if v.Scalar {
		space = C.H5Screate(C.H5S_SCALAR)
	} else {
		dim := C.hsize_t(v.Len())
		space = C.H5Screate_simple(1, &dim, nil)
	}
	if space < 0 {
		return fmt.Errorf("create dataspace for %s failed", name)
	}
	defer C.H5Sclose(space)

	if v.Dtype == "utf8" {
		tid := C.H5Tcopy(C.H5T_C_S1)
		if tid < 0 {
			return fmt.Errorf("H5Tcopy failed")
		}
		defer C.H5Tclose(tid)
		if C.H5Tset_size(tid, C.H5T_VARIABLE) < 0 || C.H5Tset_cset(tid, C.H5T_CSET_UTF8) < 0 {
			return fmt.Errorf("configure string type failed")
		}
		aid := C.H5Acreate2(id, cname, tid, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
		if aid < 0 {
			return fmt.Errorf("H5Acreate2(%s) failed", name)
		}
		defer C.H5Aclose(aid)
		ptrs := make([]*C.char, len(v.Strs)+1)
		for i, s := range v.Strs {
			ptrs[i] = C.CString(s)
		}
		defer func() {
			for _, p := range ptrs {
				if p != nil {
					C.free(unsafe.Pointer(p))
				}
			}
		}()
		if C.H5Awrite(aid, tid, unsafe.Pointer(&ptrs[0])) < 0 {
			return fmt.Errorf("H5Awrite(%s) failed", name)
		}
		return nil
	}

	ft, err := fileTypeFor(v.Dtype)
	if err != nil {
		return err
	}
	aid := C.H5Acreate2(id, cname, ft, space, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if aid < 0 {
		return fmt.Errorf("H5Acreate2(%s) failed", name)
	}
	defer C.H5Aclose(aid)
	if err := writeBuffer(&v, func(mem C.hid_t, buf unsafe.Pointer) C.herr_t {
		return C.H5Awrite(aid, mem, buf)
	}); err != nil {
		return fmt.Errorf("H5Awrite(%s): %w", name, err)
	}
	return nil
}

// TypedAttrNames returns every attribute name on the group (no count cap).
func (g *Group) TypedAttrNames() []string { return allAttrNames(g.id) }

// ReadTypedAttr reads an attribute verbatim, preserving its stored dtype and scalar/1-D shape.
func (g *Group) ReadTypedAttr(name string) (TypedValue, error) { return readTypedAttr(g.id, name) }

// WriteTypedAttr creates an attribute with v's dtype; the name must not already exist.
func (g *Group) WriteTypedAttr(name string, v TypedValue) error {
	return writeTypedAttr(g.id, name, v)
}

// TypedAttrNames returns every attribute name on the dataset (no count cap).
func (d *Dataset) TypedAttrNames() []string { return allAttrNames(d.id) }

// ReadTypedAttr reads a dataset attribute verbatim.
func (d *Dataset) ReadTypedAttr(name string) (TypedValue, error) { return readTypedAttr(d.id, name) }

// WriteTypedAttr creates a dataset attribute with v's dtype.
func (d *Dataset) WriteTypedAttr(name string, v TypedValue) error {
	return writeTypedAttr(d.id, name, v)
}

// ---------------------------------------------------------------------------
// members and datasets
// ---------------------------------------------------------------------------

// Members lists every child link of the group with its object kind (no count cap).
func (g *Group) Members() ([]Member, error) {
	var info C.H5G_info_t
	if C.H5Gget_info(g.id, &info) < 0 {
		return nil, fmt.Errorf("H5Gget_info failed")
	}
	dot := C.CString(".")
	defer C.free(unsafe.Pointer(dot))
	out := make([]Member, 0, int(info.nlinks))
	for i := C.hsize_t(0); i < info.nlinks; i++ {
		sz := C.H5Lget_name_by_idx(g.id, dot, C.H5_INDEX_NAME, C.H5_ITER_INC, i, nil, 0, C.H5P_DEFAULT)
		if sz < 0 {
			return nil, fmt.Errorf("H5Lget_name_by_idx(%d) failed", int(i))
		}
		buf := make([]byte, int(sz)+1)
		C.H5Lget_name_by_idx(g.id, dot, C.H5_INDEX_NAME, C.H5_ITER_INC, i,
			(*C.char)(unsafe.Pointer(&buf[0])), C.size_t(len(buf)), C.H5P_DEFAULT)
		name := string(buf[:sz])

		cname := C.CString(name)
		oid := C.H5Oopen(g.id, cname, C.H5P_DEFAULT)
		C.free(unsafe.Pointer(cname))
		if oid < 0 {
			return nil, fmt.Errorf("cannot open %q", name)
		}
		kind := C.H5Iget_type(oid)
		C.H5Oclose(oid)
		out = append(out, Member{
			Name:      name,
			IsGroup:   kind == C.H5I_GROUP,
			IsDataset: kind == C.H5I_DATASET,
		})
	}
	return out, nil
}

// ReadTypedDataset reads a numeric dataset's dtype, dims, and flat row-major data.
func (g *Group) ReadTypedDataset(name string) (TypedValue, []uint64, error) {
	ds, err := g.OpenDataset(name)
	if err != nil {
		return TypedValue{}, nil, err
	}
	defer ds.Close()
	tid := C.H5Dget_type(ds.id)
	if tid < 0 {
		return TypedValue{}, nil, fmt.Errorf("H5Dget_type(%s) failed", name)
	}
	defer C.H5Tclose(tid)
	dtype := dtypeOf(tid)
	if dtype == "" {
		return TypedValue{}, nil, fmt.Errorf("unsupported HDF5 type")
	}
	if dtype == "utf8" {
		return TypedValue{}, nil, fmt.Errorf("string datasets are not supported")
	}
	space := C.H5Dget_space(ds.id)
	if space < 0 {
		return TypedValue{}, nil, fmt.Errorf("H5Dget_space(%s) failed", name)
	}
	defer C.H5Sclose(space)
	rank := int(C.H5Sget_simple_extent_ndims(space))
	dims := make([]uint64, rank)
	if rank > 0 {
		cd := make([]C.hsize_t, rank)
		C.H5Sget_simple_extent_dims(space, &cd[0], nil)
		for i, d := range cd {
			dims[i] = uint64(d)
		}
	}
	n := int(C.H5Sget_simple_extent_npoints(space))
	v, err := readBuffer(dtype, n, func(mem C.hid_t, buf unsafe.Pointer) C.herr_t {
		return C.H5Dread(ds.id, mem, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, buf)
	})
	if err != nil {
		return TypedValue{}, nil, fmt.Errorf("H5Dread(%s): %w", name, err)
	}
	return v, dims, nil
}

// CreateTypedDataset writes a numeric dataset and returns it open (caller closes) so attributes
// can be attached. An empty dims slice writes a scalar dataset.
func (g *Group) CreateTypedDataset(name string, dims []uint64, v TypedValue) (*Dataset, error) {
	ft, err := fileTypeFor(v.Dtype)
	if err != nil {
		return nil, err
	}
	var space C.hid_t
	if len(dims) == 0 {
		space = C.H5Screate(C.H5S_SCALAR)
	} else {
		cd := make([]C.hsize_t, len(dims))
		for i, d := range dims {
			cd[i] = C.hsize_t(d)
		}
		space = C.H5Screate_simple(C.int(len(dims)), &cd[0], nil)
	}
	if space < 0 {
		return nil, fmt.Errorf("create dataspace for %s failed", name)
	}
	defer C.H5Sclose(space)
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	id := C.H5Dcreate2(g.id, cname, ft, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dcreate2(%s) failed", name)
	}
	if v.Len() > 0 {
		if err := writeBuffer(&v, func(mem C.hid_t, buf unsafe.Pointer) C.herr_t {
			return C.H5Dwrite(id, mem, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, buf)
		}); err != nil {
			C.H5Dclose(id)
			return nil, fmt.Errorf("H5Dwrite(%s): %w", name, err)
		}
	}
	return &Dataset{id: id}, nil
}
