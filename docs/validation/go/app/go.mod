module mcl_go_validation

go 1.22

require machine-config-go v0.0.0

// Deliberately its own standalone module, not a member of the repo's go/
// module — this app behaves like a real external consumer with its own
// independent build, mirroring how the ported Rust/Node.js apps are their
// own separate projects rather than being built as part of go/'s own module.
replace machine-config-go => ../../../../go
