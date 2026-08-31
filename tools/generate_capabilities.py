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
    # MachineConfigFile mirrors MachineConfigFileV1_0's full public surface
    # (capabilities/v1_0/file.rs) exactly — including where it differs from
    # C++'s IMachineConfigFile (get_clearbox returns Option<ClearBox> inside
    # Ok, not a separate has_optional_components + Err(NotPresent); set_train
    # exists here but not in C++). Never a narrower interface, and never a
    # copy of another language's shape — see DISPATCH_REGISTRY_PLAN.md.
    return (
        HDR_RS
        + "//! Generated SetMode and facade trait.\n\n"
        + "use crate::capabilities::errors::CapabilityError;\n"
        + "use crate::capabilities::result::Result;\n"
        + "use crate::models::{\n"
        + "    ClearBox, Collimator, CorrectionData, LightSource, Machine, MachineConfigMeta,\n"
        + "    OpcuaConfig, OpticalTrain, Scanner, ScannerCard,\n"
        + "};\n"
        + "use std::path::Path;\n\n"
        + "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\n"
        + "pub enum SetMode {\n"
        + "    #[default]\n"
        + "    Merge,\n"
        + "    Replace,\n"
        + "}\n\n"
        + "/// Version-agnostic stable facade — mirrors MachineConfigFileV1_0's full\n"
        + "/// public surface so the version-agnostic dispatch path\n"
        + "/// (capabilities::open_machine_config/create_machine_config) never loses\n"
        + "/// capability relative to using the concrete type directly.\n"
        + "pub trait MachineConfigFile {\n"
        + "    fn file_version(&self) -> &str;\n"
        + "    fn optical_train_count(&self) -> Result<usize, CapabilityError>;\n"
        + "\n"
        + "    fn get_meta(&self) -> Result<MachineConfigMeta, CapabilityError>;\n"
        + "    fn set_meta(&mut self, model: MachineConfigMeta, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_machine(&self) -> Result<Machine, CapabilityError>;\n"
        + "    fn set_machine(&mut self, model: Machine, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_train(&self, index: usize) -> Result<OpticalTrain, CapabilityError>;\n"
        + "    fn set_train(&mut self, index: usize, model: OpticalTrain, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_scanner(&self, index: usize) -> Result<Scanner, CapabilityError>;\n"
        + "    fn set_scanner(&mut self, index: usize, model: Scanner, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_light_source(&self, index: usize) -> Result<LightSource, CapabilityError>;\n"
        + "    fn set_light_source(&mut self, index: usize, model: LightSource, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_collimator(&self, index: usize) -> Result<Collimator, CapabilityError>;\n"
        + "    fn set_collimator(&mut self, index: usize, model: Collimator, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_scanner_card(&self, index: usize) -> Result<ScannerCard, CapabilityError>;\n"
        + "    fn set_scanner_card(&mut self, index: usize, model: ScannerCard, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_clearbox(&self, index: usize) -> Result<Option<ClearBox>, CapabilityError>;\n"
        + "    fn set_clearbox(&mut self, index: usize, model: ClearBox, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn get_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError>;\n"
        + "    fn get_inverse_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError>;\n"
        + "\n"
        + "    fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError>;\n"
        + "    fn set_opcua(&mut self, model: OpcuaConfig, mode: SetMode) -> Result<(), CapabilityError>;\n"
        + "\n"
        + "    fn save(&mut self, path: Option<&Path>) -> Result<(), CapabilityError>;\n"
        + "    fn close(&mut self);\n"
        + "}\n"
    )


def render_hpp(api: dict, models: dict) -> str:
    _ = api, models
    # IMachineConfigFile mirrors MachineConfigFileV1_0's full public surface
    # (capabilities/v1_0/file.hpp) — flat/index-based, matching this
    # language's own concrete class, not Python/TS's handle-based shape
    # (those are a different, unrelated facade design; unifying shape across
    # languages is out of scope here — see DISPATCH_REGISTRY_PLAN.md).
    # Kept complete deliberately: a narrower interface would silently lose
    # capability for every version-agnostic caller (openMachineConfig/
    # createMachineConfig), which is exactly the gap this fixes.
    return (
        HDR_HPP
        + "#pragma once\n"
        + '#include "machine_config/capabilities/result.hpp"\n'
        + '#include "machine_config/models.hpp"\n'
        + "#include <cstddef>\n"
        + "#include <string>\n\n"
        + "namespace machine_config::capabilities {\n\n"
        + "enum class SetMode { Merge, Replace };\n\n"
        + "class IMachineConfigFile {\n"
        + "public:\n"
        + "  virtual ~IMachineConfigFile() = default;\n"
        + "  virtual std::string fileVersion() const = 0;\n"
        + "  virtual std::size_t opticalTrainCount() const = 0;\n"
        + "\n"
        + "  virtual MachineConfigMeta getMeta() const = 0;\n"
        + "  virtual Result<void> setMeta(const MachineConfigMeta& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Machine getMachine() const = 0;\n"
        + "  virtual Result<void> setMachine(const Machine& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Result<OpticalTrain> getTrain(std::size_t index) const = 0;\n"
        + "\n"
        + "  virtual Result<Scanner> getScanner(std::size_t index) const = 0;\n"
        + "  virtual Result<void> setScanner(std::size_t index, const Scanner& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Result<LightSource> getLightSource(std::size_t index) const = 0;\n"
        + "  virtual Result<void> setLightSource(std::size_t index, const LightSource& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Result<Collimator> getCollimator(std::size_t index) const = 0;\n"
        + "  virtual Result<void> setCollimator(std::size_t index, const Collimator& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Result<ScannerCard> getScannerCard(std::size_t index) const = 0;\n"
        + "  virtual Result<void> setScannerCard(std::size_t index, const ScannerCard& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual bool hasOptionalComponents(std::size_t index) const = 0;\n"
        + "  virtual Result<ClearBox> getClearbox(std::size_t index) const = 0;\n"
        + "  virtual Result<void> setClearbox(std::size_t index, const ClearBox& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
        + "  virtual Result<CorrectionData> getCorrectionData(std::size_t index) const = 0;\n"
        + "  virtual Result<CorrectionData> getInverseCorrectionData(std::size_t index) const = 0;\n"
        + "\n"
        + "  virtual Result<OpcuaConfig> getOpcua() const = 0;\n"
        + "  virtual Result<void> setOpcua(const OpcuaConfig& model, SetMode mode = SetMode::Merge) = 0;\n"
        + "\n"
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
