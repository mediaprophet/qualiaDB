# Non-Goals: Solid Identity Provider Status

qualia-db is not an OIDC/WebID-OIDC provider; it is a relying party (future
work).

`qualia-solid-bridge` is not currently a production OIDC or WebID-OIDC provider.

The bridge may become a Solid-compatible resource server, relying party, and future audited identity bridge for Qualia identifiers. That is different from shipping a mock issuer as if it were a real identity root.

## Current Boundary

- Default builds do not expose `.well-known/openid-configuration`, `/jwks`, `/token`, or the mock WebID profile issuer route.
- The former micro-IdP routes are available only behind the non-default `demo` feature for local demonstrations.
- Demo tokens, demo JWKS, and placeholder WebID issuer metadata are not acceptable provenance for real Solid apps, access decisions, or social-web identity.

## Intended Direction

- Qualia can interoperate with Solid users as a Solid resource bridge and relying party.
- A Qualia identifier can be bound to a Solid WebID through explicit signed profile metadata.
- A future Qualia-backed WebID-OIDC provider must be implemented as an audited identity subsystem with real keys, DPoP-bound tokens, issuer discovery, key rotation, expiry, consent/session handling, and WebID issuer verification.

Until that work exists, production Solid identity should use a real Solid-OIDC issuer and Qualia should validate it rather than minting mock credentials.
