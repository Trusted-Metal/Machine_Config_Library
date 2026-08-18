#pragma once
// Umbrella public header — the single include a consumer needs for the
// default surface (models, reader, writer, builder, stable capability
// facade). See VALIDATION_PLAN.md §8 S-09 / §10.
//
// Deliberately excludes `schema.hpp`: it has a hard compile-time
// `#error "SCHEMA_DIR must be defined by CMakeLists.txt"` unless the
// consumer's build defines SCHEMA_DIR pointing at
// schema/machine_config_v1.schema.json. Bundling it here would force every
// consumer of this header — including the static tarball — to locate and
// wire up the schema file even if they never validate anything. Schema
// validation is opt-in in every other language too (Node's `validate()`
// and Python's schema helpers are separate imports, not part of the default
// surface) — consumers who want it include
// `<machine_config/schema.hpp>` themselves and define SCHEMA_DIR.

#include "machine_config/models.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"
#include "machine_config/builder.hpp"
#include "machine_config/capabilities.hpp"
#include "machine_config/version.hpp"
