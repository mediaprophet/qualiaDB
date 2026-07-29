#!/usr/bin/env python3
"""Bounded, scenario-driven verification for coding agents.

The runner never invokes a shell, emits a stable JSON result, classifies
failures, and uses an auto-cleaned temporary directory unless the caller
explicitly promotes evidence into --artifact-dir.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import queue
import subprocess
import tempfile
import threading
import time
from collections import deque
from typing import Any

SCHEMA_VERSION = 1
DEFAULT_BUDGET = 8 * 1024 * 1024


def repo_root() -> pathlib.Path:
    return pathlib.Path(__file__).resolve().parents[2]


def load_scenario(name: str) -> dict[str, Any]:
    manifest = json.loads((pathlib.Path(__file__).parent / "scenarios.json").read_text("utf-8"))
    try:
        return manifest["scenarios"][name]
    except KeyError as exc:
        names = ", ".join(sorted(manifest["scenarios"]))
        raise SystemExit(f"Unknown scenario {name!r}. Available: {names}") from exc


def classify(exit_code: int | None, timed_out: bool, budget_exceeded: bool, output: str) -> str:
    if timed_out:
        return "timeout"
    if budget_exceeded:
        return "artifact-budget-exceeded"
    if exit_code == 0:
        return "passed"
    lowered = output.lower()
    if "could not compile" in lowered or "error[e" in lowered:
        return "compile-failure"
    if "test result: failed" in lowered or "failures:" in lowered:
        return "test-failure"
    if "formatting" in lowered or "diff in " in lowered:
        return "format-failure"
    return "command-failure"


def terminate_process_tree(process: subprocess.Popen[str]) -> None:
    """Stop the command and its children so timed-out compiler jobs do not leak."""
    if process.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        process.kill()


def run_step(
    step: dict[str, Any],
    root: pathlib.Path,
    artifact_root: pathlib.Path,
    remaining_budget: int,
) -> tuple[dict[str, Any], int]:
    command = [str(part) for part in step["command"]]
    timeout_seconds = int(step.get("timeout_seconds", 300))
    log_path = artifact_root / f"{step['id']}.log"
    started = time.monotonic()
    deadline = started + timeout_seconds
    env = os.environ.copy()
    env["CARGO_TERM_COLOR"] = "never"
    env["RUST_BACKTRACE"] = env.get("RUST_BACKTRACE", "1")
    try:
        process = subprocess.Popen(
            command,
            cwd=root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            shell=False,
        )
    except OSError as error:
        message = f"{type(error).__name__}: {error}\n"
        log_path.write_text(message, "utf-8")
        return (
            {
                "id": step["id"],
                "command": command,
                "exit_code": None,
                "classification": "command-start-failure",
                "passed": False,
                "duration_ms": int((time.monotonic() - started) * 1000),
                "timed_out": False,
                "artifact_budget_exceeded": False,
                "log": log_path.name,
                "output_sample": message,
            },
            len(message.encode("utf-8")),
        )
    written = 0
    sample_parts: deque[str] = deque()
    sample_chars = 0
    timed_out = False
    budget_exceeded = False
    output_queue: queue.Queue[str | None] = queue.Queue()

    def read_output() -> None:
        assert process.stdout is not None
        for output_line in process.stdout:
            output_queue.put(output_line)
        output_queue.put(None)

    reader = threading.Thread(target=read_output, name=f"agent-qa-{step['id']}", daemon=True)
    reader.start()
    stream_closed = False
    with log_path.open("w", encoding="utf-8", newline="\n") as log:
        while True:
            if time.monotonic() > deadline:
                timed_out = True
                terminate_process_tree(process)
                break
            try:
                line = output_queue.get(timeout=0.05)
            except queue.Empty:
                line = ""
            if line is None:
                stream_closed = True
            elif line:
                encoded_size = len(line.encode("utf-8"))
                if written + encoded_size > remaining_budget:
                    budget_exceeded = True
                    terminate_process_tree(process)
                    break
                log.write(line)
                written += encoded_size
                sample_parts.append(line)
                sample_chars += len(line)
                while sample_chars > 16_000 and len(sample_parts) > 1:
                    sample_chars -= len(sample_parts.popleft())
            if process.poll() is not None and stream_closed:
                break
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            terminate_process_tree(process)
            process.wait(timeout=10)
        reader.join(timeout=1)
        if process.stdout is not None:
            process.stdout.close()
    output_sample = "".join(sample_parts)[-16_000:]
    exit_code = process.returncode
    classification = classify(exit_code, timed_out, budget_exceeded, output_sample)
    result = {
        "id": step["id"],
        "command": command,
        "exit_code": exit_code,
        "classification": classification,
        "passed": classification == "passed",
        "duration_ms": int((time.monotonic() - started) * 1000),
        "timed_out": timed_out,
        "artifact_budget_exceeded": budget_exceeded,
        "log": log_path.name,
        "output_sample": output_sample,
    }
    return result, written


def render_report(result: dict[str, Any]) -> str:
    lines = [
        "# Webizen agent QA report",
        "",
        f"- Scenario: `{result['scenario']}`",
        f"- Result: **{'PASS' if result['passed'] else 'FAIL'}**",
        f"- Started: `{result['started_at']}`",
        f"- Duration: `{result['duration_ms']} ms`",
        f"- Branch: `{result.get('git_branch', 'unknown')}`",
        "",
        "## Steps",
        "",
    ]
    for step in result["steps"]:
        mark = "PASS" if step["passed"] else "FAIL"
        lines.append(
            f"- **{mark}** `{step['id']}` — {step['classification']} "
            f"({step['duration_ms']} ms, log `{step['log']}`)"
        )
    if not result["passed"]:
        lines.extend(["", "## Agent hand-off", ""])
        for step in result["steps"]:
            if not step["passed"]:
                lines.append(
                    f"- Inspect `{step['log']}`. Failure class: `{step['classification']}`."
                )
    lines.append("")
    return "\n".join(lines)


def git_value(root: pathlib.Path, args: list[str]) -> str | None:
    try:
        return subprocess.check_output(
            ["git", *args], cwd=root, text=True, encoding="utf-8", errors="replace"
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def execute(args: argparse.Namespace, artifact_root: pathlib.Path) -> dict[str, Any]:
    root = repo_root()
    scenario = load_scenario(args.scenario)
    artifact_root.mkdir(parents=True, exist_ok=True)
    marker = artifact_root / ".webizen-agent-qa"
    marker.write_text("schema=1\n", "utf-8")
    started_wall = dt.datetime.now(dt.timezone.utc)
    started = time.monotonic()
    remaining = args.max_bytes
    steps = []
    for step in scenario["steps"]:
        result, used = run_step(step, root, artifact_root, remaining)
        steps.append(result)
        remaining -= used
        if not result["passed"] and not args.keep_going:
            break
    result = {
        "schema_version": SCHEMA_VERSION,
        "scenario": args.scenario,
        "description": scenario["description"],
        "passed": len(steps) == len(scenario["steps"]) and all(step["passed"] for step in steps),
        "started_at": started_wall.isoformat(),
        "duration_ms": int((time.monotonic() - started) * 1000),
        "artifact_budget_bytes": args.max_bytes,
        "artifact_bytes_remaining": remaining,
        "git_branch": git_value(root, ["branch", "--show-current"]),
        "git_commit": git_value(root, ["rev-parse", "HEAD"]),
        "steps": steps,
    }
    (artifact_root / "result.json").write_text(
        json.dumps(result, indent=2, ensure_ascii=False) + "\n", "utf-8"
    )
    (artifact_root / "report.md").write_text(render_report(result), "utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", default="contracts")
    parser.add_argument("--artifact-dir", type=pathlib.Path)
    parser.add_argument("--max-bytes", type=int, default=DEFAULT_BUDGET)
    parser.add_argument("--keep-going", action="store_true")
    args = parser.parse_args()
    if args.max_bytes <= 0:
        raise SystemExit("--max-bytes must be positive")

    if args.artifact_dir:
        resolved_artifact_dir = args.artifact_dir.resolve()
        try:
            result = execute(args, resolved_artifact_dir)
        except OSError as error:
            result = {
                "schema_version": SCHEMA_VERSION,
                "scenario": args.scenario,
                "passed": False,
                "classification": "artifact-io-failure",
                "artifact_dir": str(resolved_artifact_dir),
                "error": f"{type(error).__name__}: {error}",
            }
            print(json.dumps(result, ensure_ascii=False))
            return 1
        result["artifact_dir"] = str(resolved_artifact_dir)
        print(json.dumps(result, ensure_ascii=False))
        return 0 if result["passed"] else 1

    with tempfile.TemporaryDirectory(prefix="webizen-agent-qa-") as temporary:
        result = execute(args, pathlib.Path(temporary))
        summary = {
            "schema_version": result["schema_version"],
            "scenario": result["scenario"],
            "passed": result["passed"],
            "duration_ms": result["duration_ms"],
            "steps": [
                {
                    "id": step["id"],
                    "passed": step["passed"],
                    "classification": step["classification"],
                    "output_sample": step["output_sample"],
                }
                for step in result["steps"]
            ],
            "artifacts_retained": False,
        }
        print(json.dumps(summary, ensure_ascii=False))
        return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
