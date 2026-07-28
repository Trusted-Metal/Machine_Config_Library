"""Phase 1.4/1.5 — Command-line interface for the machine configuration library."""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import click
import jsonschema
import yaml

from .builder import MockConfigBuilder, YamlConfigBuilder
from .reader import MachineConfigReader, config_from_dict
from .schema import SCHEMA
from .writer import MachineConfigWriter


@click.group()
@click.version_option(version="0.1.0", prog_name="machine-config")
def main() -> None:
    """LPBF machine configuration CLI (machine-agnostic).

    Inspect, validate, and convert .h5 machine config files.
    """


# ---------------------------------------------------------------------------
# inspect
# ---------------------------------------------------------------------------

@main.command()
@click.argument("path", type=click.Path(exists=True, dir_okay=False))
@click.option(
    "--verbose", "-v",
    is_flag=True,
    default=False,
    help="Dump the complete configuration as YAML (every field).",
)
def inspect(path: str, verbose: bool) -> None:
    """Print a human-readable summary of a machine config HDF5 file.

    Without --verbose: one header block plus one summary line per optical train.

    With --verbose: the same summary followed by a full YAML dump of every
    field in the parsed config — nothing omitted, nothing curated.
    """
    try:
        reader = MachineConfigReader(path)
        config = reader.parse()
    except Exception as exc:
        click.echo(f"Error reading {path}: {exc}", err=True)
        sys.exit(1)

    m = config.machine
    bp = m.build_plate
    meta = config.meta

    click.echo(f"Machine:      {meta.machine_name}")
    click.echo(f"Manufacturer: {m.manufacturer}")
    click.echo(f"Model:        {m.model}")
    click.echo(f"Serial:       {m.serial_number}")

    bp_dims = " x ".join(str(v) for v in [bp.x, bp.y, bp.z] if v is not None)
    click.echo(f"Build Plate:  {bp_dims} {bp.x_unit or 'mm'}")
    click.echo(f"File Version: {meta.file_version}")
    click.echo(f"Export Date:  {meta.export_date}")
    click.echo(f"\nOptical Trains: {len(config.optical_trains)}")

    for train in config.optical_trains:
        s = train.scanner
        ls = train.light_source
        parts: list[str] = [f"  {train.train_id}"]
        if s.working_distance is not None:
            parts.append(f"WD: {s.working_distance} {s.working_distance_unit or 'mm'}")
        if s.scan_head_offset_x is not None:
            parts.append(f"Offset X: {s.scan_head_offset_x} mm")
        if ls.wavelength is not None or ls.power_max_nominal is not None:
            wl = f"{ls.wavelength} {ls.wavelength_unit or 'nm'}" if ls.wavelength else ""
            pw = (
                f"{ls.power_max_nominal} {ls.power_max_nominal_unit or 'W'} max"
                if ls.power_max_nominal
                else ""
            )
            laser_str = "  ".join(x for x in [wl, pw] if x)
            if laser_str:
                parts.append(f"Laser: {laser_str}")
        click.echo("  ".join(parts))

    if verbose:
        config_dict = reader._config_to_dict(config)
        click.echo("\n--- Full Configuration ---")
        click.echo(
            yaml.dump(
                config_dict,
                allow_unicode=True,
                sort_keys=False,
                default_flow_style=False,
            ).rstrip()
        )


# ---------------------------------------------------------------------------
# validate
# ---------------------------------------------------------------------------

@main.command()
@click.argument("path", type=click.Path(exists=True, dir_okay=False))
def validate(path: str) -> None:
    """Validate a machine config HDF5 file against the canonical JSON Schema."""
    click.echo(f"Validating: {path}")

    try:
        click.echo("  Parsing HDF5 ...", nl=False)
        reader = MachineConfigReader(path)
        config = reader.parse()
        click.echo(" OK")
    except Exception as exc:
        click.echo(f" FAIL\n  {exc}", err=True)
        sys.exit(1)

    try:
        click.echo("  Schema validation ...", nl=False)
        output_dict = reader._config_to_dict(config)
        jsonschema.validate(output_dict, SCHEMA)
        click.echo(" PASS")
    except jsonschema.ValidationError as exc:
        click.echo(f" FAIL\n  {exc.message}", err=True)
        sys.exit(1)

    click.echo(
        f"\n  {config.meta.machine_name}"
        f"  |  {len(config.optical_trains)} optical train(s)"
    )


# ---------------------------------------------------------------------------
# export-json
# ---------------------------------------------------------------------------

