# NL → Vibe pair (parses and type-checks)

**Natural language:** If a sensor reading exceeds 85, stage a reified overheat claim, validate it, commit, and publish on `clinic/alerts`.

**Source:** `../12_2_clinic.vibe`

Uses `<<( s p o )>>`, `<< s p o ~ reifier >>`, `graph.stage` / `graph.commit`, `aura.validate`, `pulse.publish`. Requires the matching `capability(...)` list. No Quin overlay.
