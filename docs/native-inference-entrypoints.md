# Native product inference entrypoints

Implementation checkpoint, 2026-11-04. These routes have not been compiled or
executed as part of this checkpoint. Runtime qualification remains outstanding.

The privileged native embedding must configure the existing authority rather
than accept authority fields in a prompt or tool argument. Its explicit inputs
are `InferenceHostConfiguration` (installer and project trust), the existing
signed admission and adopted `DocumentRevision`, an unlocked configured vault,
and an already created actor signer label. `configure_inference_host` installs
the supplied authority configuration; it does not enroll subjects or create a
signer. Signer creation remains a separate explicit command.

Login requires the actual subject and the complete selected identifier tuple:
value, scheme, issuer, project ID and evidence reference. The signed admission
must associate that selection with the authenticated subject. Person, entity
and agent selections follow this same contract; a hash or a controller key
alone cannot substitute for the association. Multiple identifiers remain
selectable through their complete independent tuples.

Install the native evidence policy with `install_native_inference_policy`,
supplying reviewed rules, `ObservationRequirements`, expiry and expected
generation. Legacy policy installation does not produce a native evidence
policy. Bind an authenticated actor with an explicit `NativePreparationSpec`:
either retained primary/shard model leases plus metadata and input budgets, or
a caller-selected bounded snapshot budget plus input budgets.

The actual consuming routes are:

* Local chat: `login_chat_inference_actor` logs in and binds the explicitly
  selected chat ID. The chat decode path obtains a fresh request for each turn.
* PoET: `bind_poet_inference_actor` delegates to the existing native HTTP owner.
  PoET's Rust browser UI sends the existing configured Webizen transport token
  in `x-qualia-token` on its `/llm/` JSON and streaming fetch requests. The first
  LLM action asks for that existing token if the embedding has not supplied it
  through the exported `set_native_daemon_token` function. The token stays in
  memory, scoped to the connected daemon URL. It does not authorize an actor or
  install policy. Fetch streaming preserves the server's SSE events, reports
  authentication/disconnect/parse failures to the existing error UI and aborts
  the response body on close/replacement, allowing native disconnect cancellation.
* MCP: `bind_mcp_inference_actor` explicitly delegates to the local MCP surface.
  Both the modular `qualia-mcp` tools and legacy core tools bind each inference
  agent through that retained authenticated session. No remotely callable tool
  can configure the session. Missing authority fails through the MCP error path.
* Native CLI session: `qualia-cli llm eval --authority <JSON> <PROMPT>` opens the
  explicitly selected existing protected profile vault, logs in through the
  configured signed admission, loads the selected model and binds the retained
  context in the same process as evaluation. Every `run_eval` consumes a fresh
  validated bind before direct, streaming or orchestrated execution. Scope exit
  logs out and disconnects the controller on success or failure. Reload and
  eviction discard the route. The explicitly diagnostic `llm_testing` path uses
  `NativeDecodeRequest::Developer`; normal evaluation has no developer fallback.

The CLI authority JSON has no default fields and rejects unknown fields. Supply
`profile` (absolute existing native metadata profile), `subject`, `identifier`
(the five fields described above), `signer_label`, `lifetime_seconds`, `model`,
`model_vault`, `preparation`, `retained_input_bytes`,
`tokenizer_scratch_bytes`, `domino_owned_bytes`, `domino_text_bytes` and
`domino_stack_depth`. The file is capped at 64 KiB. Budgets are explicit choices
for the selected model and input; insufficient budgets fail closed. The profile
is bound in a child process through the existing profile helper before state is
initialized; no late environment mutation redirects the configured vault.

Preferred file-backed `preparation` uses `kind: "Leased"`, `primary` and ordered
`shards`. Each source has `path` and `expected_sha256` (32 integer bytes selected
independently of the open operation). Supply aggregate `source_byte_budget`,
`mapped_byte_budget`, `total_byte_budget` and `metadata_budget`. Existing
`ModelLease::open` verifies the full digest and backing while retaining writer
exclusion. Each verified source length and mapping is charged against remaining
aggregate budgets before the next shard opens; the primary and all ordered FTW
shards stay retained through decode. There is no model-byte duplication. This is
an admission ledger, not a measured RSS ceiling; platform-specific lease support
fails explicitly. At most 64 shards are accepted. An explicit alternate
`preparation` with `kind: "Snapshot"` and `byte_budget` selects bounded snapshot
ownership for callers choosing that tradeoff.

Existing `inference-policy --profile <ABSOLUTE_DIRECTORY>` commands explicitly
create the protected vault/key only when requested, show installer public key,
configure supplied trust/admission/document, create a requested actor key and
check actor login. For native decode install the reviewed evidence policy with
`install-native --rules <JSON> --observations <JSON> --expires-at <UNIX_SECONDS>
--expected-generation <GENERATION>`. Observation JSON uses the real
`ObservationRequirements` schema. Legacy `install` remains separate. Evaluation
does not create a vault, actor signer, admission, project or policy.

There is no native desktop command registration or native inference setup UI in
this checkout (`crates/webizen-desktop` contains static resources). Wiring the
privileged setup API into the external embedding's authorized UI remains an
external boundary. CLI shell provisioning uses the existing native protected
vault setup and explicit authority input described above; runtime verification
of that complete path remains pending. No policy, admission, project,
identifier, identity or key is silently generated to hide these missing inputs.
