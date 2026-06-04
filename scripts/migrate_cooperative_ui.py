import os
import re
from pathlib import Path

source_dir = Path(r"C:\Projects\qualiaDB\Local_LIbraries\other_coop_pages")
target_dir = Path(r"C:\Projects\qualiaDB\docs\social")
blank_template = target_dir / "blank_template.html"

# Load the base wrapper from blank_template
with open(blank_template, 'r', encoding='utf-8') as f:
    template_content = f.read()

# Find where to inject content in the blank template
# We look for the start of the 'content' block in Pages framework
# Typically <div class="content"> or <div class="container-fluid container-fixed-lg">
inject_marker = '<!-- START CONTAINER FLUID -->'
if inject_marker not in template_content:
    inject_marker = '<div class="content">'

parts = template_content.split(inject_marker, 1)
if len(parts) < 2:
    print("Could not find injection point in blank_template.html")
    exit(1)

template_head = parts[0] + inject_marker + "\n<div class='container-fluid container-fixed-lg'>\n"
template_tail = "</div>\n" + parts[1]

# Tailwind to Bootstrap replacements mapping
replacements = {
    r'glass-strong rounded-3xl': 'card card-default shadow-sm rounded',
    r'glass rounded-3xl': 'card card-transparent rounded',
    r'bg-zinc-950 text-zinc-200': '', # Let bootstrap handle body
    r'text-emerald-400': 'text-success',
    r'text-amber-400': 'text-warning',
    r'text-sky-400': 'text-info',
    r'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-5': 'row',
    r'grid grid-cols-2 md:grid-cols-4 gap-4': 'row',
    r'grid md:grid-cols-3 gap-6': 'row',
    r'flex items-center justify-between': 'd-flex justify-content-between align-items-center',
    r'flex items-center gap-x-2': 'd-flex align-items-center gap-2',
    r'font-semibold text-lg': 'h5 font-weight-bold',
    r'font-display text-5xl': 'h1 font-weight-bold',
    r'px-6 py-3 border border-white/20': 'btn btn-outline-primary',
    r'px-6 py-3 bg-gradient-to-r.*': 'btn btn-primary',
    r'<dialog id="([^"]+)"': r'<div class="modal fade" id="\1"',
    r'</dialog>': r'</div>'
}

def convert_to_bootstrap(html_str):
    res = html_str
    for pattern, rep in replacements.items():
        res = re.sub(pattern, rep, res)
    return res

for file_path in source_dir.glob("*.html"):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Extract just the body/content part, ignoring Tailwind head and old nav
    # The old pages usually have <div class="max-w-screen-2xl mx-auto px-8 pt-8 pb-16">
    start_match = re.search(r'<div class="max-w-screen-2xl[^>]*>', content)
    
    if start_match:
        start_idx = start_match.end()
        # Find the end (usually before <dialog> or <script> or </body>)
        end_idx = content.rfind('<script>')
        if end_idx == -1:
            end_idx = content.rfind('</body>')
            
        inner_content = content[start_idx:end_idx].strip()
        
        # Inject ontology logic into project-detail
        if file_path.name == 'project-detail.html':
            ontology_html = """
            <div class="row mb-4">
                <div class="col-md-12">
                    <div class="card card-default">
                        <div class="card-header"><div class="card-title">N3 Ontology Evaluation</div></div>
                        <div class="card-body">
                            <div class="row">
                                <div class="col-md-4">
                                    <h5 class="text-success">Approach Type</h5>
                                    <p class="font-weight-bold">qp:RiskyApproach</p>
                                    <p class="small hint-text">Expected Multiplier: <strong>12.0x</strong> (per cooperative-evaluation.n3)</p>
                                </div>
                                <div class="col-md-4">
                                    <h5 class="text-warning">Technical Debt Level</h5>
                                    <p class="font-weight-bold">High (0.8)</p>
                                    <p class="small hint-text">qp:technicalDebtLevel discounting applied.</p>
                                </div>
                                <div class="col-md-4">
                                    <h5 class="text-info">Adjusted Multiplier</h5>
                                    <p class="font-weight-bold h3">9.6x</p>
                                    <p class="small hint-text">12.0x - (12.0x * 0.8 * 0.3) mathematical product.</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            """
            inner_content = ontology_html + inner_content

        # Convert to bootstrap
        inner_content = convert_to_bootstrap(inner_content)
        
        final_html = template_head + inner_content + template_tail
        
        out_path = target_dir / file_path.name
        with open(out_path, 'w', encoding='utf-8') as out_f:
            out_f.write(final_html)
        print(f"Migrated {file_path.name}")
    else:
        print(f"Skipped {file_path.name} (Could not find content bounds)")

print("Bulk migration complete.")
