# Qualia MCP Server

Native Qualia graph engine and modality evaluators exposed via Model Context Protocol.

## Start (IDE / stdio)

```powershell
cargo run -p qualia-cli -- mcp serve --transport stdio
```

## Start (background TCP for local tools)

```powershell
cargo run -p qualia-cli -- service start
# or MCP only:
cargo run -p qualia-cli -- mcp start --bind 127.0.0.1:4244
```

## Graph daemon (required for native tests)

```powershell
cargo run -p qualia-cli -- daemon start --dev --port 4242
```

`qualia-cli service start` starts both daemon (4242) and MCP (4244).

## Run docs tests via MCP

Call `run_docs_tests` with `{ "mode": "logic" | "wasm" | "native" | "both" }`.