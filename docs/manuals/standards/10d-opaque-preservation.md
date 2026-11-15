# Explicit .10d structural opaque retention

Status: developmental SDK contract. Semantic admission and required/optional extension negotiation remain separate.

# FND-04 opaque preservation policy — declared before implementation

This additive canonical Core SDK operation inspects and copies `.10d` v1/v2
bytes structurally. It does not admit unknown payloads to rendering, execution,
governance or authoring. Existing `parse_section_table` and material APIs retain
their strict compatibility behavior. Unsupported outer header grammar/version
refuses. Header flags and descriptor reserved metadata are retained without
interpretation in the explicit opaque inspection/copy path only.

Every descriptor is checked before a view or output is returned: known alignment
tier, bounds, metadata and payload overlaps, unique nonzero tags, stride product,
padding, per-payload and whole-file CRC. Explicit caller byte and section limits
bound processing; Core's existing maximum section count also applies. Unaligned
zero-copy tables refuse with a typed error. Unknown alignment grammar refuses.
Returned views borrow immutable source; safe Rust prevents aliased mutable output.
Exact copy retains all bytes, including unknown tag/payload/metadata, supported
outer version, flags, ordering, offsets and opaque tail bytes.

Known-section byte replacement is deliberately limited to the existing layout
and length. It is structural surgery; payload validity and cross-section semantic
admission remain the caller/typed owner's responsibility. Byte-identical no-ops
may copy any structurally accepted file. A semantic change refuses if any header
flag is unknown, any descriptor has reserved metadata, or any section tag has
unimplemented/unknown semantics. Nonzero uninterpreted tail bytes also refuse a
changed replacement; zero tail padding can remain. Required/optional and dependency negotiation for
unknown sections is unresolved; changing a known payload could invalidate an
unknown dependency even if all offsets stay unchanged. The SDK therefore does
not pretend that a CRC makes that change safe. No section addition, relocation,
version upgrade or flags rewrite is provided. No new format is defined.

Source mutation is never performed. All input checks, refusal decisions, length,
capacity and budget checks finish before the first output write. On success only
target payload, target descriptor CRC and whole-file CRC may change; all other
bytes are retained. This scope is useful retention tooling, not a completed
FND-04 gate, native Poet workflow, model, package or asset acceptance. Assets stay
ON HOLD. Root independently reviews/installs and runs native tests; staged source
and test presence are not executed evidence.

Ownership: `section.rs` module/re-export delegation only; implementation and tests
in focused child/new integration files. `section.rs` is already oversized. This
additive child extraction tracks decomposition without moving or changing the
strict parser/writer. Further parser/writer and legacy test decomposition remains
separate maintenance work; no blanket exemption for substantial new behavior.


# Proposed canonical `.10d` standard delta: explicit opaque retention

Scope: additive Core API, format v1/v2 byte layout unchanged. Retain the strict
`parse_section_table` rendering/typed default. The canonical module path is
`qualia_core_db::container_10d::section`; top-level re-exports are optional routing.

`OpaquePreservationBudget { max_bytes, max_sections }` is explicit per call.
`inspect_opaque_container` produces a private-field immutable borrowed
`OpaqueContainerView` only after canonical header validation, budget checks,
whole-file CRC and the complete raw table traversal. Each nonzero raw tag must
be unique. Unknown tags and nonzero `reserved16` do not acquire semantics. Known
alignment grammar, all bounds, metadata intersections, stride products using
u64 arithmetic, CRCs and pairwise overlaps must validate. Existing interior-zero
padding and uninterpreted-tail behavior is retained. Unknown outer versions or
alignment tiers fail. Unaligned zero-copy table storage returns a typed error.
The current header grammar's nonzero pad0 remains invalid.

`copy_opaque_container_into` copies every input byte exactly, retaining known and
unknown payloads, descriptors, supported outer version, flags, metadata, table
order/offset, and tail. It does not decode, render, execute, authorize or negotiate
required/optional unknown sections. Source is immutable; disjoint safe Rust
borrows prevent source/output aliasing. Output suffix remains unchanged.

`replace_known_section_bytes_preserving_layout` is a structural byte operation,
not semantic authoring admission. The caller's typed owner must validate payload
and known cross-section meaning. It replaces one implemented tag at its existing
offset and exact length. No relocation, section addition or format upgrade is
supported. It refuses changed payload bytes if any unknown header flag, nonzero
descriptor reserved metadata, unimplemented/unknown tag or nonzero uninterpreted
tail is present. Required/optional extension and dependency negotiation remains
unresolved. Identical no-ops remain exact copies even with retained unknowns.
Successful changes update only target bytes, target descriptor CRC and whole-file
CRC. All refusals/validation/capacity failures precede output mutation.

Caller budgets bound bytes and section count; header MAX_SECTION_COUNT remains
an independent ceiling. The structural overlap/duplicate validation is quadratic
in the admitted section count; CRC traversals are bounded by admitted byte input.
No scheduler/cancellation API or wall-clock guarantee is added. Existing Core
headers/descriptors use POD/native layout with little-endian wire expectations;
this patch does not qualify big-endian hosts or WASM. Header metric-divergence
errors inherit the existing allocated String; no universal zero-allocation claim
is made. The staged thread-local allocator test measures only its success and
policy-refusal fixtures when root executes it.

This delta does not make existing `attach_material_section` retain unknown data:
its strict read/re-encode behavior is unchanged. Its future extension needs
explicit dependency/requiredness policy and lossless layout handling. FND-01,
FND-03/FND-04 full qualification, FND-05/FND-06, native Poet contributor/reviewer
work, HMC/Q42 meaning/provenance, model completeness and human acceptance remain
open. No Ecopunk asset is generated or changed; assets remain ON HOLD.

Maintainability: `section.rs` gains module/re-export delegation only. New behavior
is isolated in `section/opaque_preservation.rs`; test fixtures/allocator live in
`tests/opaque_preservation_support/mod.rs`. Further strict parser/writer and old
test extraction from the oversized section module remains tracked maintenance,
not a claim completed by this small delegation.
