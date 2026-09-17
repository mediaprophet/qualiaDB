# Capt re-UAT — Directory palette · Talk missing-model · Mail daily inbox

**Status:** ready for Capt · **Branch:** `cursor/talk-ux-capt-uat-8021` · **Base:** `0.0.38` tip (`80bbe594`, includes #95–#100)  
**Date:** 2026-09-12  
**Why:** Cold Desktop UAT at `7abb7a6` — PRs merged, three paths still incomplete. No new paper gates.

Locks: human-alone · chatbot is a tool, not a peer · held / not yet (never `NEEDS MODEL` / `Instrument · none`) · Directory is Talk / People (no new top-level IA name) · Talk → Mail is the daily inbox · Poet Domain.info / Inalienable Domain Inboxes is secondary admin.

---

## 1) Directory palette

1. Cold Desktop. **Ctrl+K** (or **Ctrl+P**). Type `dir`. Enter.
2. Expect **Talk → People** with **Directory visible** (`data-talk-open="directory"`, `data-directory-visible="true"`). Hash `/talk/directory`.
3. Repeat with `contacts` and `address book`. Same destination.
4. Alternate: QApps → Directory → Open Directory. Or Relations sidebar → People (Directory already open; Hide is optional).

**PASS if** palette finds Directory in ≤2 keystrokes and People shows the address book without Relations → People → “Open personal directory” as the only path.  
**FAIL if** `dir` / `contacts` / `address book` returns no match, or lands on a blank DynamicPage / new top-level IA name.

## 2) Talk missing-model sayable

1. Cold Talk / Relations → Inbox. No local model active (or host returns `none`).
2. Type a message. Send.
3. Expect: message persists. Chrome says **held / not yet** / `Instrument · held / not yet` with a teachable Settings → AI instruments path.
4. Chatbot stays a **tool**, not the other party. Send does not require an agent.

**PASS if** Send works and there is no `NEEDS MODEL` / `Instrument · none` theatre.  
**FAIL if** those strings appear, or Send is blocked on a model, or the instrument is presented as a peer person.

## 3) Mail daily path

1. Talk (Relations). Sidebar **Mail**. Or Ctrl+K `mail`. Or QApps → Mail → Open Mail.
2. Expect **Talk → Mail** daily inbox (`data-mail-daily="true"`, hash `/talk/mail` or `/mail`): purpose inbox list + landed mail.
3. Receiver down → **held / not yet** + one Start receiver control. Not a broken red page.
4. Poet **Inalienable Domain Inboxes** / **Domain.info** stays under Reception / Poet — secondary admin, not this click.

**PASS if** Talk → Mail is the daily inbox and Domains admin is not the landing.  
**FAIL if** Mail opens Poet Communications / Domain.info soup.

---

## Honest held

- Missing model and stopped receiver stay **held / not yet**. No Host methods invented.
- This is not Gmail / Apple Mail / Contacts parity.
- Neo sole-pushes. Do not merge.
