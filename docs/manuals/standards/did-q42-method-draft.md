# `did:q42` Method Draft

**Status:** Internal draft  
**Date:** 2026-06-08  
**Purpose:** Define the current implementation reality and intended
standardization path for the `did:q42` method within the Qualia Protocol
Ecosystem.

## 1. Role

`did:q42` is currently used inside QualiaDB as a compact method-specific
identifier that marks a topological pointer or pointer-derived identifier token
inside Quin execution paths.

At present, the method is:

- a machine-facing identifier form
- a routing and dispatch signal inside the engine
- a bridge between textual RDF input and a 64-bit encoded Quin value

It is not yet:

- a fully specified DID method with network resolution semantics
- a stable DID Document production format
- a definition of human identity outside Qualia environments

## 2. Current Implementation Reality

The current codebase gives `did:q42` a very specific meaning:

1. `identifier.rs` parses any non-empty `did:q42:` payload.
2. The payload bytes after the prefix are hashed with FNV-1a.
3. Bit 63 is set on the resulting `u64`.
4. The engine uses that MSB to distinguish the value from ordinary `q_hash`
   tokens.

This means the current method behavior is closer to a compact topological or
dispatch pointer than to a conventional DID resolver ecosystem.

Relevant implementation anchors:

- [identifier.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/identifier.rs:1)
- [mini_parser.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/mini_parser.rs:97)
- [resolver.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/resolver.rs:134)
- [webizen_bytecode.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/webizen_bytecode.rs:15)

## 3. Method Name

The method name is:

`q42`

The full DID prefix is:

`did:q42:`

## 4. Current Syntax Contract

The current parser only enforces two rules:

1. the identifier starts with `did:q42:`
2. the payload after that prefix is non-empty

So the current accepted surface is effectively:

```text
did:q42:<method-specific-id>
```

Where `<method-specific-id>` is currently any non-empty byte sequence accepted
by the parser input path.

Important note:

- the comments in `identifier.rs` mention base-58 or multibase expectations
- the implementation does not currently validate base-58, multibase, or a
  checksum

This draft treats the implementation, not the comment, as the authoritative
starting point.

## 5. Canonical Internal Encoding

The current internal encoding rule is:

```text
encoded_pointer = fnv1a(method_specific_id_bytes) | (1 << 63)
```

Properties of this rule:

- deterministic for the same payload bytes
- zero-allocation in the hot path
- compact enough for direct Quin field embedding
- suited to MSB-based dispatch in the bytecode VM

This is the current canonical internal behavior for v0 of the method.

## 6. Quin Semantics

Within the engine, `did:q42` currently occupies a special role in Quin fields.

### Subject and Predicate Context

When a `did:q42` token is compiled from N-Triples-style input through
`mini_parser.rs`, the resulting `u64` carries bit 63 set. This lets the
runtime distinguish it from ordinary hash-based identifiers during direct
matching.

### Object Context

In `resolver.rs`, an object value with MSB set and no lexicon match is rendered
as a `did:q42` pointer surface:

```text
<did:q42:ptr/{hex}>
```

That output form is currently a resolver rendering convention. It should not
yet be treated as the canonical external method syntax.

## 7. Resolution Model

The current method does not yet define full DID resolution in the W3C sense.

Instead, current resolution behavior is limited to:

1. parsing a textual `did:q42:` token
2. converting it into a 64-bit pointer-marked value
3. allowing VM and resolver components to treat that value specially

So the current implementation has:

- parsing behavior
- normalization by byte-preserving hashing
- internal dispatch semantics

It does not yet have:

- DID URL dereferencing rules
- DID Document retrieval rules
- representation negotiation
- verification method conventions
- service endpoint conventions

## 8. Relationship To Human-Centric Agency

This method draft should not overclaim that every current `did:q42` value is a
human identity in the richer enumerated sense.

At the moment, `did:q42` serves two overlapping but distinct roles:

- pointer-oriented internal addressing
- identifier-context anchoring in some higher-level Qualia and Webizen flows

For standards work, those roles must be separated carefully so that:

- human identity is not collapsed into raw storage offsets, nyms, auth
  material, or credential artifacts
- method syntax remains technically precise
- future DID Document semantics can describe people, agreements, groups, or
  vault contexts without pretending that a DID by itself exhausts human
  identity

Put differently:

- `did:q42` is an identifier form
- authentication and authorization remain separate concerns
- verifiable claims and verifiable credentials are attestation artifacts
- none of those artifacts should be used as the definition of human identity

## 9. Initial Conformance Targets

The first internal draft should define conformance targets separately:

### `did:q42` Parser

Must:

- accept `did:q42:` prefixes
- reject missing prefix
- reject empty payload
- produce deterministic 64-bit output
- set bit 63 on the encoded value

### `did:q42` Compiler Integration

Must:

- route `did:q42:` tokens through `parse_did_q42`
- preserve special handling in subject, predicate, and object positions

### `did:q42` Renderer

Must:

- distinguish lexicon hits from pointer-marked values
- render non-lexicon MSB-marked values consistently

## 10. Security and Collision Notes

The current implementation uses FNV-1a over the method-specific identifier
payload. That is simple and fast, but it raises several issues that must be
named explicitly:

1. FNV-1a is not collision-resistant.
2. The current method is compact, but not cryptographically self-certifying.
3. Bit 63 marking is a dispatch convention, not a proof of identity.
4. The same compact encoding may be useful for runtime routing while being
   insufficient for external identity assurances.

This means any future external `did:q42` method submission will need a clearer
story for:

- collision tolerance
- verification methods
- DID Document integrity
- privacy and correlation risks

## 11. Contradictions To Resolve Before Externalization

The codebase is not yet ready to submit `did:q42` externally without narrowing
or refining the method.

Key contradictions or unfinished areas:

1. The prose sometimes describes `did:q42` as a physical topological pointer,
   while higher-level flows use it more like an identifier root for human,
   group, agreement, or vault contexts.
2. The parser comments mention base-58 or multibase, but no such validation is
   currently enforced.
3. The resolver emits `did:q42:ptr/{hex}` as a display surface, but the parser
   accepts `did:q42:<payload>` with no `ptr/` grammar.
4. There is no frozen DID Document representation.
5. There is no method-specific resolution algorithm in the full DID sense.

## 12. Proposed Direction

This draft recommends the following sequence:

1. Keep the current compact MSB-marked encoding as the internal execution form.
2. Freeze a narrow textual syntax for the method-specific identifier.
3. Decide whether `did:q42` is:
   - primarily a pointer method
   - primarily an identifier method
   - or a layered method where pointer tokens are one representation profile
4. Define a minimal DID Document model only after those roles are separated.
5. Add privacy and security considerations before any W3C-facing submission.

## 13. Open Questions

1. Should the method-specific identifier require multibase explicitly, or
   should it remain more general?
2. Should `did:q42:ptr/{hex}` become a formal DID URL path form, or remain only
   a renderer convenience?
3. Should human-facing identifier roots and topological pointer forms use the
   same method syntax, or related but distinct profiles?
4. What should a minimal DID Document for `did:q42` contain in v1?
5. How should agreement, group, vault, and personal identity contexts differ
   under this method?

## 14. Immediate Next Steps

1. Freeze the intended method-specific identifier grammar.
2. Decide whether current parser comments should be aligned to multibase or
   removed from the draft.
3. Define whether `did:q42` is one method with multiple profiles or whether the
   pointer form should be separated from richer identity forms.
4. Draft a minimal DID Document example only after the above is settled.
