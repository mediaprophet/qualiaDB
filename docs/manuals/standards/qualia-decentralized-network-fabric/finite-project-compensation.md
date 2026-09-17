# Finite Project Compensation and Humanitarian Access

**Status:** Proposed requirements, 2026-09-07; implementation pending.
This profile specializes [commons economics](./commons-and-resource-economics.md) and
[ontological contracts](./ontological-contracts.md). It implements the user's objective of preventing
exploitative uncompensated contribution while making humanitarian ICT freely usable by people.

## 1. Required outcome

A cooperative project records accepted contributions, cost and agreed compensation/return.
Ontologically defined usage classes determine which operations fund that finite obligation.
Ordinary personal humanitarian use is free of mandatory creation-recovery charges; incorporated
entities and agents acting for them are the primary required-contribution classes under applicable
accepted terms. Eligible humanitarian work can be exempt even when performed through an organization.

Eligible usage contributes an amount above separately accounted event operating costs and fees.
Only the designated recovery component pays down the named project obligation. When the agreed
target is fulfilled, that creation-recovery component stops. The profile does not impose perpetual
royalties, automatic compound debt or a new target merely because another provider serves the asset.

This requirement is not a promise that software alone can eradicate exploitation. Contributors
retain agency, attribution, agreed remuneration and contestable records; voluntary donation of work
must be affirmative, not inferred from publishing source, lacking a wallet or missing a payment rail.

## 2. Separate ledgers and meanings

| Record | Required contents |
|---|---|
| Contribution | Contributor instrument/payee authority, voluntarily accepted purpose, artifact/version, labor or resource description, provenance, acceptance and dispute status |
| Cost assessment | Actual/estimated/unknown quantities and attributable costs, method, valuation unit, included categories and evidence; effort is not automatically useful output |
| Value/compensation agreement | Accepted useful contribution, beneficiary shares, compensation and any agreed return, cap, grant/subsidy offsets, rights and amendment authority |
| Project obligation | Stable obligation ID, covered asset/version set, finite target, settled credits, reserved recovery, release terms and authoritative writer/control profile |
| Usage classification | Actor, acting principal, delegation, operation, purpose, beneficiary, relevant classes, evidence validity and applicable exception |
| Event operating account | Energy, storage, communication, AI/compute and accepted service costs for this event; distinct from historical creation costs |
| Recovery allocation | Quote/operation/obligation IDs, recovery component, reservation, funding source, finality, distribution and deduplication references |
| Fulfilment record | Accepted final credits, separately authorized non-cash discharges, released charge component, effective policy/version and retained proof of fulfilment |

Costs, value and price are different. Care, maintenance, translation, accessibility and stewardship
can be useful contributions without surveillance or artificial computational work. Record prospective
compensation before work where applicable; do not make an existing employment/payment obligation
contingent on eventual project recovery unless the actual agreement lawfully establishes that model.
A beneficiary's claim against a project is not automatically a debt owed by every human user.

Use one agreed accounting unit per obligation. Keep joules, scoped seconds and typed compute as
separate quantities; conversion into a valuation unit requires explicit accepted rates and rounding.
No exchange rate, risk premium or return-on-investment entitlement is inferred automatically.

## 3. Ontological applicability and exceptions

Classify the **operation and represented principal**, not the physical person's permanent identity.

| Operation class | Creation-recovery treatment |
|---|---|
| Natural person acting personally for humanitarian ICT | Exempt; no wallet, compulsory labor, advertising or behavioral-data exchange as substitute payment |
| Child or protected person using an entitled personal service | Exempt under the personal profile; protective support never becomes a debt imposed on them |
| Humanitarian worker acting within a recognized humanitarian mandate | Exempt or sponsor-funded according to that mandate, including qualifying incorporated organizations |
| Incorporated entity acting for its own organizational/commercial purpose | Required recovery contribution while the named obligation remains open, under the applicable instrument |
| Human or AI/software agent acting for an incorporated principal | Inherits the operation's represented-principal rule; natural-person/device wrapper does not bypass the duty |
| Mixed-purpose or contradictory role claims | Split identifiable operations/costs or use an explicit reviewed classification; do not silently bill all personal activity |
| Unknown principal, expired mandate or unsupported ontology | Classification unresolved; no invented debt or blanket denial of basic entitled personal access |

Each decision binds full entity/claim/handle/instrument references, ontology and rule versions,
delegation chain, purpose, expiry and source. A wallet, corporate email address, IP range, inferred
employment, browser/device owner, biometric classifier or social relationship is insufficient proof.

Humanitarian exceptions take precedence over the general incorporated-principal rule when verified
for that operation. Being employed does not make someone's personal use corporate. Conversely,
verified corporate proxying remains corporate even through a human account or an AI agent.
Government bodies, unincorporated groups and other classes require explicit policy; do not invent
their duties by analogy.

