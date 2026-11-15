# Native inference policy setup

The migration adds an explicit native setup route for signed policy and
authenticated actor associations. **The legacy decoder and browser token
acceptance paths still require the G3 integration.** Successful setup or a
successful foundation test does not establish that every runtime token is
already governed by this policy.

A subject may be a person, entity or agent with multiple identifiers. The
existing principal kinds are Human, SoftwareAgent and Organization; these are
the currently supported categories, not an exhaustive definition of identity.
The selected identifier retains its scheme, issuer, project and association
evidence. Authentication proves a current association and key possession.
Policy installation is a separate operator authority, and authentication alone
does not grant inference permission.

## Explicit operator commands

The `qualia-cli inference-policy` command family takes an absolute `--profile`
directory. Vault and policy metadata are bound to that same canonical directory
in a dedicated child process. These commands are local operator actions; they
are not registered as HTTP, MCP, chat or remote signing tools.

The protected vault route currently supports the Windows native credential
store. Other native targets explicitly refuse this route until qualified.
Ordinary commands open an existing protected vault and refuse missing or
unusable state. They do not create an identity or install a default policy.
`init-vault` is a separate, explicit creation operation.

```powershell
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' installer-key
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' create-actor-key --label researcher
```

If a new profile and protected vault are intended, create them explicitly first:

```powershell
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' init-vault
```

The commands print public keys only. Actor-key creation does not enroll that
key, invent a subject, or sign a project admission. Obtain a real admission from
the independently selected project authority, including the subject, controller
key, identifier associations, membership and validity evidence.

Provide the following bounded JSON files to configuration:

| File | Rust schema | Content |
|---|---|---|
| Configuration | `InferenceHostConfiguration` | Installer identifier/public key and independent project ID, issuer and public key |
| Admission | `SignedAdmission` | Actual project-signed enrollment evidence |
| Document | `DocumentRevision` | Explicit project/document/source revision and digest |

The installer public key must match the selected vault's ControllerSigning key.
Every document revision field is covered by the stored configuration signature.

```powershell
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' configure --configuration configuration.json --admission admission.json --document document.json
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' login-check --subject 'urn:subject:researcher' --identifier identifier.json --signer-label researcher --lifetime-seconds 300
```

`identifier.json` contains `value`, `scheme`, `issuer`, `project_id` and
`evidence_ref`. These select an existing admitted association. The command
performs the real key-possession challenge, prints its policy-scope metadata and
logs out. The output is not a reusable session credential. A runtime must perform
its own live login within its process.

## Policy installation and lifecycle

The operator authors `PolicyRules` with explicit actor scopes, model/request
scope, graph scopes and the required Webizen program and modality evidence.
Scope metadata may describe several identifiers for the same subject. Copying
that metadata cannot construct a live authenticated session. Authoring
delegation does not automatically grant inference access.

```powershell
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' status
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' install --rules policy-rules.json --expires-at <unix-seconds> --expected-generation <current-generation>
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' revoke --expected-generation <current-generation>
qualia-cli inference-policy --profile 'C:\QualiaProfiles\research' replace-admission --admission signed-successor.json
```

Use the actual status generation when installing or revoking. The placeholders
above are arguments to supply, not executable PowerShell values. No permissive
rules file is synthesized by these commands. Missing, expired, revoked or
incompatible policy refuses admission. Signed-admission changes require fresh
authentication and policy authorization for the resulting association scope.

The policy store uses an exclusive lock and a separate signed generation/digest
anchor. Restoring an older policy file alone is rejected. A whole trusted-store
rollback is outside this protection without an independent monotonic anchor.
Anchor-first updates fail closed on interruption; they are not a claim of an
atomic two-file transaction.

## Qualification boundary

The established serial core profile passes 10,470 tests with three ignored.
Its default-parallel run has two retained failures involving requests sharing a
single inference worker; a matched published/current comparison also reproduces
concurrent failure exposure. No production concurrency repair is claimed here.

The actual-source association/policy foundation passes 79 distinct focused tests
in both debug and optimized profiles, including current-association revalidation
after lock acquisition, expiry/revocation, capacity refusal and zero-allocation
acceptance. The four targeted client policy tests also pass, including actual
sealed-signer authentication, signed configuration, fresh/restarted processes
and refusal cases; this is not a claim that the full client suite ran. Current
CPU-only and WASM builds pass. Three CLI boundary tests and the actual CLI binary
build pass. The native consumer harness passes three tests, including real Vulkan
compute/readback on the NVIDIA RTX A2000.

The protected Windows credential-store test executes 24 operator steps, including
three subject kinds with two identifiers each, sealed actor keys, explicit policy
installation, revocation and admission replacement. Its four disposable profiles
and exact owned credentials are removed after success, error, unwind and confirmed
child timeout; absence is checked before removing each profile. The sandbox lacks
a Windows credential-store logon session, so the successful test runs the same
binary in the normal Windows session. It uses no live user vault. This test invokes
the already-bound child command; the ordinary parent-to-child wrapper still needs
its separate process-lifecycle smoke test.

Normal inference must receive a current Webizen decision before accepting a
token and publishing its effects. Numerical state restoration, grammar/sampler
state and publication ordering remain required caller integration work. Only
explicit developer/debug/lab execution may omit Qualia authorization; enabling
debug instrumentation in a governed session does not disable governance.
