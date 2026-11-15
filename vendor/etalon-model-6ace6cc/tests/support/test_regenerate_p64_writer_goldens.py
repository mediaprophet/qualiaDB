"""Small pure tests: python -m unittest discover -s <this directory> -p test_*.py."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from regenerate_p64_writer_goldens import promote_file


class PromotionTests(unittest.TestCase):
    def test_success_and_existing_file_refusal(self):
        with tempfile.TemporaryDirectory(prefix="p64-promotion-test-") as temporary:
            root = Path(temporary)
            source, target = root / "source", root / "target"
            source.write_bytes(b"verified bytes")
            promote_file(source, target)
            self.assertEqual(target.read_bytes(), b"verified bytes")
            source.write_bytes(b"replacement")
            with self.assertRaises(FileExistsError):
                promote_file(source, target)
            self.assertEqual(target.read_bytes(), b"verified bytes")

    def test_partial_write_failure_removes_only_incomplete_file(self):
        with tempfile.TemporaryDirectory(prefix="p64-promotion-test-") as temporary:
            root = Path(temporary)
            source, target, prior = root / "source", root / "target", root / "prior"
            source.write_bytes(b"verified bytes")
            prior.write_bytes(b"earlier validated output")
            original_open = Path.open

            class FailingWrite:
                def __init__(self, handle):
                    self.handle = handle

                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    self.handle.close()

                def write(self, data):
                    self.handle.write(data[:3])
                    self.handle.flush()
                    raise OSError("injected partial write failure")

            def injected_open(path, mode="r", *args, **kwargs):
                handle = original_open(path, mode, *args, **kwargs)
                return FailingWrite(handle) if path == target and mode == "xb" else handle

            with patch.object(Path, "open", injected_open):
                with self.assertRaisesRegex(OSError, "injected partial write failure"):
                    promote_file(source, target)
            self.assertFalse(target.exists())
            self.assertEqual(prior.read_bytes(), b"earlier validated output")
            self.assertEqual(source.read_bytes(), b"verified bytes")


if __name__ == "__main__":
    unittest.main()
