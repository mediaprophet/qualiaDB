# QDNF Permissive Commons and Resource Economics

**Status:** Normative design 0.1; proposed service profile, not implemented conformance
**Date:** 2026-09-05
**Scope:** Resource accounting, socioeconomic agreements, contributions, and optional micropayments

## 1. Purpose

Every network operation uses physical resources over time. A resource may be free to its recipient
while its creation, hosting, transmission, maintenance, or care is funded by somebody else. QDNF
therefore treats **energy and time as baseline accounting dimensions**, including for donated and
community-funded services. Their measurement does not itself create a debt or determine social value.

Permissive commons are resources stewarded under explicit permissions and obligations. They may be
public, community-scoped, or bilateral. Participation can involve money, useful work, reciprocal
service, pooled funding, or a gift. Micropayments provide one settlement mechanism within this model.

This profile separates four questions:

| Record | Question answered | What it does not establish |
|---|---|---|
| Resource account | What energy, time, and other resources were used, by whom, and with what evidence? | A price, a debt, or the value of a person or work |
| Commons agreement | Who may do what, under which permissions, funding and contribution rules? | Permission outside the named purpose/resource/context |
| Obligation account | What accepted contribution remains due or has been satisfied? | That a transfer has settled |
| Settlement receipt | What monetary or nonmonetary contribution was accepted, and under whose authority? | Access rights beyond the agreement or truth of a meter reading |

Energy and time remain independently inspectable. An economic profile MUST NOT silently reduce
them to one universal currency, trust score, or claim about human worth. Qualitative outcomes,
accessibility, cultural obligations, stewardship, and ecological effects can remain separate criteria.

## 2. Architectural boundary

Q0/QLink observe local resources where possible. QRoute uses coarse, scoped cost hints for route
selection. QPolicy checks agreements and budgets. An optional QSession service exchanges offers,
receipts, and settlement instructions. QSync may replicate authorized accounting records.

```text
Resource observations ──> resource account ──> agreed valuation, if any
                                                │
Commons agreement ──QPolicy──> bounded service ──> usage/contribution receipt
       │                                            │
       └── permission and funding rules ─────────────┤
                                                    v
                                          obligation reconciliation
                                                    │
                                   optional settlement adapter
```

The service URI is `qdnf:service:economics`; its feature profile is
`qdnf:feature:commons-economics`, major version 1. These are draft semantic identifiers. The profile
requires frozen deterministic-CBOR schemas, signature contexts, and executable vectors before any
implementation advertises interoperable support; see [Implementation and Conformance](./implementation-conformance.md).
Agreements and their economic records MUST use [Ontologically Defined Contracts over CBOR-LD](./ontological-contracts.md).
That profile binds the context, ontology, compression mappings, SHACL shapes, and N3 rule versions
that give quantities and obligations their meaning.

QDNF Core and Native Independent conformance MUST NOT require a wallet, token, blockchain, payment
provider, proof of wasted energy, or remote price feed. A node can operate entirely through local
agreements and donated or pooled resources. A paid service advertises its additional dependencies.

Routers MUST NOT contact a settlement provider or verify a fresh payment for every forwarded packet.
Cold policy processing installs a bounded forwarding allowance; hot processing uses counters and
verified handles. Payment keys, invoices, and human contribution records stay out of public routing
advertisements and packet headers.

## 3. Energy and time units

