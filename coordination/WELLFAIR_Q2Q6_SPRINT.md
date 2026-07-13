# WellFair Q2 + Q6 Sprint — Sub-agent orchestration (2026-07-02)

**Canonical repo:** `C:\Projects\qualia-27062026` | **Branch:** `0.0.24`  
**Epic:** Phase 2 Personal Core remainder (Q2) + medication reminders (Q6)

## Lanes

| Lane | Scope | Owns | Gate |
|------|-------|------|------|
| **A** | Disputed diagnosis records | `wellfare-core/personal_records.rs`, `api.rs`, journal kinds | Disputed epistemic + journal kind `disputed_diagnosis` |
| **B** | Housing/safety self-report | `personal_records.rs`, `personal_panel.rs`, Tauri | Journal kind `housing_safety`, Restricted sensitivity |
| **C** | Med reminder permission + due slots | `med_reminders.rs`, `medication_panel.rs`, Tauri | Prefs round-trip + due reminder from schedule |

## Verification

```powershell
cd C:\Projects\qualia-27062026
cargo test -p qualia-client-core wellfair --lib
cargo test -p wellfare-core personal_records med_reminders --lib
cargo check -p webizen-studio -p webizen-desktop
```