import re

path = 'C:/Projects/webizen-browser/webizen-desktop/src/runtime.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

pattern = r'''            let mut file = match std::fs::OpenOptions::new\(\)
                \.create\(true\)
                \.read\(true\)
                \.write\(true\)
                \.open\(&volume_path\)
            \{
                Ok\(f\) => f,
                Err\(err\) => \{
                    metrics\.note_write_failure\(\);
                    let _ = emit_ledger_health\(&app_handle, &metrics\);
                    eprintln!\(\"failed to open diffusion Q42 \{\}: \{\}\", volume_path\.display\(\), err\);
                    return;
                \}
            \};.*?let mut flush_block = \|buf: &mut Vec<NQuin>, epoch: u64\| \{.*?;
            \};'''

replacement = '''            let mut appender = match qualia_core_db::q42_volume::StreamingVolumeAppender::new(&volume_path) {
                Ok(a) => a,
                Err(err) => {
                    metrics.note_write_failure();
                    let _ = emit_ledger_health(&app_handle, &metrics);
                    eprintln!("failed to init streaming appender for {}: {}", volume_path.display(), err);
                    return;
                }
            };

            use qualia_core_db::{QUINS_PER_BLOCK, NQuin};

            let mut block_buffer = Vec::with_capacity(QUINS_PER_BLOCK);
            let mut last_persisted_epoch = 0u64;

            let mut flush_block = |buf: &mut Vec<NQuin>, epoch: u64| {
                if buf.is_empty() {
                    return;
                }
                if let Err(e) = appender.append_block(epoch, buf) {
                    eprintln!("Failed to append block to Q42: {}", e);
                }
                buf.clear();
            };'''

content = re.sub(pattern, replacement, content, flags=re.DOTALL)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
