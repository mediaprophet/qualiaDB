# Qualia sensing inspection application

This standalone source package supplies the `qualia-sensing-inspect inspect`
entrypoint and a reusable Rust `sense::inspect(&SenseAction)` function. The
function returns a JSON report without stdout; `sense::run` formats that report
for the CLI. Inspection checks exact bytes and declarations. Signature validity
does not establish an authorized signer, present permission or chain membership.

The application requires explicitly selected local `.10d`, `.qes`, `.scr` and
binding JSON inputs. It does not initialize a model, acquire sensors or publish
data. Run the already-built binary's `--help` for argument details once runtime
qualification is authorized. No executable is included in this source draft.

Optional `--index-field-id`, `--index-row` (zero-based), and `--index-kind`
(decimal u16) must be supplied together. The kind must also appear in the
binding's explicit `supported_extension_kinds` allow-list. Inspection opens
`SensingIndexView` on the same checked container resolver, uses the binding's
physical field declaration, and requires the selected canonical record to
equal the decoded sidecar record. Without these flags, index membership is
unassessed (`index_record_matches_sidecar: null`). Successful comparison reports
`true`; it does not authenticate the extension namespace, binding, container
identity key or signer. There is no automatic profile or registry adoption.

## Local directory packaging

`package.ps1` accepts an **already-built, caller-selected** Windows executable,
an existing protected local output parent, an explicit target declaration and
a byte budget. It does not invoke Cargo, build, run the executable, resolve
dependencies, download libraries or capture data. It stages a bounded directory
package under that output parent and renames it without replacing an existing
destination. It includes this README, the repository license, distribution
declaration and an integrity manifest. Sensing inputs, keys, captures and reports
are not collected or bundled automatically.

The package includes `verify-package.ps1` for a separate bounded, read-only
integrity check. From a separately trusted/reviewed copy of that verifier, an
operator using PowerShell 7.4 or later can supply `-PackageDirectory` and
`-ByteBudget`. It checks the exact package layout, entry lengths and SHA-256
hashes without running the executable. It rejects extra entries, final links
and malformed manifests. Its bounded System.Text.Json parser rejects duplicate,
unknown or missing object properties and enforces the same fixed file names,
32-byte owner marker, nonempty executable and lowercase SHA-256 declarations
as the Rust release verifier. It does not authenticate the manifest or verifier;
running a script supplied by an untrusted package is itself code execution.
Use independently trusted release hashes/signatures and host policy before
trusting or executing any packaged code. Verification is a bounded snapshot
read, not proof that files cannot change afterward.

The script's SHA-256 manifest describes the supplied artifact and copied files;
it is not a signature, reproducible-build receipt or qualification result. The
target string is a caller declaration. Native dependency loading, the supplied
binary's behavior and licensing compliance still require independent review.

## Pinned release admission

A separately trusted inspector can run `verify-release` with an explicit
`--package-directory`, detached `--signature` file, independently trusted
`--trusted-public-key` file, `--expected-target`, `--expected-version`, and
`--max-package-bytes`. Do not execute an untrusted packaged inspector to
approve itself. The library exposes the same bounded `sense::release::verify_release`
entry point. No package file is launched or installed.

The key file contains exactly 32 raw Ed25519 public-key bytes, and the signature
file contains exactly 64 raw bytes. A release authority signs the 32-byte
SHA-256 digest returned by `release_signing_digest(package_json_bytes)` using
ordinary Ed25519. The digest covers the literal domain
`qualia:sensing-inspect:release-manifest:v1` followed by NUL, the manifest byte
length as little-endian u64, and the exact manifest bytes. It is not Ed25519ph;
reformatting JSON changes the signed message. Key creation and signing are
external authority operations; this application never reads private keys.

The command strictly verifies the signature, parses the bounded manifest with
duplicate/unknown field refusal, checks the caller's version/target expectations,
then verifies every package file's exact length and SHA-256 under one aggregate
encoded-byte budget. Extra package entries fail closed. The report binds the
signature and package-content check in one protected-root snapshot operation.
The 16 KiB manifest cap and aggregate package budget exclude the fixed 96 bytes
of external key/signature input. Hashing currently allocates one budget-admitted
file at a time; this is a cold deployment operation.

The caller owns public-key trust, release revocation and rollback-resistant
version policy. A matching pin does not establish those independently, qualify
the executable or authorize execution. Protected roots/files and host no-egress
controls remain mandatory throughout verification; checks do not prevent later
file replacement. No signing key or signature was generated, and no release
verification or file access command was executed during implementation.

No packaging script, build, test or application execution was performed for
this draft. A future operator may package a qualified binary, for example:

```powershell
./package.ps1 -ExecutablePath C:\qualified\qualia-sensing-inspect.exe -OutputParent C:\packages -Target x86_64-pc-windows-msvc -ByteBudget 268435456
```

The default package name is `qualia-sensing-inspect-0.1.0-<target>`. Packaging
accepts explicit `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` and
`x86_64-pc-windows-gnu` target declarations. Select the actual binary target;
the declaration does not inspect or qualify its ABI. Native DLL dependencies
are not automatically bundled or resolved. Packaging requires a Windows fixed
local drive and rejects obvious UNC/device/relative/
parent paths, alternate streams, reserved DOS names and reparse-point ancestors.
These checks do not attest the filesystem or defeat hostile concurrent path
replacement; the output and source roots must be protected for the operation.
Hard no-egress enforcement belongs to the host. Filesystem overhead is outside
the encoded-file byte budget. Directory publication/durability and failure
cleanup need Windows/filesystem qualification. A failed or uncertain publication
never triggers deletion of the final package. Only the exact marker-verified
staging directory is eligible for best-effort cleanup.

The source workspace remains outside the production workspace and expects the
isolated sibling Etalon source through the bridge. Dependency locks and native
libraries are not bundled. Before any eventual build, coordinate offline
dependency resolution and prevent ORT feature unification from enabling binary
downloads. `distribution.json` records the package contract, not completed
deployment or compatibility certification.
