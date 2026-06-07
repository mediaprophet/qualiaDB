/**
 * knowledge-parser.js
 *
 * Builds Anatomy condition/system maps from Turtle knowledge files.
 * Uses Qualia WASM parse_turtle_wasm when available; otherwise a focused JS parser.
 */

const KnowledgeParser = (() => {
  const SYSTEM_IRI_TO_LABEL = {
    "https://qualia.anatomy.example/ontology/organ#CirculatorySystem": "Circulatory (Cardiovascular) System",
    "https://qualia.anatomy.example/ontology/organ#CardiovascularSystem": "Circulatory (Cardiovascular) System",
    "https://qualia.anatomy.example/ontology/organ#RespiratorySystem": "Respiratory System",
    "https://qualia.anatomy.example/ontology/organ#DigestiveSystem": "Digestive System",
    "https://qualia.anatomy.example/ontology/organ#HepatobiliarySystem": "Digestive System",
    "https://qualia.anatomy.example/ontology/organ#NervousSystem": "Nervous System",
    "https://qualia.anatomy.example/ontology/organ#MuscularSystem": "Muscular System",
    "https://qualia.anatomy.example/ontology/organ#SkeletalSystem": "Skeletal System",
    "https://qualia.anatomy.example/ontology/organ#MusculoskeletalSystem": "Skeletal System",
    "https://qualia.anatomy.example/ontology/organ#EndocrineSystem": "Endocrine System",
    "https://qualia.anatomy.example/ontology/organ#MetabolicSystem": "Endocrine System",
    "https://qualia.anatomy.example/ontology/organ#ImmuneLymphaticSystem": "Immune / Lymphatic System",
    "https://qualia.anatomy.example/ontology/organ#IntegumentarySystem": "Integumentary System",
    "https://qualia.anatomy.example/ontology/organ#UrinarySystem": "Urinary (Excretory) System",
    "https://qualia.anatomy.example/ontology/organ#RenalSystem": "Urinary (Excretory) System",
    "https://qualia.anatomy.example/ontology/organ#ReproductiveSystem": "Reproductive System",
    "https://qualia.anatomy.example/ontology/organ#SensorySystem": "Sensory System",
    "https://qualia.anatomy.example/ontology/organ#VestibularSystem": "Vestibular System",
    "https://qualia.anatomy.example/ontology/organ#ExocrineSystem": "Exocrine System",
    "https://qualia.anatomy.example/ontology/organ#EndocannabinoidSystem": "Endocannabinoid System (ECS)",
    "https://qualia.anatomy.example/ontology/organ#EntericNervousSystem": "Enteric Nervous System (ENS)",
    "https://qualia.anatomy.example/ontology/organ#GlymphaticSystem": "Glymphatic System",
    "https://qualia.anatomy.example/ontology/organ#HematologicSystem": "Circulatory (Cardiovascular) System"
  };

  function expandPrefixes(text, prefixes) {
    return text.replace(/\b([a-zA-Z][\w-]*):([\w-]+)/g, (match, prefix, local) => {
      if (prefixes[prefix]) return `${prefixes[prefix]}${local}`;
      return match;
    });
  }

  function parsePrefixes(text) {
    const prefixes = {};
    const lines = text.split(/\r?\n/);
    for (const line of lines) {
      const match = line.match(/@prefix\s+([\w-]+):\s+<([^>]+)>\s*\./i);
      if (match) prefixes[match[1]] = match[2];
    }
    return prefixes;
  }

  function normalizeObject(value) {
    return value.replace(/\s+/g, " ").replace(/\.$/, "").trim();
  }

  function parseTurtleJs(text) {
    const prefixes = parsePrefixes(text);
    const triples = [];
    const blocks = text
      .split(/\.\s*(?=\n|$)/)
      .map(block => block.trim())
      .filter(block => block && !block.startsWith("@prefix") && !block.startsWith("#"));

    for (let block of blocks) {
      block = expandPrefixes(block, prefixes);
      const headMatch = block.match(/^(\S+)\s+(.+)$/s);
      if (!headMatch) continue;

      const subject = headMatch[1];
      let body = headMatch[2].replace(/\s+/g, " ").trim();
      const statements = body.split(/\s*;\s*/);

      statements.forEach(statement => {
        const parts = statement.trim().split(/\s+/);
        if (parts.length < 2) return;
        const predicate = parts[0];
        const object = normalizeObject(parts.slice(1).join(" "));
        triples.push({ subject, predicate, object });
      });
    }

    return triples;
  }

  function parseWithWasm(text, wasmApi) {
    if (!wasmApi?.parse_turtle_wasm) return null;
    const triples = wasmApi.parse_turtle_wasm(text);
    if (!triples || !Array.isArray(triples)) return null;
    return triples.map(t => ({
      subject: t.subject,
      predicate: t.predicate,
      object: normalizeObject(t.object)
    }));
  }

  function systemLabelFromIri(iri) {
    if (SYSTEM_IRI_TO_LABEL[iri]) return SYSTEM_IRI_TO_LABEL[iri];
    const local = iri.split("#").pop() || iri.split("/").pop() || iri;
    return local.replace(/([a-z])([A-Z])/g, "$1 $2");
  }

  function buildConditionMap(triples, source = "turtle") {
    const bySubject = new Map();
    triples.forEach(triple => {
      if (!bySubject.has(triple.subject)) bySubject.set(triple.subject, []);
      bySubject.get(triple.subject).push(triple);
    });

    const conditions = {};
    bySubject.forEach((rows, subject) => {
      const types = rows
        .filter(row => row.predicate.endsWith("#type") || row.predicate.endsWith("/type"))
        .map(row => row.object);
      const isCondition = types.some(t => t.includes("bio#Condition") || t.endsWith("Condition"));
      if (!isCondition) return;

      const labelRow = rows.find(row =>
        row.predicate.endsWith("label") || row.predicate.endsWith("#label")
      );
      const label = labelRow
        ? labelRow.object.replace(/^"(.*)"$/, "$1")
        : subject.split("#").pop() || subject;

      const primaryRow = rows.find(row =>
        row.predicate.includes("hasPrimaryImpactSystem")
      );
      const primarySystem = primaryRow ? systemLabelFromIri(primaryRow.object) : null;
      const impacts = rows
        .filter(row => row.predicate.includes("Impacts"))
        .map(row => row.object.split("#").pop() || row.object);

      conditions[label] = {
        primarySystem,
        ontologyIri: subject,
        impacts,
        source
      };
    });

    return { version: "1.0.0", source, conditions };
  }

  async function parseConditionsTtl(text, wasmApi = null) {
    const triples = parseWithWasm(text, wasmApi) || parseTurtleJs(text);
    return buildConditionMap(triples, wasmApi?.parse_turtle_wasm ? "wasm" : "js");
  }

  return {
    parseTurtleJs,
    parseConditionsTtl,
    buildConditionMap,
    systemLabelFromIri
  };
})();

if (typeof window !== "undefined") {
  window.KnowledgeParser = KnowledgeParser;
}
