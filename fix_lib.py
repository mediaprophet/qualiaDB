import re

with open('crates/qualia-client-core/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Comment out quantum pipeline modules
content = content.replace(
    '''pub mod qpu_dispatcher;
pub mod qpu_oracle;
pub mod qpu_pipeline;''',
    '''// pub mod qpu_dispatcher; // TODO: implement qubo_compiler
// pub mod qpu_oracle;
// pub mod qpu_pipeline;''')

with open('crates/qualia-client-core/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)
