# Canonical sensing integration

The sensing implementation is available in the canonical QualiaDB checkout:

- `qualia_core_db::sensing` supplies field-bound records, uncertainty/Q42 links,
  generic association/grounding contracts and native custody facilities.
- `crates/qualia-etalon-sensing-bridge` composes Etalon observations with those
  records, optional consent adapters, exact sidecars and custody operations.
- `applications/sensing-inspect` is the standalone bounded inspection and
  signed-release-verification application.

The bridge and inspector intentionally remain standalone Cargo workspaces.
Their Etalon path dependencies use the canonical sibling checkout
`../etalon`; none refers to the removed sensing worktree. The core's production
GPU dependency pins are unchanged by this migration. Opting into a native
vendor adapter requires its own runtime, device and licensing admission.

## Migration evidence

`sensing-canonical-migration-2026-11-01.json` records 109 implementation files and
seven historical plan/review documents copied from the former detached source
tree. Every copy was checked against its source SHA-256, and the source files
were reread afterward to verify that the migration did not modify them.
The inventory predates documented integration edits and is not a release hash
manifest for the resulting implementation.

No unrelated canonical changes were overwritten. The sensing core registration
was added to the existing crate root rather than replacing it with the old
worktree's file. Source history, new qualification results and outstanding
hardware requirements should be recorded separately from the preserved plans.

The integration ownership review admits two cohesive source-size exceptions
after formatting: `sensing/uncertainty.rs` (typed uncertainty contracts and Q42
projection) and `sensing/nfc_evidence.rs` (variable-radio evidence and custody
codec). Their tests remain separate, and neither owns acquisition or deployment
lifecycles. Decompose them before adding substantial unrelated behavior. No
implementation file in this migration reaches the 1,200-line threshold.

Windows local custody head publication can return `Indeterminate` after a
namespace update when durable parent-directory synchronization cannot be
established. This is an explicit recovery state, not a confirmed durable head
commit; hosts must preserve artifacts and reconcile the protected store before
treating publication as successful. Source and unit-test qualification cannot
remove that filesystem-specific limitation.

## Qualification commands

Use one compiler workload at a time on the development host. Direct standalone
builds to the canonical target directory to avoid duplicate dependency trees:

```powershell
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'target')
cargo test --manifest-path crates/qualia-etalon-sensing-bridge/Cargo.toml --jobs 1
cargo test --manifest-path applications/sensing-inspect/Cargo.toml --jobs 1
```

These commands describe the required consumer checks. Their presence is not a
claim that execution passed. Hardware acquisition, external trust-root adoption,
native SDK signing and driver/allocation qualification are distinct from a
successful library/application build.
