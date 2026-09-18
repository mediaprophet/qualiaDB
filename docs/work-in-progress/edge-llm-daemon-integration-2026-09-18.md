# Edge LLM daemon integration

## Status

Implemented and locally verified — 2026-09-18

## Problem statement

`edge-llm.html` labelled a healthy daemon as inference-capable, but sent
unsupported `llm_infer` frames to `/qualia-bridge`. The native daemon already
exposes the supported local-model contract through `/llm/models`,
`/llm/jobs/start`, `/llm/jobs/events`, and `/llm/jobs/cancel`.

## Implementation plan

1. Replace the page's invented WebSocket inference protocol with the supported
   loopback HTTP plus Server-Sent Events contract.
2. Populate the live catalogue from `/llm/models`; retain the bundled catalogue
   only for the explicitly labelled demo mode.
3. Make live mode fail closed when no discovered local model is selected, and
   make the daemon banner distinguish health from inference readiness.
4. Render streamed tokens and final/error/cancel states from job SSE events;
   expose a cancel action that calls `/llm/jobs/cancel`.
5. Correct the on-page protocol and telemetry copy so it describes real daemon
   behaviour rather than the benchmark-only WebSocket service.
6. Verify the supported daemon endpoints against a running local daemon and run
   a DOM contract harness against the page script. A real-generation check is
   conditional on an operator-provisioned GGUF/P64 model.

## Boundaries

- This change reuses the established daemon API; it does not add a second LLM
  execution path or alter the native hot-path inference implementation.
- Model activation remains explicit. The page may run only a model reported by
  `/llm/models` and does not mount arbitrary filesystem paths.

## Verification record

- The local `0.0.39` daemon returned `200` from `/llm/models`, including the
  expected GitHub Pages CORS origin.
- A bounded `POST /llm/jobs/start` with a nonexistent model returned the
  structured `model_not_found` diagnostic expected by the page.
- A DOM-free browser-contract harness parsed the page script, supplied a
  discovered model plus streamed SSE `token` and `done` events, and verified
  that it sent `model_path`, opened the advertised event path, and restored the
  send control when the job completed.
- No real generation was attempted because this daemon reported zero installed
  local models. That is now an explicit UI state rather than a misleading
  inference-ready badge.
