module machine-config-go

// Library uses internal/h5c (CGo → libhdf5). Historical scigolib protos are
// //go:build ignore — dense attribute storage on h5py fixtures is unreadable
// (see proto5_dense_attr_audit).
go 1.22
