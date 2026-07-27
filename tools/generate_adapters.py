#!/usr/bin/env python3
"""
For every spec in schema/adapters/, render the four language adapter files
using Jinja2 templates. Idempotent — re-running overwrites existing generated files.

Usage (from any directory):
    python tools/generate_adapters.py
"""
from pathlib import Path
import yaml
from jinja2 import Environment, FileSystemLoader

REPO_ROOT    = Path(__file__).resolve().parent.parent
SPEC_DIR     = REPO_ROOT / "schema" / "adapters"
TEMPLATE_DIR = REPO_ROOT / "tools" / "templates"

OUTPUTS = {
    "python":     ("adapter_python.py.j2",
                   "python/src/machine_config/adapters/{name}.py"),
    "typescript": ("adapter_typescript.ts.j2",
                   "nodejs/src/adapters/{name}.ts"),
    "rust":       ("adapter_rust.rs.j2",
                   "rust/src/adapters/{name}.rs"),
    "cpp":        ("adapter_cpp.hpp.j2",
                   "cpp/include/machine_config/adapters/{name}.hpp"),
}


def main() -> None:
    env = Environment(
        loader=FileSystemLoader(str(TEMPLATE_DIR)),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
    )

    specs = sorted(SPEC_DIR.glob("*.yaml"))
    if not specs:
        print("No adapter specs found in schema/adapters/ — nothing to generate.")
        return

    for spec_path in specs:
        spec = yaml.safe_load(spec_path.read_text(encoding="utf-8"))
        spec.setdefault("changes", [])
        name = spec_path.stem  # e.g. "v1_0_to_v1_1"

        for lang, (tmpl_name, out_pattern) in OUTPUTS.items():
            rendered = env.get_template(tmpl_name).render(spec=spec, name=name)
            out = REPO_ROOT / out_pattern.format(name=name)
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(rendered, encoding="utf-8")
            print(f"[{lang}] wrote {out.relative_to(REPO_ROOT)}")

    print("Done. Review generated files before committing.")


if __name__ == "__main__":
    main()
