# Non-Goals: Solid Identity Provider Status

qualia-db is not a production OIDC / Solid-OIDC provider; it is a **relying party**
and **resource server** path (personal pod + consumer agent), with an optional
**demo** IdP for local smoke tests only.

`qualia-solid-bridge` is not currently a production OIDC or WebID-OIDC provider.

The bridge may become a Solid-compatible resource server, relying party, and future
audited identity bridge for Qualia identifiers. That is different from shipping a
mock issuer as if it were a real identity root.

## Current Boundary

- **Default library config** (`BridgeConfig::default()`): demo OIDC **off** unless
  `QUALIA_SOLID_DEMO_OIDC=1` or compile feature `demo`.
- **`qualia-cli solid serve`**: demo OIDC **on by default** for hackathon/local
  personal-pod smoke; pass `--no-demo-oidc` to disable.
- Demo routes (when enabled): `.well-known/openid-configuration`, `/jwks`,
  `/token`, `/authorize` (auto-approve), `/register`, plus WebID profile material.
- Demo tokens, demo JWKS, and placeholder WebID issuer metadata are **not**
  acceptable provenance for real Solid apps, access decisions, or social-web identity.

## Intended Direction

- Qualia interops with Solid users as a **Solid resource bridge** and **relying party**.
- A Qualia identifier can be bound to a Solid WebID through explicit signed profile metadata.
- A future Qualia-backed WebID-OIDC provider must be an audited identity subsystem with
  real keys, DPoP-bound tokens, issuer discovery, key rotation, expiry, consent/session
  handling, and WebID issuer verification.

Until that work exists, production Solid identity should use a real Solid-OIDC issuer
(e.g. Community Solid Server) and Qualia should **validate** it rather than minting
mock credentials.

## Hackathon note (Solid-OutPost)

Local demo OIDC exists so Webizen can act as a **personal pod** and Solid clients can
complete discovery against `http://127.0.0.1:4243` without standing up CSS first.
Institutional egress in the Solid 2026 proposal still targets CSS + Entra bridging —
not this mock IdP.
