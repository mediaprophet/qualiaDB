import re

with open('crates/qualia-client-core/src/api.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Remove tauri imports
content = re.sub(r'use tauri::.*?;', '', content)
content = re.sub(r'use tauri::.*?\n', '', content)

# Remove #[tauri::command]
content = re.sub(r'#\[tauri::command\]\n', '', content)

# Find all function definitions and rewrite their signatures
def fix_func_signature(match):
    prefix = match.group(1)
    func_name = match.group(2)
    args_str = match.group(3)
    rest = match.group(4)
    
    # Process args
    args = []
    if args_str.strip():
        # simple split by comma
        split_args = [a.strip() for a in args_str.split(',') if a.strip()]
        for a in split_args:
            # Drop tauri types
            if 'tauri::AppHandle' in a or 'tauri::State' in a or 'State<AppState>' in a or 'tauri::Window' in a:
                continue
            args.append(a)
    
    new_args = ', '.join(args)
    return f'{prefix}{func_name}({new_args}){rest}'

content = re.sub(r'(pub\s+(?:async\s+)?fn\s+)([a-zA-Z0-9_]+)\s*\((.*?)\)(\s*(?:->\s*[^\{]+)?\s*\{)', fix_func_signature, content, flags=re.DOTALL)

# Find functions using state and insert let state = APP_STATE.get().unwrap();
funcs_using_state = [
    'list_installed_apps', 'verify_and_install_app', 'get_config', 'save_config', 
    'download_and_vectorize', 'download_model', 'cancel_download', 'daemon_status', 
    'get_tax_suite', 'save_tax_suite', 'dispatch_tax_payment', 'ingest_literature', 
    'ingest_ontology', 'get_tokens', 'add_token', 'remove_token', 'get_directory_actors', 
    'add_directory_actor', 'get_front_doors', 'add_front_door', 'get_delegation_rules', 
    'add_delegation_rule', 'get_engine_telemetry', 'save_directory_state'
]

for fn in funcs_using_state:
    # Need to match: pub fn/async fn func_name(args) { ...
    # And we'll insert right after {
    pattern = r'(pub\s+(?:async\s+)?fn\s+' + fn + r'\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{)'
    replacement = r'\g<1>\n    let state = crate::state::APP_STATE.get().unwrap();'
    content = re.sub(pattern, replacement, content)

# Replace app.emit_all with download_events.send
content = content.replace('let _ = app.emit_all("download-progress", &payload);', 'let _ = state.download_events.send(payload.clone());')
content = content.replace('let _ = app.emit_all("download-progress", &processing_payload);', 'let _ = state.download_events.send(processing_payload.clone());')
content = content.replace('let _ = app.emit_all("download-progress", &done_payload);', 'let _ = state.download_events.send(done_payload.clone());')
content = content.replace('let _ = app.emit_all("ingestion-complete", payload);', '/* TODO: remove ingestion-complete */')
content = content.replace('tauri::async_runtime::spawn', 'tokio::spawn')

with open('crates/qualia-client-core/src/api.rs', 'w', encoding='utf-8') as f:
    f.write(content)
