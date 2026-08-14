# Qualia diagnostics

Production diagnostic tools are intentionally separate from temporary browser
logs and benchmark outputs. They never read a whole GGUF file into heap memory,
and retained browser evidence requires an explicit, new output directory.

## GGUF inspection

`gguf_inspect.py` reads only the header, key/value section, and tensor table.
Tensor payloads are never mapped or copied. The default metadata limit is 64 MiB
and the default per-string limit is 1 MiB; lower both limits on constrained
devices.

```powershell
python tools/diagnostics/gguf_inspect.py C:\LLM_Models\model.gguf --json --sha256
python tools/diagnostics/gguf_inspect.py C:\LLM_Models\model.gguf `
  --max-metadata-mib 16 --max-tensors 20 --tensor-prefix blk.0.
python tools/diagnostics/test_gguf_reader.py
```

The inspector rejects malformed lengths, unsupported GGUF versions, oversized
metadata, and tensor tables that point past the file. It is an inspection tool,
not an inference loader.

## Browser/WebGPU smoke receipts

Install the pinned development dependency within this directory; do not commit
`node_modules` or generated receipts.

```powershell
cd tools/diagnostics
npm install
npx playwright install chromium
npm run browser-smoke -- `
  --url http://127.0.0.1:8000/auto-bench.html `
  --out C:\qualia-evidence\browser-smoke-2026-08-14 `
  --wait-for '#bench-start'
```

The command records browser console/page errors, security isolation, WebGPU
adapter limits, and an optional page completion global in `receipt.json`. It is
headless and uses normal browser GPU policy by default. `--headful` and
`--unsafe-webgpu` are explicit opt-ins for local diagnostics only.
Add `--require-webgpu` when an unavailable adapter must fail the smoke test;
without it, adapter availability is recorded but does not make a static-page
smoke test fail.
Completion-global evidence is clipped at 32 KiB before it enters the receipt.

For a benchmark page that exposes `window.__benchResult`, add:

```powershell
--completion-global __benchResult
```

The `--out` directory must not exist. This prevents accidental overwrite and
keeps retention explicit. Receipts have a 256 KiB default byte budget; adjust it
only when the evidence genuinely needs more space.
