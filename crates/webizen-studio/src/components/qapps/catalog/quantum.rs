use super::{AppRoute, Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Quantum Computing â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "qpu-optimizer",
            name: "QPU Optimizer",
            tagline: "Quantum Optimisation",
            desc: "Formulate QUBO/QAOA problems and dispatch to 8 QPU providers (IBM / D-Wave / IonQ \
                   / Rigetti / Azure / Braket / Google / Quantinuum) via the in-process QPU dispatcher.",
            icon: "cpu",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Quantum,
        },
        QApp {
            id: "quantum-dft",
            name: "Quantum DFT Lab",
            tagline: "Quantum Transforms",
            desc: "Discrete Fourier Transform on quantum gate circuits. Integrates with IBM Quantum \
                   API; results back-annotated onto NQuin provenance chains for full auditability.",
            icon: "soundwave",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Quantum,
        },
        QApp {
            id: "qaoa-explorer",
            name: "QAOA Explorer",
            tagline: "Variational Algorithms",
            desc: "Interactive QAOA angle optimiser using SPSA gradient descent. Visualise energy \
                   landscapes, convergence curves, and compare classical vs. quantum solution quality.",
            icon: "sliders",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Quantum,
        },
        QApp {
            id: "qpu-providers",
            name: "QPU Provider Manager",
            tagline: "Quantum Infrastructure",
            desc: "Manage credentials, job quotas, and connectivity for all 8 QPU backends. Monitor \
                   queue depths, gate error rates, and fidelity metrics across providers.",
            icon: "cloud-check",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Quantum,
        },
        QApp {
            id: "nexus",
            name: "Nexus",
            tagline: "Quantum Research Cooperative",
            desc: "Living Research Timeline with LTL causal provenance, cooperative knowledge graph \
                   canvas, epistemic modal-logic claim threads (OP_KNOWS/OP_BELIEVES), native dispatch \
                   for SW alignment, DFT, MCMC, and RK4 â€” all attribution via DID-signed NQuins.",
            icon: "radioactive",
            route: Some(AppRoute::Nexus),
            stat: Stat::Active,
            cat: Cat::Quantum,
        },
    ]
}
