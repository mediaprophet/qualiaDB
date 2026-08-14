"""Regression tests for the bounded GGUF reader."""

from __future__ import annotations

import struct
import tempfile
import unittest
from pathlib import Path

from gguf_reader import GgufError, inspect_gguf


def string(value: str) -> bytes:
    encoded = value.encode()
    return struct.pack("<Q", len(encoded)) + encoded


def kv_string(key: str, value: str) -> bytes:
    return string(key) + struct.pack("<I", 8) + string(value)


def kv_u32(key: str, value: int) -> bytes:
    return string(key) + struct.pack("<I", 4) + struct.pack("<I", value)


def valid_fixture() -> bytes:
    metadata = b"".join((
        kv_string("general.architecture", "llama"),
        kv_u32("general.alignment", 32),
        kv_string("tokenizer.ggml.pre", "llama-bpe"),
    ))
    tensor = string("token_embd.weight") + struct.pack("<I", 2)
    tensor += struct.pack("<QQIQ", 4, 8, 0, 0)
    header = b"GGUF" + struct.pack("<IQQ", 3, 1, 3)
    table = header + metadata + tensor
    return table + (b"\x00" * ((-len(table)) % 32)) + (b"\x00" * 32)


class GgufReaderTests(unittest.TestCase):
    def write(self, data: bytes) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "fixture.gguf"
        path.write_bytes(data)
        return path

    def test_reads_minimal_valid_gguf_without_tensor_payload_loading(self) -> None:
        report = inspect_gguf(self.write(valid_fixture()))
        self.assertEqual(report.version, 3)
        self.assertEqual(report.metadata["general.architecture"], "llama")
        self.assertEqual(report.alignment, 32)
        self.assertEqual(report.tensors[0].dimensions, [4, 8])

    def test_rejects_truncated_header(self) -> None:
        with self.assertRaises(GgufError):
            inspect_gguf(self.write(b"GGUF"))

    def test_enforces_metadata_string_limit(self) -> None:
        data = b"GGUF" + struct.pack("<IQQ", 3, 0, 1)
        data += kv_string("general.name", "x" * 32)
        with self.assertRaises(GgufError):
            inspect_gguf(self.write(data), max_string_bytes=8)

    def test_enforces_total_metadata_limit_after_final_entry(self) -> None:
        data = b"GGUF" + struct.pack("<IQQ", 3, 0, 1)
        data += kv_string("general.name", "x" * 32)
        with self.assertRaises(GgufError):
            inspect_gguf(self.write(data), max_metadata_bytes=16)


if __name__ == "__main__":
    unittest.main()
