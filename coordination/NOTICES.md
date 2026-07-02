# QualiaDB — multi-agent coordination feed

**Canonical repo:** `C:\Projects\qualia-27062026`

All AI instruments edit this tree only. Do not use git worktrees or vendor-specific clone paths.

## Notice format

One line per event:

```
YYYY-MM-DD | INSTRUMENT | CLAIM|PROGRESS|BLOCKED|RELEASE | short description | paths (optional)
```

## Active notices

2026-07-02 | Claude (Opus 4.8) | PROGRESS | Retired plaintext add_sanctuary_note journal path — sanctuary free-text notes now live ONLY in the encrypted vault. Removed host method/types/Tauri cmd/bridge/panel section; tests repointed to therapy_note fixture. wellfare-core 76 + wellfair 78 tests; desktop + studio host + wasm green | wellfair/{api,sanctuary}.rs, webizen-desktop/commands, sanctuary_panel.rs, host_client.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Sanctuary ENCRYPTED VAULT: real at-rest boundary (PBKDF2 310k + AES-256-GCM via core-db sanctuary_crypto) with independent decoy lane; PIN never stored (verifier), no-plaintext-on-disk test passes. Host API + 4 Tauri cmds + Encrypted-vault panel section. wellfair 78 tests; desktop + studio host + studio wasm green | wellfair/sanctuary_vault.rs, wellfair/api.rs, qualia-client-core/Cargo.toml, webizen-desktop/commands, sanctuary_panel.rs, host_client.rs, shell.rs
2026-07-02 | Claude (Opus 4.8) | RELEASE | UI-wired finance/projects/credentials: 6 Tauri commands + host_client bridges + 3 Studio panels + shell nav. Desktop + studio(host) check green; studio wasm LIB green (all wellfair wasm code compiles). Fixed pre-existing wasm borrow-after-move in render/spatial_bridge.rs (out-of-lane, flagged) | webizen-desktop/commands/mod.rs, webizen-studio/components/wellfair/{finance,projects,credentials}_panel.rs, shell.rs, mod.rs, host_client.rs, render/spatial_bridge.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Clinical + Welfare + Sync-inbox wired end-to-end: host API + 5 Tauri commands + host_client bridges + 3 Studio panels (clinical/welfare/sync) + shell nav (new Clinical area, welfare in Life, sync in Tools). wellfair 71 tests; desktop + studio host + studio wasm all green | wellfair/api.rs, wellfair/policy.rs, webizen-desktop/commands, webizen-studio/components/wellfair/{clinical,welfare,sync}_panel.rs, host_client.rs, mod.rs, shell.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Phase 5 sync-operation protocol (sync_protocol.rs): versioned content-hashed ops, fail-closed quarantined inbox, idempotent replay, add-wins convergence, 2-node partition test; host build/admit API (real ed25519 sig, Classified excluded). +clinical.rs (CLI) +welfare_support.rs (LIF) swarm-authored. wellfare-core 76 + wellfair 69 tests pass; desktop + studio-wasm green | wellfair/sync_protocol.rs, wellfair/api.rs, wellfare-core/{clinical,welfare_support}.rs, conditions.rs
2026-07-02 | Claude (Opus 4.8) | RESOLVED | Studio wasm APP build now GREEN. Fixed pre-existing wasm-only mutable-borrow errors (signal .set() before `let mut` shadow) in benchmark_harness.rs (x2) + physics_simulator.rs + the spatial_bridge borrow-after-move. Whole desktop UI incl. WellFair panels now builds to wasm | webizen-studio/components/{benchmark_harness,physics_simulator}.rs, render/spatial_bridge.rs
2026-07-02 | Claude (Opus 4.8) | CLAIM | UI-wire finance/projects/credentials: Tauri commands + generate_handler regs + new Studio panels (finance/projects/credentials) + shell nav. Timothy-authorized; touches shared commands/mod.rs + studio shell — NOT touching live_share/companion_gateway (Grok's Phase-4b) | webizen-desktop/commands/mod.rs, webizen-studio/components/wellfair/{finance,projects,credentials}_panel.rs, shell.rs, mod.rs, host_client.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Phase 5 Projects + Phase 3 Credentials (swarm-authored, self-integrated): projects.rs (contributions→derived obligations, add-wins) + credentials.rs (status cache + field-selection presentation, honestly not ZK); host API + policy + journal kinds; wellfare-core 59 + wellfair 55 tests pass; desktop check green. UI+commands next | wellfare-core/{projects,credentials}.rs, wellfair/api.rs, wellfair/policy.rs, conditions.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Phase 5 slice: Personal Finance ledger core — add-wins/replay-safe merge + derived per-currency balance (§17); wellfare-core/finance.rs + host API + policy writer + journal kind; 5 finance + 53 wellfair tests pass; desktop check green. UI+command wiring next | wellfare-core/finance.rs, wellfair/api.rs, wellfair/policy.rs, conditions.rs
2026-07-02 | Claude (Opus 4.8) | PROGRESS | Reallocated by Timothy to continue WellFair. Foundation hardening: fixed decide_live_share_request desktop call-site break (threaded deny_reason); Sanctuary/Classified records now excluded from ordinary sync outbox and graph-coverage query (§5.2/§17); +2 tests → 52 wellfair pass; desktop+studio check green | wellfair/{vault,api}, webizen-desktop/commands
2026-07-02 | Grok | CLAIM | Phase 4b: push LIVE_SECTION_DECISION to companion WS + mobile harness request UI | companion_gateway, mobile-harness, live_share
2026-07-02 | Grok | RELEASE | Phase 4 swarm: live share consent, Communications tab, companion WS, 3 phase4 + 4 live_share tests | live_share, communications_panel, companion_gateway, WELLFAIR_PHASE4_SPRINT.md
2026-07-02 | Grok | CLAIM | Phase 4 swarm: live share + usage agreement + communications UI (orchestrator integrates commands) | wellfair/live_share, companion_gateway, communications_panel, phase4_tests
2026-07-02 | Grok | RELEASE | Phase 3 closeout: case_task slice, 3 phase3 tests, PHASE3 doc; studio+desktop green | life_panel, host_client, commands, phase3_tests, WELLFAIR_PHASE3_CLOSEOUT.md
2026-07-02 | Grok | CLAIM | Phase 3 closeout: case_task vertical slice + closeout doc | life_panel, host_client, commands/mod.rs, phase3_tests
2026-07-02 | Grok | RELEASE | Phase 3: life/sanctuary/wellbeing panels + missing modules; 43 wellfair + 2 phase3 tests; studio+desktop check green | wellfair/, wellfare-core/, webizen-studio/wellfair/
2026-07-02 | Grok | CLAIM | Phase 3 recovery: host_client glue, sanctuary/life/wellbeing panels, shell wiring | wellfair/, wellfare-core/, webizen-desktop commands
2026-07-02 | Grok | RELEASE | Phase 2 closeout: OS med notifications, companion E2E, audit/graph panel; 38 wellfair tests; PHASE2 doc | med_reminder_notifier, companion_tests, audit_panel, WELLFAIR_PHASE2_CLOSEOUT.md
2026-07-02 | Grok | RELEASE | Social Book write path + sharing preview; lib.rs fixes companion_gateway RA; 36 wellfair tests pass | webizen-desktop/lib.rs, social_book_panel, host_client
2026-07-02 | Grok | RELEASE | Q2+Q6: disputed diagnosis, housing/safety, med reminder prefs+due slots; 36 wellfair tests pass | personal_records, med_reminders, personal_panel, medication_panel
2026-07-02 | Grok | RELEASE | §8.1 exit sprint: journey test, Turtle export package, graph coverage query; 31 wellfair tests pass | wellfair/{journey_tests,export_package,graph_query}, tools_panel, webizen-desktop
2026-07-02 | Grok | RELEASE | Parallel sprint landed: sync outbox, replay idempotency, conditions/allergies UI+API; 27 wellfair + 3 conditions tests pass | wellfair/{sync_outbox,replay_tests,api,vault}, wellfare-core/conditions.rs, personal_panel, webizen-desktop commands
2026-07-02 | Grok | RELEASE | Phase 2 Q5+Q6+Q2: medication/nutrition panel, sleep debt+heatmap, emergency contacts; 18 wellfair tests pass | wellfair/, webizen-studio/wellfair/, webizen-desktop commands
2026-07-02 | Grok | RELEASE | Chemistry/crypto stash on 0.0.24 (4256c731): SCF+DIIS, integral engine, OS keyring vault lock/unlock+CSPRNG; 52 chem + 5 vault + 14 wellfair tests pass | crates/qualia-core-db/src/{identity/key_vault,specialized_libs/chemistry_modeling}
2026-07-02 | Grok | RELEASE | Merged grok desktop clone → canonical 0.0.24; pushed origin/0.0.24 (f0265dfe + ec84d512); 14 wellfair tests pass | C:\Projects\qualia-27062026
2026-07-02 | Grok | RELEASE | Phase 2 Consent panel + sleep dashboard + policy evaluate/grant/revoke API; journal summary projections | wellfair/{consent_store,policy,api,journal,import_samsung}, consent_panel, sleep_panel, webizen-desktop commands
2026-07-02 | Grok | PROGRESS | WS1 checkpoint: dag.bin+meta.json+vault.q42 graph quins persist; Social Book panel; mobile outbox restore | wellfair/{checkpoint_store,graph_store,vault}, social_book_panel
2026-07-02 | Grok | PROGRESS | Phase 1 complete: journal+receipts jsonl, WAL checkpoint, Health tab UI, policy receipts panel | wellfair/{journal,receipt,vault,api}, webizen-studio/health_panel
2026-07-01 | Grok | PROGRESS | Phase 1 WellFair: live host snapshot, PolicyDecisionService, Samsung CSV→WAL import, Tools UI panel | wellfair/{api,import_samsung,snapshot,policy}, webizen-desktop commands
2026-07-01 | Grok | RELEASE | WS2+3 committed on 0.0.24; legacy qualia_bindings/webizen retained in commit (WS1 retires separately) | feat(wellfair) qapp_install+companion_bundle+shell
2026-07-01 | Grok | CLAIM | Full floor after Gemini stand-down: WS2+3 commit + frontend compile verification; auditing wellfare-core deletions against WS1 plan | canonical 0.0.24
2026-07-01 | Grok | PROGRESS | Build green: webizen-studio + webizen-desktop check 1m03s; only 2 deletions in tree (qualia_bindings, webizen) — match WS1 retire list; record.rs added | wellfare-core, docs/plans/wellfair-webizen-desktop/
2026-07-01 | Grok | PROGRESS | WS2+3 verified: qapp_install 5/5, companion_bundle 1/1, webizen-studio check green; fixed qapp_version test typos + bundled_qapps dead code | qualia-client-core, webizen-studio
2026-07-01 | Grok | PROGRESS | M1 qapp_install + M2 companion_bundle skeleton + WellFair shell at /wellfair | qapp_install.rs, companion_bundle/, webizen-studio/components/wellfair/
2026-07-01 | Grok | CLAIM | WellFair 0.0.24 Workstreams 2+3: shell, shared UI, qApp install authority, companion bundle builder | crates/webizen-studio/src/components/wellfair/, crates/qualia-client-core/src/qapp_*.rs, scripts/wellfair-companion-*
2026-07-01 | Antigravity | (assumed) | Workstream 1: VaultService, PolicyService, WebizenHostApi v1, wellfare-core refactor | crates/wellfare-core/, crates/qualia-client-core/src/wellfair/
2026-07-01 | Grok | RELEASE | Studio 0.0.23 work consolidated into canonical tree; worktree instructions removed from CLAUDE.md