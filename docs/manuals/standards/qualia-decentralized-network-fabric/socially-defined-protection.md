# Socially Defined Protection for Vulnerable People

**Status:** Proposed requirements, 2026-09-07; implementation pending.
This profile specializes [security and governance](./security-privacy-governance.md),
[Identifier Fabric](./identifier-fabric-integration.md), [QSR](./qualia-scoped-rendezvous.md) and
[evidence retention](./electronic-evidence-and-retention.md).

## 1. Purpose and interpretation

Provide safer discovery, communication, delegated action and recovery for vulnerable people,
particularly children and politically exposed persons (PEPs). PEP is used here for a person with
public/political exposure, consistent with the project's sanctuary example; it is a protection
context, not an accusation or automatic financial-risk decision. The financial-sector
[PEP guidance](https://www.fatf-gafi.org/en/publications/Fatfrecommendations/Peps-r12-r22.html)
is not a QDNF mandate to screen users or publish their classification.

The profile also supports survivors, people under coercion, refugees, humanitarian workers and
others whose circumstances require protection. A person can request private protective settings
without proving membership in a public vulnerable-person class.

The design follows the principles of children's privacy, participation and evolving capacities
described in [General Comment 25](https://docstore.ohchr.org/SelfServices/FilesHandler.ashx?enc=gPlr12x%2FqkkXuzyCGy9WjbgeS3G8LMBqM1oGbmJ%2FUxfS1Aawk4NvOdZroofMkqs55RZmw55R6urONi%2FUSq6xSw%3D%3D).
It does not claim legal compliance or that encryption alone prevents abuse.

## 2. Required invariants

1. A relationship, payment, role, public office, shared group or co-attestation is never blanket
   permission to discover, contact, introduce, locate, observe or control a person.
2. Protection status, age evidence, home/school/refuge location and private relationship graphs
   are sensitive data; public search must not reveal that a person belongs to this profile.
3. Paying providers or corporate actors cannot buy contact with a protected person or bypass blocks.
4. Guardianship is a scoped duty; neither a guardian nor an institution owns the child or receives
   automatic access to every message, key, location or report.
5. The child/person's accessible participation, evolving capacity and ability to contest a decision
   are part of policy. There is no universal age threshold or permanent incapacity inferred by QDNF.
6. Protective defaults preserve access to trusted help and basic entitled services. Isolation,
   forced monitoring or compulsory payment is not an acceptable default protection mechanism.
7. Authenticating an instrument is distinct from proving the user's age, present intent, personal
   identity, safety or the correctness of a guardian/official claim.
8. Already disclosed bytes cannot be recalled, and a compromised endpoint or observer may still
   cause harm. Report these limits without presenting risk scores as certainty.

## 3. Policy records and decision order

A ProtectionPolicy binds protected scope, operations, disclosure/contact defaults, selected
relationship grants, participation/consent requirements, review/recovery authority, time/profile
version and appeal route. Evidence of age/capacity/mandate is separate from the public instrument.
Use minimum necessary attributes or scoped attestations; do not require raw identity documents,
biometrics or public date of birth as a universal protocol admission condition.

Evaluate current block/revocation, protected disclosure policy, valid delegated role, purpose,
person/child participation requirements, resource limits and finally economic terms. Payment can
never repair a failed earlier gate. Unknown/conflicting claims produce a bounded safe pending state
with local/trusted help options, not permission or an indefinite inaccessible account.

Signed delegation names each operation: discovery, direct contact, introduction, group invitation,
content access, location, recording, automation, moderation, recovery and evidence disclosure.
Authority is directional, purpose-limited and nontransitive. A permitted introducer cannot add a
third party to an existing private relationship or delegate its role indefinitely.

Humanitarian fee exemptions and protective settings are separate claims. A protected user need
not reveal the reason for protection to obtain a general personal-use entitlement. A PEP using
resources on behalf of an incorporated principal follows that operation's economic rule while
retaining privacy/safety protections; public office is neither free corporate use nor adverse trust.

## 4. Discovery and contact state machine

```text
NotDiscoverable -> ScopedInvitation -> RequestPending -> ConsentedContact -> Active
                                      |                    |
                                      +-> Rejected          +-> Suspended / Blocked / Expired
```

Public listing is a separate opt-in operation under the applicable policy. Knowledge of an
identifier does not bypass the invitation/consent gate.

- QSR must exclude protected-person membership, age, status, relationships and precise location
  from public facets, proofs and absence responses. Public services can be advertised under
  separate role instruments without exposing their operator's private persona.
- Private lookup tokens remain scope/purpose-bound. Equality/access-pattern leakage is assessed;
  token rotation is not retroactive secrecy. No public vulnerable-user directory is constructed.
- Invitations and contact requests have independent low quotas before expensive crypto or human
  notification. Blocked/rate-limited attempts produce nondisclosing responses; a response must not
  reveal that a hidden account exists or whether a particular person blocked the requester.
- New identifiers, aliases, paid relays and other group members cannot automatically override a
  block. Within authorized local scope, correlate evidence of evasion only as necessary; do not
  invent a global identity blacklist or claim perfect resistance to new actors.
- Group membership does not enable unsolicited private messages, location access or invitations.
  Requesting contact and delivering content require distinct grants.
- Recheck authority at queued delivery, offline mailbox retrieval, session migration and device
  recovery. Disable revoked invitations and queued disclosures across epochs/cells.
  ProtectionPolicy specifies maximum authorization age, required policy/issuer checkpoints and
  permitted revocation delay. If the required freshness cannot be established, sensitive delivery
  stays pending; separately authorized trusted help remains available. A profile requiring a live
  authorization check cannot accept a cached/offline grant as equivalent. Bounded-validity profiles
  disclose their possible revocation delay instead of promising instantaneous global revocation.
- Discovery/contact controls include forwarded messages, embeds, media metadata, external links,
  attachments and LIG fetches. No automatic remote resource loads that reveal private addresses
  or location to an unapproved sender.

## 5. Guardians, public roles and trusted help

Verify guardian/caregiver/fiduciary authority from the selected policy source, including purpose,
duration, permitted actions and conflicts. A guardian's key possession does not establish unlimited
legal authority. Support multiple legitimate roles without merging their identifiers or pooling
their powers. Review policies as circumstances/capacity change; transfer ordinary control and
expire outdated roles without deleting the person's history.

Recovery and high-impact changes require the applicable independent safeguards, which can include
multiple authorized reviewers. Several keys held by one controlling party do not constitute
independence. Emergency powers are narrow, time-limited, evidenced and reviewable; they cannot
turn into persistent surveillance or quietly override a person's protective block.

An abusive/compromised guardian, employer, administrator or epoch publisher is an explicit threat.
Offer a separately authorized confidential trusted-help/reporting route and alternative recovery
authority under the protection policy. Do not automatically notify the alleged abuser, expose the
report in a shared activity feed or grant the subject of a report access to its evidence.
Preserve due process and human review without equating an allegation with guilt.

The protection policy names an independent safeguard-amendment authority and a minimum help/
appeal route that a contested guardian/controller cannot unilaterally remove. Changes require
the affected person's appropriate participation and the policy's independent review, with separately
verifiable authority and an auditable transition. Keep independently usable help locators available
outside the contested directory/control path; do not require the alleged abuser's grant to contact
that help. Confidential help access does not grant authority to transfer account control: any
takeover/recovery still needs its own validated process, and an allegation alone cannot authorize it.
If one party controls the endpoint and every communication path, QDNF cannot guarantee access;
the interface must disclose that limit and provide supported out-of-band options where available.

For PEPs and humanitarian workers, separate office/organization duties from personal/family
relationships. Delegated staff or bots may manage specified public-service traffic, not private
health, family, location or sanctuary data. Officeholder succession cannot inherit the predecessor's
private persona. Protective measures apply without party affiliation, prominence-based ranking or
a presumption of wrongdoing.

## 6. Reporting, evidence and safe recovery

### 6.1 Known-peer confidential clinical exchange

Secure communication of medical information between a patient/protected person and a known
clinician is an explicit supported target workflow. This includes PEPs, children of VIPs and other
people at risk through association. The older [Sanctuary definition](../../webizen_permissive_commons.md)
already places medical records in private bilateral contexts; QDNF realizes that intent through
private relationship grants and QSession. This is a design requirement, not a deployed-service claim.

**Establish the relationship:** the peers may pair in person or through an already authenticated
channel, confirming the intended recipient's instrument/key binding and a private invitation.
Recognizing a name, knowing an email address or finding a clinician role assertion is insufficient
to authenticate a network endpoint. Bind a contextual patient-clinician relationship with the
selected care purpose, permitted operations, expiry/review, protected delivery policy and directly
usable private route hints. Professional/organization evidence is checked where the chosen care
policy requires it; the protocol does not mandate one worldwide professional or person registry.

**Use the relationship directly:** valid pre-established relationship grants satisfy the relevant
invitation/contact prerequisites. They do not require repeated public discovery or repeated prompts
for every routine exchange within the accepted scope. Neither the patient, their PEP/VIP association,
nor the clinician-patient edge is published to a public index. Exchange private route updates over
the relationship; use QSR only within an explicitly authorized private scope when needed.

**Agree the actual recipients:** the grant may identify a particular clinician or an explicitly
authorized care team, with separate role/membership changes. Employer, insurer, public office,
parent/guardian and other clinicians do not acquire access from association alone. For a child,
apply the selected participation/capacity/guardian policy without a blanket guardian-copy rule.
Protect confidential help from a contested guardian as specified in §5.

**Transfer selected information:** authenticate both endpoints, establish the selected QSession
security profile and apply current disclosure permissions to chosen messages, documents, images
or record subsets. Preserve source, version and recipient bindings and bound attachment parsing.
Gate incoming clinical replies independently under the agreed bilateral permissions. A standing
grant can cover routine updates; adding recipients, broadening record scope, recording a consultation,
AI transcription/training, or onward referral requires its own applicable authorization.

**Define the encryption boundary:** intermediaries relay ciphertext; storage and clinical endpoints
use their declared access/key/backup controls. If a hospital service terminates encryption or its
administrators hold decryption keys, disclose that recipient boundary before sharing; do not claim
that only the named doctor can read the data. Generic network, payment and audit logs contain no
medical payload. Care data does not become a forensic archive or model-training source by default.
End-to-end payload confidentiality does not guarantee traffic anonymity or a safe compromised endpoint.

**Handle intermittency and changes:** an authorized encrypted mailbox may hold pending material,
but storage, delivery, clinician review and clinical response are separate states. Enforce the
required freshness at delivery; an unavailable clinician or stale grant cannot become an implicit
care-team substitution. Device/key replacement, staff departure or referral needs a verified
relationship/authority transition before private routes or content are disclosed. Withdrawal stops
future unauthorized access; already received records follow their separately accepted retention
and access policy and cannot be magically recalled.

**Present a usable workflow:** show who can decrypt, which information will be shared, applicable
standing permissions and pending/delivered/reviewed states. A transport acknowledgement is not
evidence that a clinician read the information or provided care. Offer separately agreed fallback
contact instructions when delivery fails; do not silently send records through a different provider.
Personal-care humanitarian eligibility and any clinic/provider contributions remain separate from
patient confidentiality; paying for an incorporated service never purchases wider medical access.

Acceptance covers a privately paired adult PEP sending selected records, a child of a VIP sharing
under the applicable care policy, routine clinician replies under standing grants, an authorized
care-team change, a revoked clinician with a pending mailbox, hospital-side decryption disclosure,
and a failed delivery with no public discovery or unapproved recipient fallback.

### 6.2 Reporting and safe recovery

Provide accessible block, mute, report, withdraw-consent and trusted-help actions. Explain their
effects without claiming all remote copies disappear. Recovery can revoke compromised keys,
sessions, invitations and route disclosures while preserving necessary custody/obligation evidence.

Default reports contain minimal selected material and user-visible purpose/recipient information.
Promote exact evidence, interpretation context and relevant contrary material only through the
existing authorization/hold lifecycle. No blanket message retention, generalized content surveillance
or routine export to police, employers or guardians is implied. Specific external disclosures need
their own authority and review; the protocol does not promise successful prosecution.

Retention and deletion policies distinguish protective state, temporary diagnostics, abuse reports
and held evidence. A report's expiry must not silently release another hold; a protective block
need not retain a full message archive. Protect against report spam, malicious allegations, witness
correlation and indefinite unresolved case accumulation within admitted budgets.

If privacy-preserving crypto/key/custody support is unavailable on a target, the interface reports
that limitation and offers supported local/trusted alternatives. It must not return fake successful
protection or silently upload private content to an external service.

## 7. Economics, resources and operator accountability

Personal/humanitarian entitlements follow [finite compensation](./finite-project-compensation.md).
No creation-recovery surcharge, forced labor or sale of behavioral data funds a child's entitled
basic access. Operating capacity is explicitly donated, sponsor-reserved or otherwise accepted;
scarcity has a disclosed fair queuing/degradation policy and cannot silently become personal debt.

Reserve bounded contact-control, revocation, recovery and reporting capacity separately from bulk
traffic. A request claiming emergency/protection cannot exhaust the host; validate scope and use
proportional reserved paths. Do not claim unlimited safety service under an exhausted resource pool.

Logs identify responsible instruments, delegated actions and reviewer decisions with scoped
attribution. Protect the audit from the potentially abusive operator under the selected custody/
trust model. A single operator that controls endpoints, all recovery keys and all witnesses cannot
provide independent safeguards merely by signing several records.

## 8. Acceptance scenarios and owners

| Scenario | Required behavior |
|---|---|
| Stranger queries a child/PEP facet or existence proof | No unauthorized listing, classification, membership or absence leakage |
| Approved group member sends unsolicited private contact | Independent contact consent still required |
| Paying corporate bot asks for location/contact | Economic status cannot bypass protective policy |
| Guardian is the reported threat | Confidential alternative help/recovery; no automatic notification/disclosure to that guardian |
| Blocked actor rotates keys or uses an introducer | No automatic regrant; scoped evasion handling with no global person registry |
| Revoked mandate has queued custody messages | Recheck before delivery; expose no stale authorized content |
| Partitioned mailbox holds formerly valid guardian grant | Keep sensitive delivery pending when freshness cannot be established; preserve separately authorized help |
| Contested controller removes help/recovery options | Reject unilateral safeguard amendment; independently reachable help remains distinct from account takeover |
| Worker leaves office / child gains independent capacity | Retire old role scopes and migrate control through policy without transferring private persona |
| Protection backend or funding pool unavailable | Honest degraded/pending state; supported help/local alternatives and no invented debt |
| Malicious report requests broad evidence export | Scoped authorization, minimum disclosure, contestability and no automatic guilt |
| Attachment triggers an external fetch | No hidden network/location disclosure before the required consent |

SEM-01 owns ProtectionPolicy and authority compilation; NET-04 owns private QSR discovery and
invitation exposure; NET-05 owns gated sessions/delivery; SVC owns subscriptions/custody rechecks;
EVD owns reporting evidence; OPS-02 owns accessible help/recovery workflows; QA-02 owns independent
adversarial checks. Shared runtime owners reserve control capacity.

Use directory-backed policy/contact/invitation/recovery/reporting libraries with focused files,
bounded caller storage and separate backend/tests. Apply the 42 MiB Webizen pass and 512 MiB
ordinary-cell limits; the size of a protected community does not justify unbounded in-memory graphs.
