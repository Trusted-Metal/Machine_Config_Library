"""File_Version 1.0 stable model facade."""
from __future__ import annotations

from pathlib import Path
from typing import Any, Iterator, Optional

from machine_config.builder import MockConfigBuilder
from machine_config.capabilities.errors import CapabilityError, SessionClosedError, capability_error
from machine_config.capabilities.generated import SetMode
from machine_config.capabilities.merge import apply_set_mode, snapshot
from machine_config.capabilities.result import Result, err, ok
from machine_config.reader import MachineConfigReader, config_from_dict
from machine_config.writer import MachineConfigWriter


class MachineConfigFileV10:
    def __init__(self, data: dict[str, Any], path: str | None, version: str) -> None:
        self._data = data
        self._path = path
        self._version = version
        self._closed = False

    @classmethod
    def open(cls, path: str | Path) -> Result["MachineConfigFileV10", CapabilityError]:
        try:
            reader = MachineConfigReader(str(path))
            config = reader.parse()
            fv = (config.meta.file_version or "1.0").strip() or "1.0"
            if fv != "1.0":
                return err(
                    capability_error(
                        "UnsupportedVersion",
                        f'No adapter for File_Version "{fv}" (v1.0 facade)',
                    )
                )
            data = reader._config_to_dict(config, include_binary=True)
            return ok(cls(data, str(path), fv))
        except Exception as e:  # noqa: BLE001
            return err(capability_error("IoError", str(e)))

    @classmethod
    def create(cls, version: str = "1.0") -> Result["MachineConfigFileV10", CapabilityError]:
        import os
        import tempfile

        fv = version.strip() or "1.0"
        if fv != "1.0":
            return err(
                capability_error(
                    "UnsupportedVersion",
                    f'create() unsupported for File_Version "{fv}"',
                )
            )
        config = MockConfigBuilder(n_lasers=1).build()
        config.meta.file_version = "1.0"
        fd, tmp = tempfile.mkstemp(suffix=".h5")
        os.close(fd)
        try:
            MachineConfigWriter(config).write(tmp)
            opened = cls.open(tmp)
            if not opened.ok:
                return opened
            opened.value._path = None
            return opened
        except Exception as e:  # noqa: BLE001
            return err(capability_error("IoError", str(e)))
        finally:
            try:
                os.unlink(tmp)
            except OSError:
                pass

    def _assert_open(self) -> None:
        if self._closed:
            raise SessionClosedError()

    def file_version(self) -> str:
        return self._version

    def meta(self) -> "_NodeHandle":
        self._assert_open()
        return _NodeHandle(self, lambda: self._data["meta"], lambda v: self._data.__setitem__("meta", v))

    def machine(self) -> "_NodeHandle":
        self._assert_open()
        return _NodeHandle(
            self, lambda: self._data["machine"], lambda v: self._data.__setitem__("machine", v)
        )

    def _train(self, index: int) -> "_TrainHandle":
        return _TrainHandle(self, index)

    def optical_train(self, index: int) -> Result["_TrainHandle", CapabilityError]:
        self._assert_open()
        n = len(self._data["optical_trains"])
        if index < 0 or index >= n:
            return err(
                capability_error(
                    "InvalidIndex",
                    f"optical train index {index} out of range [0, {n})",
                )
            )
        return ok(self._train(index))

    def optical_trains(self) -> "_TrainCollection":
        self._assert_open()
        return _TrainCollection(self)

    def opcua(self) -> Result["_NodeHandle", CapabilityError]:
        self._assert_open()
        if self._data.get("opcua") is None:
            return err(capability_error("NotPresent", "OPCUA group is not present"))
        return ok(
            _NodeHandle(
                self,
                lambda: self._data["opcua"],
                lambda v: self._data.__setitem__("opcua", v),
            )
        )

    def save(self, path: str | None = None) -> Result[None, CapabilityError]:
        self._assert_open()
        out = path or self._path
        if not out:
            return err(
                capability_error(
                    "ValidationError", "save() requires a path for create()-d files"
                )
            )
        try:
            config = config_from_dict(self._data)
            MachineConfigWriter(config).write(out)
            self._path = out
            return ok(None)
        except Exception as e:  # noqa: BLE001
            return err(capability_error("IoError", str(e)))

    def close(self) -> None:
        self._closed = True


class _NodeHandle:
    def __init__(self, file: MachineConfigFileV10, getter, setter) -> None:
        self._file = file
        self._getter = getter
        self._setter = setter

    def get_model(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._getter())

    def set_model(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        self._file._assert_open()
        self._setter(apply_set_mode(self._getter(), model, mode))
        return ok(None)


class _TrainHandle:
    def __init__(self, file: MachineConfigFileV10, index: int) -> None:
        self._file = file
        self._index = index

    def _train(self) -> dict[str, Any]:
        return self._file._data["optical_trains"][self._index]

    def get_model(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._train())

    def set_model(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        self._file._assert_open()
        trains = self._file._data["optical_trains"]
        trains[self._index] = apply_set_mode(trains[self._index], model, mode)
        return ok(None)

    def get_scanner(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._train()["scanner"])

    def set_scanner(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        return self._set_child("scanner", model, mode)

    def get_light_source(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._train()["light_source"])

    def set_light_source(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        return self._set_child("light_source", model, mode)

    def get_collimator(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._train()["collimator"])

    def set_collimator(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        return self._set_child("collimator", model, mode)

    def get_scanner_card(self) -> dict[str, Any]:
        self._file._assert_open()
        return snapshot(self._train()["scanner_card"])

    def set_scanner_card(
        self, model: dict[str, Any], mode: SetMode = SetMode.MERGE
    ) -> Result[None, CapabilityError]:
        return self._set_child("scanner_card", model, mode)

    def _set_child(
        self, key: str, model: dict[str, Any], mode: SetMode
    ) -> Result[None, CapabilityError]:
        self._file._assert_open()
        train = self._train()
        train[key] = apply_set_mode(train[key], model, mode)
        return ok(None)

    def optional_components(self) -> Optional["_OptionalComponentsHandle"]:
        self._file._assert_open()
        oc = self._train().get("optional_components") or {}
        if oc.get("clearbox") is None:
            return None
        return _OptionalComponentsHandle(self._file, self._index)


class _OptionalComponentsHandle:
    def __init__(self, file: MachineConfigFileV10, index: int) -> None:
        self._file = file
        self._index = index

    def clearbox(self) -> Result[_NodeHandle, CapabilityError]:
        self._file._assert_open()
        train = self._file._data["optical_trains"][self._index]
        oc = train.get("optional_components") or {}
        if oc.get("clearbox") is None:
            return err(capability_error("NotPresent", "clearbox is not present"))

        def getter() -> dict[str, Any]:
            return self._file._data["optical_trains"][self._index]["optional_components"][
                "clearbox"
            ]

        def setter(v: dict[str, Any]) -> None:
            self._file._data["optical_trains"][self._index]["optional_components"][
                "clearbox"
            ] = v

        return ok(_NodeHandle(self._file, getter, setter))


class _TrainCollection:
    def __init__(self, file: MachineConfigFileV10) -> None:
        self._file = file

    def __len__(self) -> int:
        self._file._assert_open()
        return len(self._file._data["optical_trains"])

    def get(self, index: int) -> Result[_TrainHandle, CapabilityError]:
        return self._file.optical_train(index)

    def __iter__(self) -> Iterator[_TrainHandle]:
        self._file._assert_open()
        for i in range(len(self)):
            yield self._file._train(i)
