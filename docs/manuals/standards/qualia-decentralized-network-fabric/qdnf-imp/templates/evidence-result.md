# Implementation evidence manifest template

## Provenance

- Package/child IDs and claim:
- Evidence author and independent reviewer:
- Date, source revision and reproducible dirty-state fingerprint:
- Target, toolchain, dependencies, features and profile versions:
- Hardware/runtime and test or independent-oracle versions:

## Observation

- Requirement and expected result:
- Exact reproducible command/harness and fixture inputs:
- Workload, seed, time/resource budgets and measurement definitions:
- Observed result, sample count and uncertainty:
- Pass/fail/blocked/not-selected status with rationale:
- Negative/error/cancel/restart cases exercised:
- Raw artifact references, sizes, hashes, retention and access controls:
- Limitations, baseline failures and affected unsupported claims:

## Acceptance

- Integrated source state to which this evidence applies:
- Reviewer assessment and follow-up findings:
- Conditional applicability decision, if any:
- Affected consumer gates and rerun triggers:

Store this manifest under an integrator-selected subdirectory inside qdnf-imp when evidence
exists; registry `evidence` entries use relative manifest paths. Store large/private raw artifacts
in explicitly owned bounded storage and reference them with access/custody information.
A checked box plus a filename proves neither successful behavior nor legal admissibility.
