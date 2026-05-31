// Qualia-DB Semantic Drift Data

const SEMANTIC_GRAPH = {
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@graph": [
    // --- AWFUL ---
    {
      "@id": "word:awful",
      "term": "awful",
      "start_year": 1000,
      "end_year": 1499,
      "skos:definition": "Worthy of awe; majestic; commanding profound respect and reverence.",
      "location": "iso3166:GB-ENG (England)",
      "language": "iso639:enm (Middle English)"
    },
    {
      "@id": "word:awful",
      "term": "awful",
      "start_year": 1500,
      "end_year": 1799,
      "skos:definition": "Fearful or terrifying; causing dread.",
      "location": "iso3166:GB (Great Britain)",
      "language": "iso639:en (Early Modern English)"
    },
    {
      "@id": "word:awful",
      "term": "awful",
      "start_year": 1800,
      "end_year": 2026,
      "skos:definition": "Extremely bad; unpleasant; appalling.",
      "location": "Global Anglosphere",
      "language": "iso639:en (Modern English)"
    },

    // --- MEAT ---
    {
      "@id": "word:meat",
      "term": "meat",
      "start_year": 1000,
      "end_year": 1299,
      "skos:definition": "Any solid food in general (as opposed to drink).",
      "location": "iso3166:GB-ENG (England)",
      "language": "iso639:ang (Old English - 'mete')"
    },
    {
      "@id": "word:meat",
      "term": "meat",
      "start_year": 1300,
      "end_year": 2026,
      "skos:definition": "The flesh of an animal used as food.",
      "location": "Global Anglosphere",
      "language": "iso639:en (Modern English)"
    },

    // --- GIRL ---
    {
      "@id": "word:girl",
      "term": "girl",
      "start_year": 1000,
      "end_year": 1399,
      "skos:definition": "A child or young person of either sex.",
      "location": "iso3166:GB-ENG (England)",
      "language": "iso639:enm (Middle English - 'gurle')"
    },
    {
      "@id": "word:girl",
      "term": "girl",
      "start_year": 1400,
      "end_year": 2026,
      "skos:definition": "A female child or young woman.",
      "location": "Global Anglosphere",
      "language": "iso639:en (Modern English)"
    }
  ]
};

// State
let currentWord = 'awful';
let currentYear = 1300;

// DOM Elements
const slider = document.getElementById('time-slider');
const yearLabel = document.getElementById('year-label');
const container = document.getElementById('meaning-container');

// Hash function simulation
function hashString(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash).toString(16).padStart(16, '0');
}

function updateUI() {
    // Force a re-render animation by replacing the node
    const activeData = SEMANTIC_GRAPH["@graph"].find(n => 
        n.term === currentWord && 
        currentYear >= n.start_year && 
        currentYear <= n.end_year
    );

    if (!activeData) return;

    // Simulate the Context vector generation
    const contextStr = \`year:\${currentYear} + loc:\${activeData.location} + lang:\${activeData.language}\`;
    const contextHash = "0x" + hashString(contextStr);
    const subjectHash = "0x" + hashString(activeData["@id"]);

    const cardHTML = \`
        <div class="meaning-card" id="active-card">
            <h2>"\${activeData.term}"</h2>
            <div class="definition">"\${activeData["skos:definition"]}"</div>
            
            <div class="metadata-grid">
                <div class="meta-item">
                    <span>Geographic Boundary</span>
                    <div class="meta-value">\${activeData.location}</div>
                </div>
                <div class="meta-item">
                    <span>Language / Lexicon</span>
                    <div class="meta-value">\${activeData.language}</div>
                </div>
                <div class="meta-item">
                    <span>Subject Hash (Static Anchor)</span>
                    <div class="meta-value" style="color: #fff;">\${subjectHash}</div>
                </div>
                <div class="meta-item">
                    <span>Active Context Vector (Time+Space)</span>
                    <div class="context-hash">\${contextHash}</div>
                </div>
            </div>
        </div>
    \`;

    container.innerHTML = cardHTML;
}

// Global hook for buttons
window.setWord = function(word) {
    currentWord = word;
    document.querySelectorAll('.word-btn').forEach(btn => {
        btn.classList.toggle('active', btn.innerText.includes(word));
    });
    updateUI();
};

// Event Listeners
slider.addEventListener('input', (e) => {
    currentYear = parseInt(e.target.value);
    yearLabel.innerText = currentYear + " CE";
    updateUI();
});

// Init
updateUI();