The physical accounting reference units are **joules (J)** and **seconds (s)**. In SI, the second is
a base unit and the joule is a derived unit; “baseline” here describes their role in QDNF accounting.
Power is an energy rate: `1 W = 1 J/s`. These units follow the
[BIPM International System of Units](https://www.bipm.org/en/measurement-units).

| Quantity | Reference unit | Required scope |
|---|---|---|
| Energy used | J | Device, component, bearer, or attributed share; measurement boundary and interval |
| Elapsed duration | s | One operation's monotonic start/end on the named clock/boot |
| Device service time | s | Named device or allocated capacity; concurrency and utilization basis |
| Radio airtime | s | Channel/bearer, transmission/reception basis, and retry treatment |
| Human contribution time | s | Voluntarily declared activity, contributor's contextual identifier, acceptance rule |
| Availability or reservation | s | Capacity reserved and interval, distinct from active use |

An operation MUST have explicit energy and elapsed-time accounting states when this profile applies.
Each state is `measured`, `estimated`, or `unknown`; no sensor is required to participate. A numeric
zero means an evidenced zero at the declared resolution. Missing telemetry MUST NOT be encoded as
zero. Additional quantities may be `not-applicable` only with a reason.

Latency, elapsed time, device service time, and human work MUST have different quantity identifiers
even though all can be expressed in seconds. Two devices working concurrently for six seconds can
account for twelve device-seconds and six elapsed seconds. Lamport order is never a duration; wall
timestamps and monetary interest periods are not substitutes for a local monotonic timer.

### 3.1 Quantity representation

Each quantity carries a full semantic URI, unit URI, unsigned 64-bit coefficient, decimal scale in
`[-9, 9]`, evidence state, scope digest, interval/clock reference, and measurement/model reference.
Its value is `coefficient × 10^scale` in the declared unit. Numeric fields are absent for `unknown`.
Uncertainty or lower/upper bounds use the same unit and scope; the method states what they mean.
Quantity URIs and unit URIs are bounded to 255 UTF-8 bytes and compared in full before hash dispatch.

Amounts and rates use checked integer/rational arithmetic. Overflow, underflow, invalid scales,
incompatible dimensions, and division by zero MUST reject the operation; saturation is unsuitable
for billing. Rounding is explicit in the agreement. Fractional settlement remainders carry forward
within the account so dividing work into smaller messages cannot multiply rounding charges.

Monetary amounts additionally require asset identifier, issuer or settlement namespace, and asset
scale. A local work credit is a scoped agreement-backed claim; it is not automatically convertible
to currency, joules, or somebody else's time credit. Conversion requires separately accepted terms.

### 3.2 Measurement and allocation

Energy is a meter delta or an integration of power over an interval. An estimate such as
`energy_J = average_power_W × duration_s` MUST name its model and assumptions. A device's rated
maximum power alone is not measured consumption. Claimed precision must follow the evidence.

An allocation policy MUST state whether it includes idle power, cooling, networking, retries,
storage, or amortized equipment/creation costs. Shared device/facility readings require an explicit
allocation method; mutually exclusive shares MUST NOT exceed the parent total. Parent totals and
their attributed children must not both be summed into a charge. Unknown components remain visible.

Transport resource consumption may include retransmissions; delivered-content accounting deduplicates
the accepted content/range. Failed work consumes resources but is billable only under the accepted
failure/cancellation terms. Cache hits record the resources actually used. Avoided computation may
be reported as a counterfactual estimate, never as energy actually consumed or an automatic debt.

A signature authenticates an assertion's issuer and bytes. It does not prove the sensor, allocation,
work quality, or claimed contribution is correct. Meter authority and evidence acceptance are
agreement-specific, and disputed evidence remains available to the authorized review process.

### 3.3 Budgets and admission

Resource limits and payment limits are separate. A participant may set an energy limit, elapsed-time
deadline, device-time/airtime allowance, monetary ceiling, and maximum unsettled exposure together.
Accepting a price never increases a physical budget or battery reserve.

Before each bounded work quantum, reserve its conservative resource and payment allowance, including
measurement lag and already admitted work. Reservations shared by concurrent sessions/providers MUST
be atomic against the same budget. Release unused reservations on completion or cancellation.
Pause new work before the remaining allowance is insufficient; closure and reconciliation have a
separately reserved small budget. A hard energy guarantee requires a defensible upper bound and
enforcement capability. If unavailable, reject that guarantee or offer an explicitly accepted
estimate-based limit. Unknown telemetry cannot satisfy a verified-energy requirement.

## 4. Commons agreements and socioeconomic choices

A signed, versioned agreement identifies the resource and rights authority, participants or eligible
community, purpose, permitted actions, sensitivity, contribution terms, funding source, validity,
withdrawal/review procedure, and policy digest. It records who can amend terms and resolve disputes.
Agreements bind full identifiers and strong digests; metadata flags are only compiled policy handles.

| Funding/contribution mode | Accounting treatment |
|---|---|
| Gift or volunteered service | Record accepted contribution and resource use; recipient owes nothing |
| Community-funded entitlement | Reserve against an authorized pool, with a named eligibility and exhaustion policy |
| Reciprocity or time credit | Record agreed useful work, capacity, or service; accept through the named verifier |
| Cost sharing | Allocate accepted costs using a published rule; disclose overhead and subsidy |
| Paid service / micropayment | Apply an accepted quote and settlement profile within spending limits |
| Threshold-funded release | Count eligible accepted contributions toward an agreed release threshold |

Several modes may fund one operation, but the same contribution MUST NOT discharge two obligations
without explicit allocation. Eligible participants, beneficiary shares, subsidy rules, and amendment
authority are visible to affected parties without publishing private membership or payment history.

Human work is voluntarily attested and accepted for the stated purpose. Device uptime is not human
labor, and continuous surveillance is not required to verify a contribution. Time banking may adopt
equal-hour credits; specialist services may adopt different rates. QDNF imposes neither valuation.
Useful care, translation, maintenance, and cultural stewardship can be recognized without inventing
a market price or requiring artificial computational work.

Community profiles SHOULD define a funded baseline access allowance, capacity limits, and a response
to exhausted funds. Contributors are not required to supply unlimited unpaid resources. Admission
and fair-use rules may use scoped credentials, invitations, or shared quotas; creating extra DIDs
must not automatically mint extra entitlements. They MUST NOT require a universal person identifier
or equate inability to pay with untrustworthiness.

### 4.1 Access and obligations

The admission sequence for a governed operation is:

```text
identity/capability/consent and sensitivity checks
  -> bounded offer/terms exchange
  -> accepted funding and reserved budgets
  -> agreement obligation check
  -> scoped application delivery
```

Block, revocation, and non-derogable policy checks retain precedence throughout. A valid payment is
evidence toward a named obligation only. It cannot purchase consent, lift a block, change a private
resource to public, or authorize another purpose. An accepted work or subsidy receipt can satisfy
the same obligation where the agreement permits it. Read authority never implies spending authority.

The commons lane does not mean publicly downloadable plaintext; the bilateral lane does not mean
payment is always required. Sensitivity and publication gates still apply. Once plaintext has been
delivered, local gates cannot guarantee control of every recipient's copies or later use.

## 5. Valuation and threshold licensing

Physical consumption, production cost, use value, compensation, and price are related by agreed
policy rather than physical identity. Higher energy use or longer task duration does not imply
higher social value. Fixed outcome prices can reward efficiency; metered prices require explicit
ceilings and acceptance of the measurement method.

An illustrative valuation, only when all terms share the same agreed settlement unit, is:

```text
gross = fixed_fee + energy_J × rate_per_J + device_s × rate_per_device_s
        + accepted_human_s × rate_per_human_s + other_agreed_charges
recipient_due = max(0, gross - allocated_subsidy - accepted_credit)
```

Rates, allowed charge categories, beneficiary allocation, rounding, fee treatment, and any markup
must be agreed before work. An included energy cost cannot be charged again through an inclusive
device-time tariff. Funding above the charge follows an explicit refund/carry-forward rule. Neither
exchange rates nor compounding obligations are inferred from elapsed time.

### 5.1 Threshold-funded release

The [existing Permissive Commons manual](../../webizen_permissive_commons.md) describes a Threshold
Shift License. For QDNF this becomes an explicit agreement, with:

- immutable resource/version and rights-authority references;
- initial obligations, beneficiaries, eligible contribution types, and target unit;
- any accepted risk adjustment, rate, compounding schedule, and absolute ceiling;
- a deduplicated contribution ledger, including refunds, disputes, and unsettled amounts;
- a release predicate, decision authority/quorum, and successor permission/licence digest; and
- terms for surplus contributions and post-release hosting or maintenance costs.

Only contributions accepted under the named finality policy count toward release. Incompatible
assets or time credits require agreed conversion before aggregation. A threshold MUST NOT increase
automatically because the provider changes its rate or an old quote expires.

A release is a signed policy transition. It changes only the named economic/licensing restriction;
privacy, consent, cultural/community conditions, and other rights remain scoped. “Post-threshold”
need not mean universal public access. If irrevocable release was promised, later costs or disputes
cannot silently reimpose the retired charge; they require the separately agreed remedy or a new
voluntary service agreement. Creation funding and ongoing delivery costs have distinct accounts.

The existing risk-compounded formula is one possible valuation policy, not a QDNF universal law.
Threshold execution, rights verification, and adapter-backed settlement require implementation
evidence before being described as shipped features.

## 6. Micropayment and contribution lifecycle

### 6.1 Offer and acceptance

A quote binds provider/payee authority, requester or sponsor, resource/context/purpose, operation ID,
agreement version, funding mode, measurement policy, exact rates or fixed amount, settlement unit,
all fees, expiry/time confidence, limits, delivery criteria, and cancellation/refund/dispute terms.
It identifies permitted payment adapters and any third-party dependence.

The payer accepts the exact quote digest using a spend capability scoped to payee, asset, amount,
purpose, validity, and maximum aggregate exposure. A standing grant may authorize small operations
within those limits; a human need not approve every packet. A revised quote requires new acceptance
unless its changes are already explicitly permitted by that grant. No automatic balance top-up.

Offers, consent checks, and safe refusal messages use a bounded negotiation allowance. Opening a
QSession or discovering a route MUST NOT authorize a debit. Protect negotiation with quotas rather
than a circular requirement to pay before discovering the payment terms.

### 6.2 Service and accounting records

The profile defines these ontology-backed CBOR-LD records. Their term mappings and bounded wrapper
schemas await the P9 bundle/schema/vector freeze:

| Record | Required bindings and evidence |
|---|---|
| CommonsAgreement | Terms and authority from §4, predecessor/version, review and threshold rules |
| ResourceUsage | Operation/segment, subject device or contextual contributor, quantities from §3, measurement policy |
| ServiceQuote | Agreement, parties, charge scope, rates/fees, limits, validity, adapter and delivery rules |
| QuoteAcceptance | Exact quote digest, payer/sponsor spend authority, resource/payment reservation |
| ContributionReceipt | Agreement, operation/segment, contributor, accepted output/work or funding, verifier, allocation |
| SettlementInstruction | Acceptance, obligation/receipt set digest, payer/payee, asset, exact amount, idempotency key |
| SettlementReceipt | Instruction digest, adapter/issuer, provider reference, amount, state, finality evidence |
| Adjustment / Closure | Prior receipt references, reason/authority, refunded/released/residual amounts, final account digest |

All records include version, type URI, unique ID, issuer authority, audience/context, sequence,
predecessor where relevant, validity/time evidence, and sensitivity/retention policy. The signed
deterministic-CBOR wrapper binds the exact CBOR-LD payload and semantic-bundle digest using the
[Cryptographic Profile's COSE_Sign1 format](./cryptographic-profile.md#9-stored-signatures-and-cose)
with record-type domain separation. Preserve evidence needed for historical verification after
credential expiry; expiry prevents new spending and does not erase an existing receipt.

Default encoded record ceiling is 16 KiB, with at most 16 quantity entries or receipt references and
nesting depth 8. Larger evidence sets use bounded content-addressed manifests fetched under their
own byte budget, never automatic recursive retrieval. Record stores have configured byte, count,
retention, and pending-operation limits. At capacity, pause new billable work; never evict replay or
outstanding-spend evidence merely to keep charging. Receipts and signatures remain external bounded
objects; NQuin stores typed references without changing the 48-byte ABI or allocating new opcodes.

Use the [QualiaDB core and Q42 persistence model](./core-storage-and-cache.md) for these records,
their indexed facts, and recoverable accounting mutations. Reuse does not make an unresolved
settlement disposable cache data; durable intent, source bytes, and projection must recover together.

### 6.3 State transitions and bounded exposure

```text
offered -> accepted/reserved -> active -> usage/contribution accepted
                                         -> settlement pending -> settled -> closed
```

Zero-price and nonmonetary agreements close after accepted contribution/reconciliation without a
monetary transfer. Paid work may proceed in bounded segments, each with its own exposure allowance.
Prepayment, postpayment, escrow, or conditional transfer is chosen in the quote. The profile MUST
state who bears non-delivery, nonpayment, and adapter failure risk; a signature alone provides no
atomic exchange of money for arbitrary service quality.

Small charges SHOULD accumulate locally and settle at an agreed amount, time, or closure threshold.
If a rail's fee or minimum amount exceeds the remaining authorization, retain a pending balance,
use an agreed subsidy/waiver, or pause; do not silently increase the debit. Refund fees are also capped.

The account deduplicates accepted work by `(agreement, operation, provider, segment)` and payment by
`(payer authority, adapter, idempotency key)`. A content agreement additionally deduplicates its
delivery obligations across providers by content/range identity. QSession ACKs, lost responses,
retransmission, rekey, reconnect, or route migration MUST NOT create new chargeable delivery.
Any agreed billing of transport retries remains a separate capped category.

Persist intent and reservation before external submission. If submission times out or the process
crashes, the result is `unknown`/`pending`, never presumed failed. Query/reconcile the same instruction
before retry, including after switching adapters; never issue a second payment to escape uncertainty.
An adapter that cannot support safe reconciliation cannot offer unattended retry for that operation.

Cancellation stops new work and reconciles accepted segments and unused reservations. Revocation
stops new access/spend without erasing prior obligations. Disputes isolate the contested entry; an
amendment or refund references the original instead of rewriting it. No punitive global reputation
score or new collection capability follows from a disputed account.

### 6.4 Offline operation and adapter boundaries

Offline allowances MUST be bounded by amount, resource, payee/group, expiry, and outstanding exposure.
The agreement states its double-spend assumptions: issuer-reserved allocations, a specific secure
hardware profile, or an explicitly accepted credit risk. A signed receipt or CRDT merge alone cannot
guarantee that a transferable offline balance was not spent in another partition.

During partition, pre-funded entitlements or accepted local credit may continue under those limits.
Unverifiable external settlement stays pending. Reconnection detects duplicate/conflicting spends
and applies the agreed reconciliation authority; ordinary last-writer-wins cannot settle the dispute.

Each payment adapter declares its asset namespace, authorization model, fees/minimums, finality and
reversal rules, timeout/reconciliation behavior, custody model, and network dependencies. Interledger
STREAM is relevant prior art for small transfers and money/data streams; QDNF does not claim STREAM
compatibility merely by using a QSession stream. See the
[Interledger STREAM specification](https://interledger.org/developers/rfcs/stream-protocol/).

Adapters using DNS/HTTPS or other Internet services cross an explicit LIG capability boundary. Native
service operation and locally funded agreements remain usable without that adapter. No payment rail
is designated mandatory or claimed implemented by this design.

## 7. Worked examples

### 7.1 A community-funded cached resource

A provider serves one verified content block using an estimated 120 J over six measured elapsed
seconds. The estimate uses an average 20 W model covering the named device only. The agreement makes
the resource free to eligible members and charges an authorized community pool. Its receipt retains
`120 J, estimated`, `6 s, measured`, evidence scopes, and the funding allocation. Unknown upstream
energy stays unknown. The recipient owes zero; a dropped ACK does not consume a second entitlement.

### 7.2 An illustrative metered charge

Use a fictional, nonredeemable community unit `CU`, with `1 CU = 1,000,000 micro-CU`. Suppose the
accepted tariff is 2 micro-CU/J and 30 micro-CU/device-second, with the latter explicitly excluding
energy. There are no other fees. For 120 J and six device-seconds:

```text
energy charge                  120 × 2 = 240 micro-CU
device-time charge              6 × 30 = 180 micro-CU
gross                                    420 micro-CU
allocated community subsidy              300 micro-CU
recipient obligation                     120 micro-CU
```

The payer's ceiling is 500 micro-CU and the resource reservation permits this quantum. The remaining
120 micro-CU can accumulate toward settlement under the agreed exposure cap. This invented tariff
illustrates dimensional accounting; it is neither a market price nor a universal energy/time rate.

### 7.3 Human contribution and release

A community accepts thirty minutes of translation work as 1,800 human-contribution seconds under
its agreed review rule. Device energy is separately measured, estimated, or unknown. The work can
satisfy a reciprocal obligation. It counts toward an asset's funding threshold only if that
agreement defines an accepted conversion/allocation; the protocol invents neither an exchange rate
nor a claim that all human work is interchangeable. Reaching the threshold changes the named licence
condition while ongoing hosting remains explicitly funded.

## 8. Conformance scenarios

These are required future tests for the optional profile, not results of this documentation review:

1. Operate a local donated/community network without a wallet, Internet, or price oracle.
2. Preserve measured/estimated/unknown states; reject incompatible units and quantity scopes.
3. Distinguish parallel device-seconds, elapsed time, human work, and Lamport sequence.
4. Reject overflow and rate/asset mismatch; splitting receipts cannot increase rounding charges.
5. Enforce aggregate reservations across concurrent services/providers, including in-flight work.
6. Payment, subsidy, and work receipts satisfy only their named obligation; blocks and consent win.
7. Lost ACK, content duplication, retry, reconnect, and path migration produce no duplicate debit.
8. Crash after submission reconciles the original instruction; unknown settlement cannot trigger a
   second adapter payment. Revocation/cancellation releases unused reservations correctly.
9. Fees, minimum transfers, and exhausted pools never exceed an accepted ceiling or invent credit.
10. Detect overlapping meter allocations and forged/exaggerated usage without treating signatures
    as measurement truth; preserve disputes and corrections.
11. Partition tests expose offline double spending and reconcile without LWW or fictitious finality.
12. Threshold contributions deduplicate; pending/disputed funds cannot trigger premature release;
    post-release delivery charges cannot silently revive a retired creation obligation.
13. Receipts avoid public personal identifiers, and bounded stores stop admission before overflow
    while retaining outstanding replay/settlement state.
14. Unsupported economic features fail the affected paid operation without disabling unrelated
    authorized services or introducing payment-provider calls in forwarding loops.
15. Changed CBOR-LD context/ontology/table bindings, unknown duties, and unsupported SHACL/N3
    constructs fail before access or debit; prior acceptances retain their original semantics.

## 9. References and implementation evidence

The preceding rules are QDNF design choices. External standards supply unit definitions, policy
vocabulary, and settlement prior art; they do not certify this design's implementation.

- [BIPM SI](https://www.bipm.org/en/measurement-units): physical units and dimensions.
- [Ontological Contracts](./ontological-contracts.md): required CBOR-LD semantic bundle, validation,
  signature binding, and offline execution profile.
- [W3C ODRL Information Model 2.2](https://www.w3.org/TR/odrl-model/): permissions, prohibitions,
  duties, and agreements as potential policy interchange vocabulary. A mapping still needs a
  defined profile and enforcement tests; ODRL metadata alone does not enforce a payment or licence.
- [Interledger STREAM](https://interledger.org/developers/rfcs/stream-protocol/): payment-stream
  prior art; adapter interoperability must be demonstrated separately.
- [Source and Current-Stack Review](./source-and-current-stack-review.md): repository evidence and
  gaps in existing commons/economic primitives.
- [Implementation and Conformance](./implementation-conformance.md): work package and release gates.
