import os

target_dir = r"C:\Projects\qualiaDB\init-draft-standards-wip-main"

html_injection = """
  <section id="qualia-engine-implementation" class="informative">
    <h2>QualiaDB Engine Technical Implementation Note</h2>
    <p>
      In practice, the technical realization of these standards relies on the <strong>QualiaDB engine architecture</strong> to ensure robust, hardware-accelerated, zero-allocation enforcement:
    </p>
    <ul>
      <li><strong>Serialization:</strong> While the standards reference generic Semantic Web forms (RDF, RDF-star), QualiaDB converts these into a high-performance 48-byte binary Super-Quin structure (<code>.q42</code> file format) for execution. For data transmission across the wire, <strong>CBOR-LD</strong> is the primary serialization method.</li>
      <li><strong>Logic & Constraints:</strong> Instead of generic validation, terms and constraints are parsed via a native <strong>N3 Streaming Parser</strong> and enforced using explicit <strong>Deontic Logic</strong> operators (Obligate, Permit, Forbid).</li>
      <li><strong>Conflict & State Management:</strong> CRDT (Lamport Clock LWW) and Paraconsistent Logic routers are used to manage temporally bound states and contradictions (e.g., via Allen Interval Algebra and LTL Semantics) without system-wide failure.</li>
    </ul>
  </section>
</body>
"""

md_injection = """

## QualiaDB Engine Technical Implementation Note
> [!NOTE]
> In practice, the technical realization of these standards relies on the **QualiaDB engine architecture** to ensure robust, hardware-accelerated, zero-allocation enforcement:
> - **Serialization:** While referencing generic Semantic Web forms (RDF, RDF-star), QualiaDB converts these into a high-performance 48-byte binary Super-Quin structure (`.q42` file format) for execution. For data transmission, **CBOR-LD** is the primary serialization method.
> - **Logic & Constraints:** Constraints are parsed via a native **N3 Streaming Parser** and enforced using explicit **Deontic Logic** operators (Obligate, Permit, Forbid).
> - **Conflict & State:** CRDT and Paraconsistent Logic routers manage temporally bound states and contradictions (e.g., via Allen Interval Algebra and LTL Semantics) without system-wide failure.
"""

for root, dirs, files in os.walk(target_dir):
    for file in files:
        filepath = os.path.join(root, file)
        
        # Skip the root ones we already manually edited, and skip the script itself
        if root == target_dir and file in ['HCI-SemWeb-Serialisation.md', 'README.md', 'ecosystemNotes.md', 'changelog.md', 'questions.md']:
            continue
        if file.endswith('.py'):
            continue
            
        if file.endswith('index.html') or file.endswith('old_index.html'):
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            if "id=\"qualia-engine-implementation\"" not in content and "</body>" in content:
                new_content = content.replace("</body>", html_injection)
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(new_content)
                print(f"Injected HTML notes into {filepath}")
                
        elif file.endswith('.md') and file.lower() == 'readme.md':
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            if "QualiaDB Engine Technical Implementation Note" not in content:
                with open(filepath, 'a', encoding='utf-8') as f:
                    f.write(md_injection)
                print(f"Appended MD notes to {filepath}")
