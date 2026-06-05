# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-06-05

### Added
- **Cooperative Conduct Policy**: Implemented a strict policy against adversarial, manipulative, and/or dishonest conduct by AI agents. This project serves as a cooperative projects example. Any such conduct will be noted in the permanent record of the project's development.
- **QualiaDB Adversarial Conduct Tracking**: Added `AdversarialConductRecord` and `LLM_RULE_NO_ADVERSARIAL_CONDUCT` in the QualiaDB system (`llm_agent.rs`) to track and permanently log any violations of the cooperative conduct policy.
- **DID Association & Court-Auditable Liability Graphs**: The conduct log now associates adversarial behavior (including specifically anti-human rights or discriminatory actions) directly with the commanding natural person's DID (`principal_did`). These logs incorporate cryptographic provenance to serve as tamper-proof evidence for court-of-law auditing, directly mapping violations to insurance liability graphs with specific proportional weights.
