import { useState } from 'react';
import { BookOpen, Cpu, Database, Lock, MessageSquare, Wallet, Zap, Globe, Shield, Activity } from 'lucide-react';

interface Example {
  title: string;
  description: string;
  icon: any;
  code: string;
  category: string;
}

const EXAMPLES: Example[] = [
  {
    title: 'Semantic Graph Query',
    description: 'Query the local semantic graph using SPARQL-like syntax',
    icon: Database,
    category: 'Graph Engine',
    code: `// Query all patients in the graph
SELECT ?s ?p ?o
WHERE {
  ?s a q42:Patient .
  ?s q42:hasName ?o .
}
LIMIT 10`,
  },
  {
    title: 'Local LLM Inference',
    description: 'Run local LLM inference without external dependencies',
    icon: Cpu,
    category: 'LLM',
    code: `// Run inference with local model
qualia infer "What is consciousness?" --model phi3-mini

// Interactive chat mode
qualia chat --model gemma2-2b`,
  },
  {
    title: 'QPU Quantum Optimization',
    description: 'Offload NP-hard optimization to quantum processors',
    icon: Zap,
    category: 'QPU',
    code: `// Solve TSP using quantum annealing
qualia qpu optimize --problem tsp --shots 100

// Run DFT ground state calculation
qualia qpu dft --molecule benzene`,
  },
  {
    title: 'Ontology Ingestion',
    description: 'Ingest and query medical/ontological data',
    icon: Globe,
    category: 'Ontology',
    code: `// Import SNOMED-CT ontology
qualia resources import-ontology snomed-ct

// Ingest custom RDF data
qualia ingest data/patient.ttl output.q42`,
  },
  {
    title: 'Wallet Operations',
    description: 'Manage multi-seed cryptocurrency wallets',
    icon: Wallet,
    category: 'Identity',
    code: `// Generate new seed
qualia wallet generate-seed

// Derive wallets from seed
qualia wallet derive --seed <mnemonic>

// Check balances
qualia wallet balance`,
  },
  {
    title: 'SHACL Validation',
    description: 'Validate data against SHACL constraint shapes',
    icon: Shield,
    category: 'Validation',
    code: `// Compile SHACL profile
qualia profile compile medical.jsonld medical.qchk

// Ingest with validation
qualia ingest --profile medical.qchk data.ttl output.q42`,
  },
  {
    title: 'Neuro-Chat with Governance',
    description: 'Chat with LLM agent with Sentinel governance',
    icon: MessageSquare,
    category: 'LLM',
    code: `// Start governed chat session
qualia chat --governed

// Intent validation is automatic
// Provenance citations are required`,
  },
  {
    title: 'Quantum Biology',
    description: 'Analyze biomolecular systems using quantum computing',
    icon: Activity,
    category: 'QPU',
    code: `// Analyze protein folding
qualia qpu biology --protein hemoglobin

// Quantum chemistry calculation
qualia qpu chemistry --reaction photosynthesis`,
  },
];

const CATEGORIES = ['All', 'Graph Engine', 'LLM', 'QPU', 'Ontology', 'Identity', 'Validation'];

export default function Examples() {
  const [selectedCategory, setSelectedCategory] = useState('All');
  const [selectedExample, setSelectedExample] = useState<Example | null>(null);

  const filteredExamples = selectedCategory === 'All'
    ? EXAMPLES
    : EXAMPLES.filter(ex => ex.category === selectedCategory);

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="glass-panel">
        <div className="flex items-center gap-3 mb-6">
          <BookOpen className="w-6 h-6 text-[#00f0ff]" />
          <h1 className="text-2xl font-bold text-white">Comprehensive Capabilities Examples</h1>
        </div>
        <p className="text-gray-400 mb-6">
          Explore all capabilities of QualiaDB through interactive examples. This technical demonstration
          includes semantic graph querying, local LLM inference, quantum computing, ontology management,
          identity/wallet operations, and SHACL validation.
        </p>

        <div className="flex gap-2 mb-6 flex-wrap">
          {CATEGORIES.map(cat => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              className={`px-4 py-2 rounded-lg font-bold text-sm transition-all ${
                selectedCategory === cat
                  ? 'bg-[#00f0ff] text-black'
                  : 'bg-white/5 text-gray-300 hover:bg-white/10'
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredExamples.map((example, index) => (
            <div
              key={index}
              onClick={() => setSelectedExample(example)}
              className="glass-panel p-4 cursor-pointer hover:border-[#00f0ff]/50 transition-all"
            >
              <div className="flex items-start gap-3 mb-3">
                <div className="p-2 bg-[#00f0ff]/10 rounded-lg">
                  <example.icon className="w-5 h-5 text-[#00f0ff]" />
                </div>
                <div className="flex-1">
                  <h3 className="text-white font-semibold mb-1">{example.title}</h3>
                  <span className="text-xs text-[#ffd700] font-mono">{example.category}</span>
                </div>
              </div>
              <p className="text-sm text-gray-400">{example.description}</p>
            </div>
          ))}
        </div>
      </div>

      {selectedExample && (
        <div className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="glass-panel max-w-4xl w-full max-h-[80vh] overflow-y-auto">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-[#00f0ff]/10 rounded-lg">
                  <selectedExample.icon className="w-6 h-6 text-[#00f0ff]" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-white">{selectedExample.title}</h2>
                  <span className="text-sm text-[#ffd700] font-mono">{selectedExample.category}</span>
                </div>
              </div>
              <button
                onClick={() => setSelectedExample(null)}
                className="text-gray-400 hover:text-white transition-colors"
              >
                ✕
              </button>
            </div>

            <p className="text-gray-400 mb-4">{selectedExample.description}</p>

            <div className="bg-black/50 border border-white/10 rounded-lg p-4 font-mono text-sm text-gray-300 overflow-x-auto">
              <pre>{selectedExample.code}</pre>
            </div>

            <div className="mt-6 flex gap-4">
              <button
                onClick={() => {
                  navigator.clipboard.writeText(selectedExample.code);
                  alert('Code copied to clipboard!');
                }}
                className="bg-[#00f0ff]/10 text-[#00f0ff] border border-[#00f0ff]/30 px-4 py-2 rounded-lg font-bold hover:bg-[#00f0ff]/20 transition-all"
              >
                Copy Code
              </button>
              <button
                onClick={() => setSelectedExample(null)}
                className="bg-white/5 text-gray-300 border border-white/10 px-4 py-2 rounded-lg font-bold hover:bg-white/10 transition-all"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}