# Webizen agent QA

This runner gives coding agents a deterministic, bounded verification contract.
It executes commands without a shell and returns one JSON object on stdout.
Failures are classified as compile, test, format, timeout, command or artifact
budget failures.

```powershell
py -3 tools/agent_qa/runner.py --scenario contracts
py -3 tools/agent_qa/runner.py --scenario check
py -3 tools/agent_qa/runner.py --scenario ui
py -3 tools/agent_qa/runner.py --scenario full
py -3 tools/agent_qa/runner.py --scenario full --artifact-dir target\agent-qa\full
```

The `ui` scenario is self-contained: it builds the current WASM shell, serves
the generated assets on loopback, runs the 0.0.28 Playwright contract, and
tears the server down. It does not require a previously running desktop app.
The native command boundary is verified by the desktop Rust contract step.

`full` uses `scope.json` for its formatting gate. This is deliberate: an agent
can enforce its programme without formatting or claiming unrelated concurrent
work elsewhere in a dirty worktree. Repository-wide formatting may still be
audited separately with `cargo fmt --all -- --check`.

Without `--artifact-dir`, a uniquely named temporary directory is removed
automatically after the JSON summary is emitted. Explicitly retained runs
contain:

- `.webizen-agent-qa` ownership marker;
- `result.json` for machine consumers;
- `report.md` for human hand-off;
- one bounded log per step.

The default total log budget is 8 MiB. Exceeding it terminates the current step
and fails closed.
