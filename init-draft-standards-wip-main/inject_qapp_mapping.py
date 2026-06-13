import os

target_dir = r"C:\Projects\qualiaDB\init-draft-standards-wip-main"

mappings = {
    "cml": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Contextual Workspace QApp & Ontology Hub:** The contextual markup proposed here maps directly to the 56-bit `context` hash inside QualiaDB's 48-byte `NQuin`. This allows Webizen Desktop to support deep, bidirectional transclusion (deep bidirectional) across infinite graphs natively without data duplication.\n""",
    "SemBookmarks": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Contextual Workspace QApp & Graph Explorer:** Semantic bookmarks leverage the `context` field of the `NQuin` to slice and index multidimensional transclusions securely across the Webizen Studio canvas.\n""",
    "DigitalBirthRecord": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Credential Manager & Wallet QApp:** The Digital Birth Record maps into the wallet interface, managed via `did:q42` topological pointers and Ed25519 Author-Scoped Merkle Roots instead of legacy centralized registries.\n""",
    "biometrics": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Credential Manager & Wallet QApp:** Biometric claims are anchored as verifiable claims in the user's sovereign wallet, ensuring cryptographic separation of seeds and persona contexts.\n""",
    "agreements": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Smart Contract & Graph Explorer QApp:** Executable agreements rely on the `SuspendedTransactionQueue` to capture multi-party (M:N) signatures for ratification within the Webizen Desktop environment.\n""",
    "rights": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Smart Contract QApp:** Human rights topologies map to Deontic Logic constraints (Obligate, Permit, Forbid) enforced across sovereign agreements.\n""",
    "ADP": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **LLM Hub & Chat Graph QApp:** Agent discovery feeds into the Epistemic Logic matrix (Knowledge/Belief certainty scoring) within the Chat Graph, safely bridging interactions to native models.\n""",
    "DOA": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **Directory & Chat Graph QApp:** Describes agent boundaries within the Directory QApp and informs the logic bounds for local GGUF models in the LLM Hub.\n""",
    "PC": """\n## QApp Architecture Mapping\n> [!NOTE]\n> **QApp Vault & Capability Manager:** Permissive Commons routing utilizes Paraconsistent Logic to isolate contradictory rules without crashing the host, managed via `.qchk` capability profiles.\n"""
}

for root, dirs, files in os.walk(target_dir):
    dir_name = os.path.basename(root)
    if dir_name in mappings:
        for file in files:
            if file.lower() == 'readme.md':
                filepath = os.path.join(root, file)
                with open(filepath, 'a', encoding='utf-8') as f:
                    f.write(mappings[dir_name])
                print(f"Injected QApp Mapping to {filepath}")
            elif file.endswith('index.html') and not os.path.exists(os.path.join(root, 'README.md')):
                # Fallback to HTML if no README
                filepath = os.path.join(root, file)
                with open(filepath, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                html_injection = mappings[dir_name].replace("## QApp Architecture Mapping\n> [!NOTE]\n> ", "<h3>QApp Architecture Mapping</h3><p><strong>Note:</strong> ").replace("\n", "</p>")
                
                if "QApp Architecture Mapping" not in content and "</body>" in content:
                    new_content = content.replace("</body>", f"<section class='informative'>{html_injection}</section>\n</body>")
                    with open(filepath, 'w', encoding='utf-8') as f:
                        f.write(new_content)
                    print(f"Injected HTML QApp Mapping to {filepath}")
