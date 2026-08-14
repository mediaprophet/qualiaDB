"""Bounded, streaming GGUF metadata and tensor-table reader.

This module deliberately never maps or reads tensor payloads.  It validates the
header, KV section, and tensor table with checked offsets, so it remains useful
for multi-gigabyte model files on memory-constrained machines.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import BinaryIO, Optional


class GgufError(ValueError):
    """The source does not satisfy the supported GGUF structural contract."""


_SCALAR_WIDTHS = {
    0: 1,  # uint8
    1: 1,  # int8
    2: 2,  # uint16
    3: 2,  # int16
    4: 4,  # uint32
    5: 4,  # int32
    6: 4,  # float32
    7: 1,  # bool
    10: 8,  # uint64
    11: 8,  # int64
    12: 8,  # float64
}
_TYPE_NAMES = {
    0: "uint8", 1: "int8", 2: "uint16", 3: "int16", 4: "uint32",
    5: "int32", 6: "float32", 7: "bool", 8: "string", 9: "array",
    10: "uint64", 11: "int64", 12: "float64",
}
_INTERESTING_KEYS = {
    "general.architecture", "general.name", "general.alignment",
    "tokenizer.ggml.pre", "tokenizer.ggml.model", "tokenizer.chat_template",
}


@dataclass(frozen=True)
class TensorSummary:
    name: str
    dimensions: list[int]
    ggml_type: int
    data_offset: int


@dataclass(frozen=True)
class GgufInspection:
    path: str
    file_bytes: int
    version: int
    key_value_count: int
    tensor_count: int
    alignment: int
    tensor_data_offset: int
    metadata: dict[str, object]
    tensors: list[TensorSummary]
    matching_tensor_count: int
    tensors_truncated: bool

    def as_dict(self) -> dict[str, object]:
        result = asdict(self)
        result["tensors"] = [asdict(tensor) for tensor in self.tensors]
        return result


class _Reader:
    def __init__(self, handle: BinaryIO, file_bytes: int, max_string_bytes: int):
        self.handle = handle
        self.file_bytes = file_bytes
        self.max_string_bytes = max_string_bytes
        self.offset = 0

    def _checked_end(self, size: int, label: str) -> int:
        if size < 0 or size > self.file_bytes - self.offset:
            raise GgufError(f"truncated {label} at byte {self.offset}")
        return self.offset + size

    def read(self, size: int, label: str) -> bytes:
        self._checked_end(size, label)
        data = self.handle.read(size)
        if len(data) != size:
            raise GgufError(f"unable to read {label} at byte {self.offset}")
        self.offset += size
        return data

    def skip(self, size: int, label: str) -> None:
        self._checked_end(size, label)
        self.handle.seek(size, 1)
        self.offset += size

    def u32(self, label: str) -> int:
        return int.from_bytes(self.read(4, label), "little")

    def u64(self, label: str) -> int:
        return int.from_bytes(self.read(8, label), "little")

    def string(self, label: str, capture: bool) -> Optional[str]:
        size = self.u64(f"{label} length")
        if size > self.max_string_bytes:
            raise GgufError(
                f"{label} is {size} bytes; exceeds --max-string-kib limit"
            )
        data = self.read(size, label)
        if not capture:
            return None
        return data.decode("utf-8", errors="replace")

    def value(self, value_type: int, capture: bool) -> object:
        if value_type == 8:
            return self.string("GGUF string", capture)
        if value_type == 9:
            element_type = self.u32("GGUF array element type")
            count = self.u64("GGUF array count")
            if element_type == 9 or element_type not in _TYPE_NAMES:
                raise GgufError(f"unsupported GGUF array element type {element_type}")
            if element_type == 8:
                if count > self.file_bytes // 8:
                    raise GgufError("impossible GGUF string-array count")
                for _ in range(count):
                    self.string("GGUF string-array item", False)
            else:
                width = _SCALAR_WIDTHS[element_type]
                if count > (self.file_bytes - self.offset) // width:
                    raise GgufError("truncated GGUF array")
                self.skip(count * width, "GGUF array")
            return {"array_type": _TYPE_NAMES[element_type], "count": count} if capture else None
        width = _SCALAR_WIDTHS.get(value_type)
        if width is None:
            raise GgufError(f"unsupported GGUF value type {value_type}")
        raw = self.read(width, "GGUF scalar")
        if not capture:
            return None
        if value_type == 7:
            if raw not in (b"\x00", b"\x01"):
                raise GgufError("GGUF boolean is not 0 or 1")
            return raw != b"\x00"
        if value_type in (6, 12):
            import struct
            return struct.unpack("<f" if value_type == 6 else "<d", raw)[0]
        return int.from_bytes(raw, "little", signed=value_type in (1, 3, 5, 11))


def _aligned(value: int, alignment: int) -> int:
    if alignment <= 0 or alignment & (alignment - 1):
        raise GgufError(f"GGUF alignment {alignment} is not a positive power of two")
    return (value + alignment - 1) & -alignment


def inspect_gguf(
    path: Path,
    *,
    max_metadata_bytes: int = 64 * 1024 * 1024,
    max_string_bytes: int = 1024 * 1024,
    max_entries: int = 1_000_000,
    max_tensors: int = 50,
    tensor_prefix: Optional[str] = None,
) -> GgufInspection:
    """Inspect a GGUF header without allocating model-sized memory."""
    file_bytes = path.stat().st_size
    if file_bytes < 24:
        raise GgufError("file is too short for a GGUF header")
    with path.open("rb") as handle:
        reader = _Reader(handle, file_bytes, max_string_bytes)
        if reader.read(4, "GGUF magic") != b"GGUF":
            raise GgufError("missing GGUF magic")
        version = reader.u32("GGUF version")
        if version not in (2, 3):
            raise GgufError(f"unsupported GGUF version {version}; expected 2 or 3")
        tensor_count = reader.u64("tensor count")
        key_value_count = reader.u64("KV count")
        if tensor_count > max_entries or key_value_count > max_entries:
            raise GgufError(f"entry count exceeds --max-entries ({max_entries})")

        metadata_start = reader.offset
        metadata: dict[str, object] = {}
        alignment = 32
        for _ in range(key_value_count):
            if reader.offset - metadata_start > max_metadata_bytes:
                raise GgufError("KV section exceeds --max-metadata-mib limit")
            key = reader.string("GGUF key", True)
            assert key is not None
            value_type = reader.u32("GGUF value type")
            capture = key in _INTERESTING_KEYS or key.endswith((
                "block_count", "embedding_length", "context_length",
                "attention.head_count", "attention.head_count_kv",
                "attention.key_length", "feed_forward_length", "rope.dimension_count",
            ))
            value = reader.value(value_type, capture)
            if reader.offset - metadata_start > max_metadata_bytes:
                raise GgufError("KV section exceeds --max-metadata-mib limit")
            if capture:
                metadata[key] = value
            if key == "general.alignment" and isinstance(value, int):
                alignment = value

        tensors: list[TensorSummary] = []
        matching_tensor_count = 0
        max_tensor_data_offset = 0
        prefix = tensor_prefix.encode("utf-8") if tensor_prefix else None
        for _ in range(tensor_count):
            raw_name = reader.string("tensor name", True)
            assert raw_name is not None
            dimensions_count = reader.u32("tensor dimensions count")
            if dimensions_count > 8:
                raise GgufError(f"tensor {raw_name!r} has {dimensions_count} dimensions")
            dimensions = [reader.u64("tensor dimension") for _ in range(dimensions_count)]
            ggml_type = reader.u32("tensor GGML type")
            data_offset = reader.u64("tensor data offset")
            max_tensor_data_offset = max(max_tensor_data_offset, data_offset)
            if prefix is None or raw_name.encode("utf-8", errors="replace").startswith(prefix):
                matching_tensor_count += 1
                if len(tensors) < max_tensors:
                    tensors.append(TensorSummary(raw_name, dimensions, ggml_type, data_offset))

        tensor_data_offset = _aligned(reader.offset, alignment)
        if tensor_data_offset > file_bytes:
            raise GgufError("tensor data offset is beyond end of file")
        if tensor_count and max_tensor_data_offset >= file_bytes - tensor_data_offset:
            raise GgufError("tensor payload offset is beyond end of file")
    return GgufInspection(
        path=str(path), file_bytes=file_bytes, version=version,
        key_value_count=key_value_count, tensor_count=tensor_count,
        alignment=alignment, tensor_data_offset=tensor_data_offset,
        metadata=metadata, tensors=tensors,
        matching_tensor_count=matching_tensor_count,
        tensors_truncated=len(tensors) < matching_tensor_count,
    )
