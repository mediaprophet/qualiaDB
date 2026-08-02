import importlib.util
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("mobile_wasm_lab", ROOT / "tools" / "mobile_wasm_lab.py")
LAB = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(LAB)


class AnatomyAssetStagingTests(unittest.TestCase):
    def make_docs(self, root: Path) -> Path:
        docs = root / "docs"
        playground = docs / "playground"
        playground.mkdir(parents=True)
        for stem in LAB.ANATOMY_ASSETS:
            (playground / f"{stem}.qualia").write_bytes(LAB.QBDL_MAGIC + bytes(2048))
        return docs

    def test_stages_valid_legacy_bundles_under_canonical_names(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            docs = self.make_docs(Path(temp))
            receipts = LAB.stage_anatomy_assets(docs)
            self.assertEqual([item["name"] for item in receipts], ["anatomy-male.hmc", "anatomy-female.hmc"])
            self.assertTrue(all(item["magic"] == "QBDL" for item in receipts))
            self.assertTrue(all((docs / item["publicPath"].lstrip("/")).is_file() for item in receipts))
            self.assertTrue(all((docs / "playground" / f"{stem}.qualia").is_file() for stem in LAB.ANATOMY_ASSETS))

    def test_rejects_invalid_magic_before_staging(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            docs = self.make_docs(Path(temp))
            (docs / "playground" / "anatomy-female.qualia").write_bytes(b"NOPE" + bytes(2048))
            with self.assertRaisesRegex(ValueError, "invalid QBDL magic"):
                LAB.stage_anatomy_assets(docs)
            self.assertFalse((docs / "playground" / "anatomy-female.hmc").exists())

    def test_rejects_invalid_existing_canonical_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            docs = self.make_docs(Path(temp))
            (docs / "playground" / "anatomy-male.hmc").write_bytes(b"BAD!" + bytes(2048))
            with self.assertRaisesRegex(ValueError, "invalid QBDL magic"):
                LAB.stage_anatomy_assets(docs)


if __name__ == "__main__":
    unittest.main()
