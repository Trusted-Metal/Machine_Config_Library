package h5c

/*
#include <hdf5.h>
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"encoding/binary"
	"fmt"
	"math"
	"unsafe"
)

// Generic 1-D compound datasets, used to carry extension tables verbatim. Members are limited to
// the contract's dtypes: fixed-length strings and fixed-width integers and floats.

// MemberError reports a compound member that cannot be carried, so callers can name its path.
type MemberError struct {
	Member  string
	Message string
}

func (e *MemberError) Error() string { return e.Member + ": " + e.Message }

// CompoundColumn is one member of a compound dataset together with its values; exactly one of
// Strs/Ints/Uints/Floats is populated, matching Dtype. Size is the byte length of a "utf8" member.
type CompoundColumn struct {
	Name   string
	Dtype  string
	Size   int
	Strs   []string
	Ints   []int64
	Uints  []uint64
	Floats []float64
}

func memberWidth(c *CompoundColumn) int {
	switch c.Dtype {
	case "utf8":
		return c.Size
	case "i8", "u8":
		return 1
	case "i16", "u16":
		return 2
	case "i32", "u32", "f32":
		return 4
	default:
		return 8
	}
}

// IsCompoundDataset reports whether the named dataset has a compound type.
func (g *Group) IsCompoundDataset(name string) bool {
	ds, err := g.OpenDataset(name)
	if err != nil {
		return false
	}
	defer ds.Close()
	tid := C.H5Dget_type(ds.id)
	if tid < 0 {
		return false
	}
	defer C.H5Tclose(tid)
	return C.H5Tget_class(tid) == C.H5T_COMPOUND
}

// ReadCompoundDataset reads a 1-D compound dataset: its members in declared order, each with its
// column of values, and the row count.
func (g *Group) ReadCompoundDataset(name string) ([]CompoundColumn, uint64, error) {
	ds, err := g.OpenDataset(name)
	if err != nil {
		return nil, 0, err
	}
	defer ds.Close()
	ftid := C.H5Dget_type(ds.id)
	if ftid < 0 {
		return nil, 0, fmt.Errorf("H5Dget_type(%s) failed", name)
	}
	defer C.H5Tclose(ftid)

	space := C.H5Dget_space(ds.id)
	if space < 0 {
		return nil, 0, fmt.Errorf("H5Dget_space(%s) failed", name)
	}
	defer C.H5Sclose(space)
	if rank := int(C.H5Sget_simple_extent_ndims(space)); rank != 1 {
		return nil, 0, fmt.Errorf("compound datasets must be 1-D, got rank %d", rank)
	}
	rows := int(C.H5Sget_simple_extent_npoints(space))

	ntid := C.H5Tget_native_type(ftid, C.H5T_DIR_ASCEND)
	if ntid < 0 {
		return nil, 0, fmt.Errorf("H5Tget_native_type(%s) failed", name)
	}
	defer C.H5Tclose(ntid)
	rowSize := int(C.H5Tget_size(ntid))

	n := int(C.H5Tget_nmembers(ftid))
	cols := make([]CompoundColumn, n)
	offsets := make([]int, n)
	for i := 0; i < n; i++ {
		cname := C.H5Tget_member_name(ftid, C.uint(i))
		member := C.GoString(cname)
		C.H5free_memory(unsafe.Pointer(cname))
		mt := C.H5Tget_member_type(ftid, C.uint(i))
		if mt < 0 {
			return nil, 0, &MemberError{member, "cannot read the member type"}
		}
		class := C.H5Tget_class(mt)
		col := CompoundColumn{Name: member}
		switch class {
		case C.H5T_STRING:
			if C.H5Tis_variable_str(mt) > 0 {
				C.H5Tclose(mt)
				return nil, 0, &MemberError{member, "variable-length strings are not supported in a compound type"}
			}
			col.Dtype, col.Size = "utf8", int(C.H5Tget_size(mt))
		case C.H5T_INTEGER, C.H5T_FLOAT:
			if C.H5Tget_order(mt) == C.H5T_ORDER_BE {
				C.H5Tclose(mt)
				return nil, 0, &MemberError{member, "big-endian compound members are not supported"}
			}
			col.Dtype = dtypeOf(mt)
		}
		C.H5Tclose(mt)
		if col.Dtype == "" {
			return nil, 0, &MemberError{member, "unsupported compound member type (nested compound, array, enum and bool members are not supported)"}
		}
		cols[i] = col
		offsets[i] = int(C.H5Tget_member_offset(ntid, C.uint(i)))
	}

	buf := make([]byte, rows*rowSize)
	if rows > 0 {
		if C.H5Dread(ds.id, ntid, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&buf[0])) < 0 {
			return nil, 0, fmt.Errorf("H5Dread(%s) failed", name)
		}
	}
	for i := range cols {
		decodeColumn(&cols[i], buf, rows, rowSize, offsets[i])
	}
	return cols, uint64(rows), nil
}

func decodeColumn(c *CompoundColumn, buf []byte, rows, rowSize, offset int) {
	w := memberWidth(c)
	order := binary.NativeEndian
	for r := 0; r < rows; r++ {
		b := buf[r*rowSize+offset : r*rowSize+offset+w]
		switch c.Dtype {
		case "utf8":
			end := 0
			for end < len(b) && b[end] != 0 {
				end++
			}
			c.Strs = append(c.Strs, string(b[:end]))
		case "i8":
			c.Ints = append(c.Ints, int64(int8(b[0])))
		case "i16":
			c.Ints = append(c.Ints, int64(int16(order.Uint16(b))))
		case "i32":
			c.Ints = append(c.Ints, int64(int32(order.Uint32(b))))
		case "i64":
			c.Ints = append(c.Ints, int64(order.Uint64(b)))
		case "u8":
			c.Uints = append(c.Uints, uint64(b[0]))
		case "u16":
			c.Uints = append(c.Uints, uint64(order.Uint16(b)))
		case "u32":
			c.Uints = append(c.Uints, uint64(order.Uint32(b)))
		case "u64":
			c.Uints = append(c.Uints, order.Uint64(b))
		case "f32":
			c.Floats = append(c.Floats, float64(math.Float32frombits(order.Uint32(b))))
		case "f64":
			c.Floats = append(c.Floats, math.Float64frombits(order.Uint64(b)))
		}
	}
}

// CreateCompoundDataset writes a 1-D compound dataset and returns it open (caller closes) so
// attributes can be attached. Members are packed in order; strings are fixed-length UTF-8.
func (g *Group) CreateCompoundDataset(name string, rows uint64, cols []CompoundColumn) (*Dataset, error) {
	rowSize := 0
	for i := range cols {
		rowSize += memberWidth(&cols[i])
	}
	tid := C.H5Tcreate(C.H5T_COMPOUND, C.size_t(rowSize))
	if tid < 0 {
		return nil, fmt.Errorf("H5Tcreate failed")
	}
	defer C.H5Tclose(tid)

	offset := 0
	for i := range cols {
		c := &cols[i]
		var mt C.hid_t
		owned := false
		if c.Dtype == "utf8" {
			mt = C.H5Tcopy(C.H5T_C_S1)
			if mt < 0 || C.H5Tset_size(mt, C.size_t(c.Size)) < 0 ||
				C.H5Tset_cset(mt, C.H5T_CSET_UTF8) < 0 || C.H5Tset_strpad(mt, C.H5T_STR_NULLPAD) < 0 {
				return nil, fmt.Errorf("configure string type for %s failed", c.Name)
			}
			owned = true
		} else {
			var err error
			if mt, err = fileTypeFor(c.Dtype); err != nil {
				return nil, err
			}
		}
		cname := C.CString(c.Name)
		status := C.H5Tinsert(tid, cname, C.size_t(offset), mt)
		C.free(unsafe.Pointer(cname))
		if owned {
			C.H5Tclose(mt)
		}
		if status < 0 {
			return nil, fmt.Errorf("H5Tinsert(%s) failed", c.Name)
		}
		offset += memberWidth(c)
	}

	dim := C.hsize_t(rows)
	space := C.H5Screate_simple(1, &dim, nil)
	if space < 0 {
		return nil, fmt.Errorf("create dataspace for %s failed", name)
	}
	defer C.H5Sclose(space)
	cname := C.CString(name)
	defer C.free(unsafe.Pointer(cname))
	id := C.H5Dcreate2(g.id, cname, tid, space, C.H5P_DEFAULT, C.H5P_DEFAULT, C.H5P_DEFAULT)
	if id < 0 {
		return nil, fmt.Errorf("H5Dcreate2(%s) failed", name)
	}

	if rows > 0 {
		buf := make([]byte, int(rows)*rowSize)
		offset = 0
		for i := range cols {
			encodeColumn(&cols[i], buf, int(rows), rowSize, offset)
			offset += memberWidth(&cols[i])
		}
		if C.H5Dwrite(id, tid, C.H5S_ALL, C.H5S_ALL, C.H5P_DEFAULT, unsafe.Pointer(&buf[0])) < 0 {
			C.H5Dclose(id)
			return nil, fmt.Errorf("H5Dwrite(%s) failed", name)
		}
	}
	return &Dataset{id: id}, nil
}

func encodeColumn(c *CompoundColumn, buf []byte, rows, rowSize, offset int) {
	w := memberWidth(c)
	order := binary.LittleEndian
	for r := 0; r < rows; r++ {
		b := buf[r*rowSize+offset : r*rowSize+offset+w]
		switch c.Dtype {
		case "utf8":
			copy(b, c.Strs[r])
		case "i8":
			b[0] = byte(int8(c.Ints[r]))
		case "i16":
			order.PutUint16(b, uint16(int16(c.Ints[r])))
		case "i32":
			order.PutUint32(b, uint32(int32(c.Ints[r])))
		case "i64":
			order.PutUint64(b, uint64(c.Ints[r]))
		case "u8":
			b[0] = byte(c.Uints[r])
		case "u16":
			order.PutUint16(b, uint16(c.Uints[r]))
		case "u32":
			order.PutUint32(b, uint32(c.Uints[r]))
		case "u64":
			order.PutUint64(b, c.Uints[r])
		case "f32":
			order.PutUint32(b, math.Float32bits(float32(c.Floats[r])))
		case "f64":
			order.PutUint64(b, math.Float64bits(c.Floats[r]))
		}
	}
}
