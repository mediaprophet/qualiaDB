"""Regenerate the tiny legacy FFN ternary golden from an immutable Qualia checkout.

  python regenerate_p64_stream_golden.py verify
  python regenerate_p64_stream_golden.py generate --qualia-root PATH
    --dependencies PATH/target/debug/deps --output NEW_DIRECTORY

Uses cached Rust 1.96.1 dependencies, no network/GPU/Cargo build. TemporaryDirectory
owns compiler artifacts; 32 MiB is a post-compilation retention check, not a peak
compiler bound. The only promoted file is SHA256-verified and below 64 KiB.
This qualifies serialization of an actual ternary transform, not Q4/AWQ parity.
"""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

from regenerate_p64_writer_goldens import REVISION, SOURCE_PATHS, promote_file

NAME = "ffn-ternary-page9.p64"
SHA256 = "b4646e8180c7fca6943fb94cae9464574a60d2badda6a97d8c42d35642b71ac8"


def verify(path):
    if path.stat().st_size > 65536:
        raise ValueError("golden exceeds 64 KiB retention budget")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != SHA256:
        raise ValueError(f"SHA256 mismatch: {digest}")
    print(f"verified {NAME}: {digest}; legacy {REVISION}")


def generate(args):
    root = args.qualia_root.resolve(strict=True)
    dependencies = args.dependencies.resolve(strict=True)
    subprocess.run(["git", "-c", f"safe.directory={root.as_posix()}", "-C", str(root),
                    "diff", "--quiet", REVISION, "--", *SOURCE_PATHS], check=True)
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve(strict=True)
    if (output / NAME).exists():
        raise ValueError("refusing to overwrite existing golden")
    with tempfile.TemporaryDirectory(prefix="p64-stream-reference-", dir=output) as temporary:
        scratch = Path(temporary)
        template = Path(__file__).with_name("legacy_p64_stream_generator.rs").read_text(encoding="utf-8")
        (scratch / "generate.rs").write_text(template.replace("@QUALIA_ROOT@", root.as_posix().replace('"', '\\"')), encoding="utf-8")
        manifold = (root / SOURCE_PATHS[-2]).read_text(encoding="utf-8")
        start = manifold.index("#[repr(C)]")
        (scratch / "manifold_coordinate.rs").write_text(manifold[start:manifold.index("#[repr(u8)]", start)], encoding="utf-8")
        lib = (root / SOURCE_PATHS[-1]).read_text(encoding="utf-8")
        start = lib.index("pub const fn q_hash(s: &str) -> u64 {")
        (scratch / "q_hash.rs").write_text(lib[start:lib.index("\n}", start) + 2], encoding="utf-8")
        executable = scratch / "generate.exe"
        command = ["rustup", "run", "1.96.1", "rustc", "--edition=2021", "-D", "warnings",
                   str(scratch / "generate.rs"), "-L", f"dependency={dependencies}"]
        for name in ["serde", "serde_json", "bytemuck", "half", "memmap2"]:
            matches = list(dependencies.glob(f"lib{name}-*.rlib"))
            if not matches:
                raise ValueError(f"missing prebuilt {name}")
            selected = max(matches, key=lambda path: path.stat().st_mtime_ns)
            command.extend(["--extern", f"{name}={selected}"])
        subprocess.run([*command, "-o", str(executable)], check=True, timeout=180)
        if sum(path.stat().st_size for path in scratch.rglob("*") if path.is_file()) > 32 * 1024 * 1024:
            raise ValueError("compiled scratch exceeds 32 MiB post-compilation retention limit")
        generated = scratch / NAME
        subprocess.run([str(executable), str(generated)], check=True, timeout=60)
        verify(generated)
        promote_file(generated, output / NAME)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("verify")
    check.add_argument("file", nargs="?", type=Path,
                       default=Path(__file__).with_name("p64_stream_golden") / NAME)
    build = commands.add_parser("generate")
    build.add_argument("--qualia-root", type=Path, required=True)
    build.add_argument("--dependencies", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    verify(args.file) if args.command == "verify" else generate(args)


if __name__ == "__main__":
    main()