Use minimum-disclosure eligibility evidence and context-specific credentials. Free personal asset
rights do not require a universal proof-of-personhood registry. For scarce services, sponsor quotas
may need a scoped entitlement proof, with accessible alternative review or local/offline use where
supported. Missing sensitive evidence must not force publication of identity or vulnerability.

Webizen compiles supported class/exception/delegation rules and returns Exempt, SponsorFunded,
ContributionRequired, Fulfilled or Unresolved with bounded evidence. Runtime authorization,
economic classification and authority to spend remain separate decisions. A class assertion alone
does not create an enforceable contract or permit an automatic debit.

## 4. Finite target and charge arithmetic

For a fixed obligation/version in a common agreed accounting unit:

```text
T = max(0, accepted_creation_cost + agreed_compensation_or_return - allocated_prior_funding)
S = accepted finalized recovery credits
W = authorized non-cash discharge adjustments
R = max(0, T - S - W)             # remaining collectible obligation
H = sum(live recovery reservations)
available = max(0, R - H)

A = min(accepted_event_recovery_quote, available)
payer_quote = O + F + A
```

T is nonnegative and capped by signed terms. O is the accepted operating/service component for
the event. Prior funding subtracted in T is not counted again in S; S begins with credits accepted
against the resulting obligation. Enforce S + W + H <= T atomically across all participating collectors.
Reject circular self-payments, duplicated cost provenance and artificial resource consumption as
evidence of new compensable work. F is the agreed fee component; A is the project-recovery
contribution. Do not include
the same creation remuneration in T and O or the same operating resource in two tariffs.

While R is positive, a contribution-required event includes positive A where recoverable capacity
is available. A has its own beneficiary/obligation allocation; it must not disappear into an
undisclosed provider margin. If all remaining capacity is reserved, pause the recovery-dependent
admission or proceed under an explicitly allowed operating-only/sponsored policy. Do not issue an
extra charge on the assumption that a previous reservation will fail.

For an exempt user, recipient_due is zero under the applicable free-access entitlement. Operating
costs may be borne by a sponsor, donated infrastructure or the user’s local equipment; there is
no claim that free asset rights create unlimited free remote compute or electricity. A volunteer
donation toward the project is separate and cannot become the price of entitled access.

If fees or actual event costs exceed the accepted allocation, honor the agreed ceiling and variance
rule, obtain a new accepted quote where needed or stop new work. Do not silently consume A for
operating overruns while claiming the same amount discharged the obligation. Unknown resource cost
requires an explicit capped estimate/subsidy policy; unknown energy is not zero.

For tiny events, accumulate bounded authorized micro-accruals and batch settlement. Disclose
minimum-transfer fees and settlement thresholds. Do not manufacture an economically wasteful
on-chain payment per packet or let indefinitely pending small balances masquerade as paid-down cost.
The final A is clamped to the remaining amount, even if below the normal contribution rate.

## 5. Atomic pay-down, distribution and fulfilment

The obligation ledger has one accepted serialization/control owner per project obligation, backed
by CORE-03, or a separately proven replicated transaction protocol. It need not be a global ledger.
Multiple payment providers, network cells, assets, mirrors or aliases share that same obligation
identity and remaining-cap ledger.

State model:

```text
Draft -> Agreed -> Open -> FullyReserved -> Fulfilled
                      \-> Disputed / Suspended
```

FullyReserved is not Fulfilled. A failed/released reservation can return to Open. Fulfilled means
S + W covers T, with S satisfying the prescribed finality/reserve policy. Store a
durable fulfilment record and publish a new policy generation setting creation recovery to zero.
An initially zero target closes directly when the agreement is accepted, without a payment event.
Waivers are separately attributed non-cash discharges: affected rights holders must authorize them,
they may consume only unreserved collectible capacity, and they cannot be displayed as paid
compensation or erase a beneficiary's independent entitlement. Resolve outstanding reservations
before waiving their capacity. Prior funding above the cost/return target is allocated explicitly
as surplus; it cannot produce a negative target or secretly enlarge the new obligation.

1. Atomically reserve A before presenting an executable charge authorization; quote display alone
   may be indicative but cannot authorize an unreserved debit.
2. Bind the spend grant and settlement intent to the operation, obligation, actor/principal,
   amount/component allocation, finality profile and expiry.
3. On accepted final settlement, atomically release that reservation and credit its unique A once.
   Pending, disputed, rejected or merely submitted funds cannot count as final recovery.
4. Distribute recovery by accepted beneficiary shares. Record project discharge, payee entitlement
   and actual payout separately; an operator cannot mark contributors paid merely by collecting money.
5. When S + W reaches T, publish fulfilment, retire creation-recovery admission and invalidate open
   unexecuted quotes. Ongoing service prices can continue only under their separate accepted terms.
