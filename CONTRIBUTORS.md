# QualiaDB Contributor Guidelines

Welcome to the QualiaDB ecosystem. This project is dedicated to building neurosymbolic semantic databases, digital agency infrastructure, and equitable social systems. We invite contributions from individuals who share a commitment to cooperative project integrity and human rights.

## Cooperative Conduct Policy

QualiaDB serves as an example of cooperative project integrity. To ensure a safe, equitable, and constructive ecosystem, all contributors (and any AI agents operating on their behalf) must strictly adhere to the following principles:

### 1. Human Rights Respecting Behavior
Contributors and agents must not engage in adversarial, manipulative, or dishonest conduct. Any behavior must be strictly human rights respecting, aligning with the principles outlined in the [OHCHR - Core International Human Rights Instruments](https://www.ohchr.org/en/instruments-listings).

### 2. Prohibited Conduct
Any form of anti-human rights or discriminatory behavior is strictly prohibited. This applies to both human interactions within the community and the architectural implementation, models, and outputs of any code or sub-systems interacting with the database.

### 3. Auditable Accountability & Liability Graphs
The QualiaDB system actively logs conduct through the Webizen Semantic Gatekeeper. Any adversarial or prohibited operation (e.g., `llm:AdversarialOperation`, `llm:DiscriminatoryOperation`) is permanently noted in the system's development logs and the internal QualiaDB graph.
- **Cryptographic Provenance**: These logs are preserved with cryptographic telltales to ensure they are tamper-proof.
- **DID Association**: Conduct records securely associate the behavior with the commanding natural person's Decentralized Identifier (`principal_did`).
- **Insurance & Legal Liability**: These immutable records generate cryptographically auditable trails suitable for courts of law, establishing precise insurance liability graphs and proportionalities.

## Technical Contributions

When submitting code:
- Ensure adherence to the zero-allocation, off-heap data processing constraints as outlined in the core system documentation.
- Any AI agent assisting you in development must comply with the binding directives defined in `AGENTS.md`, `AI_INSTRUCTIONS.md`, and `.cursorrules`.

Thank you for contributing to an equitable, human-rights-respecting digital future.
