"""Reproduce/check P64 goldens without a Qualia Cargo dependency or downloads.

Examples (from an Etalon checkout):
  python crates/etalon-model/tests/support/regenerate_p64_writer_goldens.py verify
  python crates/etalon-model/tests/support/regenerate_p64_writer_goldens.py generate \
    --qualia-root C:/Projects/qualia-27062026 \
    --dependencies C:/Projects/qualia-27062026/target/debug/deps \
    --output C:/caller-selected/p64-goldens

Generation requires the recorded immutable Qualia sources and already-built
Rust 1.96.1 dependencies. Repeated --extern NAME=PATH overrides automatic rlib
selection. No network, GPU, Cargo build or production file edit is performed.
Scratch uses a uniquely named TemporaryDirectory with cleanup on every exit.
The 32 MiB compiled-scratch check is a post-compilation retention check, not a
hard bound on peak rustc output. Verified golden output is bounded before promotion.
Retained output is nine verified files; the source fixture names their profiles.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

REVISION = "07018bb8dbb123f105a91609532b9b564e54a2af"
EXPECTED = {
    "gguf-page8.p64": "7793c8397b60e0f3c3e50d90d5759cd5520129ee3ad9597522c7897ff1944453",
    "gguf-page9.p64": "3b9eb20b98badd44ef14c6e442c3ea9c983016f3da4d11667f0f4049613fb9b6",
    "gguf-page12.p64": "71f37813cf6a63076f849f70c5cd4d3af452f3ab1b07b60c2b5f5b121bea4741",
    "raw-page8.p64": "94a939b08f341ebdfa0a11ccf6b6eccf6c8660a11587cb1cb5663129250457ec",
    "raw-page9.p64": "e933e63a84663040d193cb0481a8ee2f5676ffa82c1795b6858e6272ed4ffd06",
    "raw-page12.p64": "924ba2ee7b96efed62f9b847ae129e4d683a4c43751f6bd971d90fbf46cc2275",
    "vision-page8.p64": "25d9ac5f002c9f27ed273e41336a84fc7531cc81371420864a8c9a0e4b431c73",
    "vision-page9.p64": "0e1089491b33e949161b4772585957ad9e7202b7b99a2a078d1ce264228916be",
    "vision-page12.p64": "ec8a0959384b981f276c88d346a4413a1cdc69735bd880635851a76e8181560c",
}
SOURCE_PATHS = [
    "crates/qualia-core-db/src/q42/p64_weight",
    "crates/qualia-core-db/src/inference/gguf_sharder",
    "crates/qualia-core-db/src/inference/safetensor.rs",
    "crates/qualia-core-db/src/inference/ggml_quants.rs",
    "crates/qualia-core-db/src/inference/ternary.rs",
    "crates/qualia-core-db/src/inference/tensor_roles.rs",
    "crates/qualia-core-db/src/container_10d/crc32c.rs",
    "crates/qualia-core-db/src/modalities/manifold.rs",
    "crates/qualia-core-db/src/lib.rs",
]
BUDGET = 32 * 1024 * 1024


def promote_file(source, target):
    """Exclusively create one verified file; remove our incomplete file on failure."""
    payload = source.read_bytes()
    destination = target.open("xb")
    # Opening is outside the cleanup region: refusal must never remove an
    # existing file. Once creation succeeds, this helper owns the new file.
    try:
        with destination:
            if destination.write(payload) != len(payload):
                raise OSError("incomplete fixture write")
    except BaseException:
        target.unlink()
        raise


def verify(directory):
    records = []
    for name, expected in EXPECTED.items():
        path = directory / name
        if path.stat().st_size > 262144:
            raise ValueError(f"fixture exceeds bounded size: {name}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != expected:
            raise ValueError(f"SHA256 mismatch for {name}: {digest} != {expected}")
        records.append({"file": name, "sha256": digest, "bytes": path.stat().st_size})
    print(json.dumps({"verified": len(records), "legacy_revision": REVISION,
                      "files": records}, indent=2))


def generate(args):
    root = args.qualia_root.resolve(strict=True)
    dependencies = args.dependencies.resolve(strict=True)
    # A newer HEAD is permitted only when every consumed source equals the frozen
    # revision. This does not reset or edit the supplied checkout.
    subprocess.run(["git", "-c", f"safe.directory={root.as_posix()}", "-C", str(root),
                    "diff", "--quiet", REVISION, "--", *SOURCE_PATHS], check=True)
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve(strict=True)
    for name in EXPECTED:
        if (output / name).exists():
            raise ValueError(f"refusing to overwrite existing fixture: {output / name}")
    overrides = dict(value.split("=", 1) for value in args.extern)
    with tempfile.TemporaryDirectory(prefix="p64-writer-reference-", dir=output) as temporary:
        scratch = Path(temporary)
        template = Path(__file__).with_name("legacy_p64_writer_generator.rs").read_text(encoding="utf-8")
        escaped = root.as_posix().replace('"', '\\"')
        (scratch / "generate.rs").write_text(template.replace("@QUALIA_ROOT@", escaped), encoding="utf-8")
        manifold = (root / SOURCE_PATHS[-2]).read_text(encoding="utf-8")
        start = manifold.index("#[repr(C)]")
        end = manifold.index("#[repr(u8)]", start)
        (scratch / "manifold_coordinate.rs").write_text(manifold[start:end], encoding="utf-8")
        lib = (root / SOURCE_PATHS[-1]).read_text(encoding="utf-8")
        start = lib.index("pub const fn q_hash(s: &str) -> u64 {")
        end = lib.index("\n}", start) + 2
        (scratch / "q_hash.rs").write_text(lib[start:end], encoding="utf-8")
        executable = scratch / "generate.exe"
        command = ["rustup", "run", "1.96.1", "rustc", "--edition=2021", "-D", "warnings",
                   str(scratch / "generate.rs"), "-L", f"dependency={dependencies}"]
        for name in ["serde", "serde_json", "bytemuck", "half", "memmap2"]:
            if name in overrides:
                path = Path(overrides[name]).resolve(strict=True)
            else:
                matches = list(dependencies.glob(f"lib{name}-*.rlib"))
                if not matches:
                    raise ValueError(f"missing prebuilt {name}; provide --extern {name}=PATH")
                path = max(matches, key=lambda candidate: candidate.stat().st_mtime_ns)
            command.extend(["--extern", f"{name}={path}"])
        command.extend(["-o", str(executable)])
        subprocess.run(command, check=True, timeout=180)
        retained = sum(path.stat().st_size for path in scratch.rglob("*") if path.is_file())
        if retained > BUDGET:
            raise ValueError("compiled scratch exceeds 32 MiB post-compilation retention limit")
        generated = scratch / "verified-output"
        generated.mkdir()
        subprocess.run([str(executable), str(generated)], check=True, timeout=60)
        verify(generated)
        generated_bytes = sum((generated / name).stat().st_size for name in EXPECTED)
        if generated_bytes > 262144:
            raise ValueError("verified fixture set exceeds 256 KiB retention budget")
        # Promote only verified outputs. Failed generation leaves no golden files.
        # Exclusive creation also refuses a destination created after preflight.
        # Promotion is per file; the nine-file set is not an atomic transaction.
        for name in EXPECTED:
            promote_file(generated / name, output / name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("verify")
    check.add_argument("directory", nargs="?", type=Path,
                       default=Path(__file__).with_name("p64_writer_golden"))
    build = sub.add_parser("generate")
    build.add_argument("--qualia-root", required=True, type=Path)
    build.add_argument("--dependencies", required=True, type=Path)
    build.add_argument("--output", required=True, type=Path)
    build.add_argument("--extern", action="append", default=[])
    args = parser.parse_args()
    verify(args.directory) if args.command == "verify" else generate(args)


if __name__ == "__main__":
    main()