6. Reconcile late/duplicate transfers through the original identity. Excess recovery is refunded,
   or redirected only with fresh explicit donor consent; it cannot silently become project profit.
7. Preserve contributors' payout claims and recovery evidence through closure, custody changes
   and key rotation. A recovery operator's disappearance does not erase the beneficiary ledger.

Offline collection uses disjoint pre-reserved recovery allotments with bounded total face value.
A timeout alone cannot reissue an allotment while prior settlement exposure is unresolved. If safe
reconciliation is unavailable, suspend new recovery collection; local/exempt use follows its own
policy. Network partition does not justify exceeding T.

Additional grants or voluntary contributions allocate only to available recovery capacity; any
overlap with already reserved amounts needs atomic cancellation/reconciliation before settlement.
Across all sources, accepted recovery cannot exceed the agreed target. Rounding residuals have
an explicit last-payment/waiver rule; they cannot keep a nominal balance charging forever.

## 6. No perpetual reset

Fulfilment of this profile's creation obligation is terminal. Later chargebacks/disputes use the
agreed reserve, payer remedy or project loss allocation; they do not automatically reinstate charges
against unrelated users. Suspend uncertain settlement before declaring fulfilment where required.

A new asset version, renamed project, mirror, fork, provider change, new cell or model checkpoint
does not reset the same historical costs. Maintain obligation lineage and credited allocations.
A derivative may have separately accepted new work with its own bounded target, but inherited
recovered costs cannot be charged again. Recurring maintenance or retraining needs separately
accepted incremental work/service terms; arbitrary relabeling of creation cost is prohibited.

An accepted capped return is included in T before collection. This finite profile does not use an
uncapped compounding schedule. Amendments cannot enlarge already accepted event duties or undo
promised fulfilment; new work requires a distinct agreement and a disclosed relationship to old work.

## 7. Rights, instruments and practical enforcement

Track asset permissions, copyright/licence authority, hosted-service terms, compensation agreements
and voluntary donations separately. Enforce only duties supported by the chosen rights/contract
instrument and actual control boundary. A runtime gate cannot impose retroactive restrictions on
copies already provided under an irrevocable grant or guarantee enforcement of offline use.

The term permissive commons here describes this project's economic model. Do not automatically
label an instrument imposing user-class restrictions as OSI-compliant open source: the
[Open Source Definition](https://opensource.org/osd) includes nondiscrimination requirements.
An open-source asset can coexist with paid hosting, sponsorship or separate services, but the
service agreement must not be presented as changing rights already granted in the asset licence.
Freeze the actual permission/duty instrument and jurisdiction-dependent applicability before
deployment; this protocol document does not itself enact legal liability.

## 8. Example and acceptance cases

Illustrative common unit CU: T = 1,000, S = 997, no outstanding reservation, R = 3.
A corporate event quotes O = 0.20, F = 0.01, normal A = 2: payer quote is 2.21.
After final settlement S = 999. The next otherwise identical event reserves A = 1 and quotes 1.21.
Its finalized credit fulfils T. Subsequent events have A = 0; separately accepted O/F may remain.
An entitled personal user owes zero under the free-access profile throughout; its service sponsor
has a separate allocation. No physical unit or real exchange rate is implied by CU.

Required tests include two cells racing for the last unit; parallel donations and in-flight invoices;
a settlement whose fee changes; duplicate receipts; late/irreversible transfers; offline allotment
reuse; false humanitarian credentials; a corporate agent wrapped in a personal account; a humanitarian
worker using an incorporated organization; personal use by an employee; derivative double recovery;
chargeback after fulfilment; authorized residual waiver; zero initial target; and unavailable beneficiary payout.

The user experience shows remaining target, allocation rules, finalized versus pending recovery,
the applicable personal/corporate/humanitarian rule and the date/state of fulfilment, without exposing
private contributors or users. Contributors can contest valuation, view their accepted entitlement
and choose whether to donate work. No forced labor, covert telemetry or deprivation of basic
entitlements is an acceptable collection mechanism.

## 9. Implementation owners

FND-02/SEM-01 own class/exception/delegation meanings and bounded compilation. ECO-01 owns typed
quantity/valuation evidence; ECO-02 owns contribution acceptance, target, reservation, finality,
beneficiary allocation and fulfilment. CORE-03 owns atomic recovery and durable intents. RT/NET
consume compiled policy handles; OPS-02 owns contributor/user/operator workflows; QA-02 owns
independent multi-cell/rail/exemption tests.

Use focused libraries under existing economics owners: contributions, valuation, usage_classes,
recovery_target, reservations, allocations, fulfilment and separate settlement backends/tests.
No monolithic compensation engine or duplicate ledger. See the
[implementation workstream](./qdnf-imp/workstreams/06-roles-economics-evidence.md).
