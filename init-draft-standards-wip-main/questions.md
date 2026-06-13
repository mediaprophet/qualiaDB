# Questions

- Given that QualiaDB turns RDF into `.q42` using CBOR-LD natively, should we completely remove textual formats (like Turtle or JSON-LD) from the normative parts of these standards, maintaining them only as legacy ingestion pathways?
- Are there specific logic libraries besides `N3Logic` (which is implemented natively inside the engine) that should be explicitly documented as disabled or explicitly excluded to prevent namespace clashes or feature bloat?
- Should we provide SHACL mappings for the LTL temporal operators (Globally, Finally, Next, Until, Release) and Paraconsistent Logic routers?
- Regarding the replacement of 'identities' with 'verifiable claims' and 'human agency': do these replacements align correctly with the ontological models (like `HCAIO`), or are there specific edge cases where legacy systems force us to use the term 'identity'?