@main.command(name="export-json")
@click.argument("path", type=click.Path(exists=True, dir_okay=False))
@click.option(
    "--output", "-o",
    type=click.Path(),
    default=None,
    help="Write to this file instead of stdout.",
)
@click.option(
    "--indent",
    default=2,
    show_default=True,
    help="JSON indentation level.",
)
@click.option(
    "--include-binary",
    is_flag=True,
    default=False,
    help=(
        "Include correction arrays and raw .fc3 bytes in the output. "
        "Produces large output (~14 MB for a 2-laser config). "
        "Default: metadata and scalar fields only."
    ),
)
def export_json(path: str, output: str | None, indent: int, include_binary: bool) -> None:
    """Export a machine config HDF5 file to canonical JSON."""
    try:
        json_str = MachineConfigReader(path).to_json(indent=indent, include_binary=include_binary)
    except Exception as exc:
        click.echo(f"Error: {exc}", err=True)
        sys.exit(1)

    if output:
        Path(output).write_text(json_str, encoding="utf-8")
        click.echo(f"Written to {output}")
    else:
        click.echo(json_str)


# ---------------------------------------------------------------------------
# write
# ---------------------------------------------------------------------------

@main.command()
@click.argument("json_path", metavar="JSON_FILE", type=click.Path(exists=True))
@click.option("--output", "-o", required=True, type=click.Path(), help="Output HDF5 file.")
def write(json_path: str, output: str) -> None:
    """Write a canonical JSON config back to HDF5 format."""
    try:
        data = json.loads(Path(json_path).read_text(encoding="utf-8"))
        config = config_from_dict(data)
        MachineConfigWriter(config).write(output)
        click.echo(f"Written to {output}")
    except Exception as exc:  # noqa: BLE001
        click.echo(f"Error: {exc}", err=True)
        sys.exit(1)


# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------

@main.command()
@click.option(
    "--from-yaml", "yaml_path",
    type=click.Path(exists=True),
    default=None,
    help="Build from a YAML spec file.",
)
@click.option("--mock", is_flag=True, default=False, help="Generate a synthetic config.")
@click.option("--lasers", default=2, show_default=True, help="Number of lasers (--mock only).")
@click.option("--output", "-o", required=True, type=click.Path(), help="Output HDF5 file.")
def build(yaml_path: str | None, mock: bool, lasers: int, output: str) -> None:
    """Build an HDF5 config from a YAML spec or as a synthetic mock."""
    if yaml_path:
        try:
            YamlConfigBuilder(yaml_path).save(output)
            click.echo(f"Written to {output}")
        except Exception as exc:  # noqa: BLE001
            click.echo(f"Error: {exc}", err=True)
            sys.exit(1)
    elif mock:
        try:
            MockConfigBuilder(n_lasers=lasers).save(output)
            click.echo(f"Written to {output}")
        except Exception as exc:  # noqa: BLE001
            click.echo(f"Error: {exc}", err=True)
            sys.exit(1)
    else:
        click.echo("Error: specify --from-yaml or --mock.", err=True)
        sys.exit(2)


# ---------------------------------------------------------------------------
# correction-hash
# ---------------------------------------------------------------------------

@main.command(name="correction-hash")
@click.argument("path", type=click.Path(exists=True))
@click.option("--train", default=0, show_default=True, help="0-indexed optical train number.")
@click.option("--inverse", is_flag=True, default=False, help="Hash the inverse correction grid.")
def correction_hash(path: str, train: int, inverse: bool) -> None:
    """Print the SHA-256 hash of a ClearBox correction grid.

    Values are hashed as flat little-endian float64 bytes, making the digest
    directly comparable with the Rust ``correction-hash`` command.
    """
    import hashlib
    import numpy as np  # noqa: PLC0415
    try:
        reader = MachineConfigReader(path)
        arr: np.ndarray = (
            reader.get_inverse_correction_data(train)
            if inverse
            else reader.get_correction_data(train)
        )
        # Explicit little-endian float64 for cross-platform determinism.
        digest = hashlib.sha256(arr.astype("<f8").tobytes()).hexdigest()
        click.echo(digest)
    except Exception as exc:  # noqa: BLE001
        click.echo(f"Error: {exc}", err=True)
        sys.exit(1)


# ---------------------------------------------------------------------------
# demo
# ---------------------------------------------------------------------------

@main.command()
def demo() -> None:
    """Generate a synthetic 2-laser config and print its summary."""
    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as tmp:
        tmp_path = Path(tmp.name)
    try:
        MockConfigBuilder(n_lasers=2).save(tmp_path)
        reader = MachineConfigReader(tmp_path)
        config = reader.parse()
        click.echo("=== DEMO: Synthetic 2-laser config ===")
        click.echo(reader.to_json(indent=2))
    finally:
        tmp_path.unlink(missing_ok=True)
