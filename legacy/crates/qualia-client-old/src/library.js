const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const terminal = document.getElementById('terminal');
const bookmarkList = document.getElementById('bookmarkList');

function log(msg, type = '') {
    const div = document.createElement('div');
    if (type) div.className = type;
    const now = new Date();
    const ts = `[${now.getHours().toString().padStart(2,'0')}:${now.getMinutes().toString().padStart(2,'0')}:${now.getSeconds().toString().padStart(2,'0')}]`;
    div.innerText = `${ts} ${msg}`;
    terminal.appendChild(div);
    terminal.scrollTop = terminal.scrollHeight;
}

// Drag & Drop Handlers
dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('dragover');
});

dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('dragover');
});

dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('dragover');
    handleFiles(e.dataTransfer.files);
});

fileInput.addEventListener('change', (e) => {
    handleFiles(e.target.files);
});

async function handleFiles(files) {
    if (!files || files.length === 0) return;
    
    // Check if we are running in Tauri
    const isTauri = window.__TAURI__ != null;
    
    for (const file of files) {
        const lowerName = file.name.toLowerCase();
        const isPdf = lowerName.endsWith('.pdf');
        const isOntology = lowerName.endsWith('.rdf') || lowerName.endsWith('.owl') || lowerName.endsWith('.ttl') || lowerName.includes('-star');

        if (!isPdf && !isOntology) {
            log(`Skipped unsupported file: ${file.name}`, 'error');
            continue;
        }

        log(`Queueing document: ${file.name}`, 'sys');
        
        if (isTauri) {
            try {
                let response;
                if (isPdf) {
                    response = await window.__TAURI__.tauri.invoke('ingest_pdf', { fileName: file.name });
                } else {
                    response = await window.__TAURI__.tauri.invoke('ingest_ontology', { fileName: file.name });
                }
                log(`Processing Complete: ${file.name}`);
                updateBookmarks(response.bookmarks);
            } catch (err) {
                log(`Ingestion Failed: ${err}`, 'error');
            }
        } else {
            // Mock processing for browser dev mode
            log(`Parsing CML/CMLD inline URIs...`);
            setTimeout(() => {
                log(`Mapped to HCAI Ontology & UN Rights.`);
                setTimeout(() => {
                    log(`Compiled ${file.name} to binary .q42`);
                    updateBookmarks([
                        { entity: "Article 12: Right to Privacy", tags: ["UN-HR", "HCAI:Agency", "DataSovereignty-Removed"] },
                        { entity: "Informed Consent Model", tags: ["HCAI:Agreements", "ODRL", "Proxy-Consent"] }
                    ]);
                }, 1000);
            }, 1000);
        }
    }
}

function updateBookmarks(bookmarks) {
    if (!bookmarks || bookmarks.length === 0) return;
    
    // Clear initial state
    if (bookmarkList.querySelector('li').innerText.includes('No bookmarks')) {
        bookmarkList.innerHTML = '';
    }
    
    bookmarks.forEach(bm => {
        const li = document.createElement('li');
        li.className = 'bookmark-item';
        
        const entity = document.createElement('div');
        entity.className = 'bookmark-entity';
        entity.innerText = bm.entity;
        li.appendChild(entity);
        
        bm.tags.forEach(tag => {
            const span = document.createElement('span');
            span.className = 'tag';
            span.innerText = tag;
            li.appendChild(span);
        });
        
        bookmarkList.appendChild(li);
    });
}
