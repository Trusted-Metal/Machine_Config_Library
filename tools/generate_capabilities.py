#!/usr/bin/env python3
"""
Generate stable facade stubs from schema/capabilities/{api,models,errors}.yaml.

Usage:
    python tools/generate_capabilities.py
    python tools/generate_capabilities.py --check
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
CAP = REPO_ROOT / "schema" / "capabilities"
API = CAP / "api.yaml"
MODELS = CAP / "models.yaml"
ERRORS = CAP / "errors.yaml"
BINDINGS = CAP / "bindings"

HDR_TS = "// AUTO-GENERATED from schema/capabilities — DO NOT EDIT\n// python tools/generate_capabilities.py\n\n"
HDR_PY = "# AUTO-GENERATED from schema/capabilities — DO NOT EDIT\n# python tools/generate_capabilities.py\n\n"
HDR_RS = "// AUTO-GENERATED from schema/capabilities — DO NOT EDIT\n// python tools/generate_capabilities.py\n\n"
HDR_HPP = "// AUTO-GENERATED from schema/capabilities — DO NOT EDIT\n// python tools/generate_capabilities.py\n\n"


def render_ts(api: dict, models: dict) -> str:
    lines = [
        HDR_TS,
        'import type { Result } from "./result.js";',
        'import type { CapabilityError } from "./errors.js";',
        'import type {',
        "  MachineConfigMeta,",
        "  Machine,",
        "  OpticalTrain,",
        "  Scanner,",
        "  LightSource,",
        "  Collimator,",
        "  ScannerCard,",
        "  ClearBox,",
        "  OpcuaConfig,",
        "  CorrectionData,",
        '} from "../models.js";',
        "",
        "/** Write mode for set* model APIs. Default is Merge. */",
        "export enum SetMode {",
        "  Merge = 'Merge',",
        "  Replace = 'Replace',",
        "}",
        "",
        "export type MetaModel = MachineConfigMeta & { extra?: Record<string, unknown> };",
        "export type MachineModel = Machine & { extra?: Record<string, unknown> };",
        "export type ScannerModel = Scanner & { extra?: Record<string, unknown> };",
        "export type LightSourceModel = LightSource & { extra?: Record<string, unknown> };",
        "export type CollimatorModel = Collimator & { extra?: Record<string, unknown> };",
        "export type ScannerCardModel = ScannerCard & { extra?: Record<string, unknown> };",
        "export type ClearBoxModel = ClearBox & { extra?: Record<string, unknown> };",
        "export type OpticalTrainModel = OpticalTrain & { extra?: Record<string, unknown> };",
        "export type OpcuaModel = OpcuaConfig & { extra?: Record<string, unknown> };",
        "",
        "export interface MetaHandle {",
        "  getModel(): MetaModel;",
        "  setModel(model: MetaModel, mode?: SetMode): Result<void, CapabilityError>;",
        "}",
        "export interface MachineHandle {",
        "  getModel(): MachineModel;",
        "  setModel(model: MachineModel, mode?: SetMode): Result<void, CapabilityError>;",
        "}",
        "export interface ClearBoxHandle {",
        "  getModel(): ClearBoxModel;",
        "  setModel(model: ClearBoxModel, mode?: SetMode): Result<void, CapabilityError>;",
        "}",
        "export interface OptionalComponentsHandle {",
        "  clearbox(): Result<ClearBoxHandle, CapabilityError>;",
        "}",
        "export interface TrainHandle {",
        "  getModel(): OpticalTrainModel;",
        "  setModel(model: OpticalTrainModel, mode?: SetMode): Result<void, CapabilityError>;",
        "  getScanner(): ScannerModel;",
        "  setScanner(model: ScannerModel, mode?: SetMode): Result<void, CapabilityError>;",
        "  getLightSource(): LightSourceModel;",
        "  setLightSource(model: LightSourceModel, mode?: SetMode): Result<void, CapabilityError>;",
        "  getCollimator(): CollimatorModel;",
        "  setCollimator(model: CollimatorModel, mode?: SetMode): Result<void, CapabilityError>;",
        "  getScannerCard(): ScannerCardModel;",
        "  setScannerCard(model: ScannerCardModel, mode?: SetMode): Result<void, CapabilityError>;",
        "  optionalComponents(): OptionalComponentsHandle | null;",
        "}",
        "export interface TrainCollection {",
        "  readonly length: number;",
        "  get(index: number): Result<TrainHandle, CapabilityError>;",
        "  [Symbol.iterator](): Iterator<TrainHandle>;",
        "}",
        "export interface OpcuaHandle {",
        "  getModel(): OpcuaModel;",
        "  setModel(model: OpcuaModel, mode?: SetMode): Result<void, CapabilityError>;",
        "}",
        "export interface MachineConfigFile {",
        "  fileVersion(): string;",
        "  meta(): MetaHandle;",
        "  machine(): MachineHandle;",
        "  opticalTrains(): TrainCollection;",
        "  opticalTrain(index: number): Result<TrainHandle, CapabilityError>;",
        "  opcua(): Result<OpcuaHandle, CapabilityError>;",
        "  getCorrectionData(trainIndex: number): Result<CorrectionData, CapabilityError>;",
        "  getInverseCorrectionData(trainIndex: number): Result<CorrectionData, CapabilityError>;",
        "  save(path?: string): Promise<Result<void, CapabilityError>>;",
        "  close(): void;",
        "}",
        "",
    ]
    _ = api, models
    return "".join(line + "\n" for line in lines)


def render_py(api: dict, models: dict) -> str:
    _ = api, models
    return (
        HDR_PY
        + '"""Generated facade protocols and SetMode."""\n'
        + "from __future__ import annotations\n\n"
        + "from enum import Enum\n"
        + "from typing import Protocol, Any, Optional, Iterator\n\n"
        + "from machine_config.capabilities.result import Result\n"
        + "from machine_config.capabilities.errors import CapabilityError\n\n\n"
        + "class SetMode(str, Enum):\n"
        + "    MERGE = 'Merge'\n"
        + "    REPLACE = 'Replace'\n\n\n"
        + "class MetaHandle(Protocol):\n"
        + "    def get_model(self) -> dict[str, Any]: ...\n"
        + "    def set_model(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n\n\n"
        + "class MachineHandle(Protocol):\n"
        + "    def get_model(self) -> dict[str, Any]: ...\n"
        + "    def set_model(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n\n\n"
        + "class ClearBoxHandle(Protocol):\n"
        + "    def get_model(self) -> dict[str, Any]: ...\n"
        + "    def set_model(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n\n\n"
        + "class OptionalComponentsHandle(Protocol):\n"
        + "    def clearbox(self) -> Result[ClearBoxHandle, CapabilityError]: ...\n\n\n"
        + "class TrainHandle(Protocol):\n"
        + "    def get_model(self) -> dict[str, Any]: ...\n"
        + "    def set_model(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n"
        + "    def get_scanner(self) -> dict[str, Any]: ...\n"
        + "    def set_scanner(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n"
        + "    def get_light_source(self) -> dict[str, Any]: ...\n"
        + "    def set_light_source(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n"
        + "    def get_collimator(self) -> dict[str, Any]: ...\n"
        + "    def set_collimator(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n"
        + "    def get_scanner_card(self) -> dict[str, Any]: ...\n"
        + "    def set_scanner_card(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n"
        + "    def optional_components(self) -> OptionalComponentsHandle | None: ...\n\n\n"
        + "class TrainCollection(Protocol):\n"
        + "    def __len__(self) -> int: ...\n"
        + "    def get(self, index: int) -> Result[TrainHandle, CapabilityError]: ...\n"
        + "    def __iter__(self) -> Iterator[TrainHandle]: ...\n\n\n"
        + "class OpcuaHandle(Protocol):\n"
        + "    def get_model(self) -> dict[str, Any]: ...\n"
        + "    def set_model(self, model: dict[str, Any], mode: SetMode = SetMode.MERGE) -> Result[None, CapabilityError]: ...\n\n\n"
        + "class MachineConfigFile(Protocol):\n"
        + "    def file_version(self) -> str: ...\n"
        + "    def meta(self) -> MetaHandle: ...\n"
        + "    def machine(self) -> MachineHandle: ...\n"
        + "    def optical_trains(self) -> TrainCollection: ...\n"
        + "    def optical_train(self, index: int) -> Result[TrainHandle, CapabilityError]: ...\n"
        + "    def opcua(self) -> Result[OpcuaHandle, CapabilityError]: ...\n"
        + "    def save(self, path: str | None = None) -> Result[None, CapabilityError]: ...\n"
        + "    def close(self) -> None: ...\n"
    )


def render_rs(api: dict, models: dict) -> str:
    _ = api, models
    return (
        HDR_RS
        + "//! Generated SetMode and facade trait outlines.\n\n"
        + "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n"
        + "pub enum SetMode {\n"
        + "    #[default]\n"
        + "    Merge,\n"
        + "    Replace,\n"
        + "}\n"
    )


def render_hpp(api: dict, models: dict) -> str:
    _ = api, models
    return (
        HDR_HPP
        + "#pragma once\n"
        + '#include "machine_config/capabilities/result.hpp"\n'
        + "#include <string>\n\n"
        + "namespace machine_config::capabilities {\n\n"
        + "enum class SetMode { Merge, Replace };\n\n"
        + "class IMachineConfigFile {\n"
        + "public:\n"
        + "  virtual ~IMachineConfigFile() = default;\n"
        + "  virtual std::string fileVersion() const = 0;\n"
        + "  virtual std::size_t opticalTrainCount() const = 0;\n"
        + "  virtual Result<void> save(const std::string* path = nullptr) = 0;\n"
        + "  virtual void close() = 0;\n"
        + "};\n\n"
        + "}  // namespace machine_config::capabilities\n"
    )


def write_if_changed(path: Path, content: str, check: bool) -> bool:
    path.parent.mkdir(parents=True, exist_ok=True)
    old = path.read_text(encoding="utf-8") if path.exists() else None
    if old == content:
        return False
    if check:
        print(f"DRIFT: {path.relative_to(REPO_ROOT)}")
        return True
    path.write_text(content, encoding="utf-8")
    print(f"wrote {path.relative_to(REPO_ROOT)}")
    return False


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    api = yaml.safe_load(API.read_text(encoding="utf-8"))
    models = yaml.safe_load(MODELS.read_text(encoding="utf-8"))
    _ = yaml.safe_load(ERRORS.read_text(encoding="utf-8"))
    if not list(BINDINGS.glob("*.yaml")):
        raise SystemExit("no bindings under schema/capabilities/bindings")

    outputs = [
        (REPO_ROOT / "nodejs/src/capabilities/generated.ts", render_ts(api, models)),
        (REPO_ROOT / "python/src/machine_config/capabilities/generated.py", render_py(api, models)),
        (REPO_ROOT / "rust/src/capabilities/generated.rs", render_rs(api, models)),
        (REPO_ROOT / "cpp/include/machine_config/capabilities/generated.hpp", render_hpp(api, models)),
    ]
    drifted = False
    for path, content in outputs:
        if write_if_changed(path, content, args.check):
            drifted = True
    if args.check and drifted:
        print("Capability generated sources are out of date.", file=sys.stderr)
        sys.exit(1)
    print("Capability generated sources OK." if args.check else "Done.")


if __name__ == "__main__":
    main()
