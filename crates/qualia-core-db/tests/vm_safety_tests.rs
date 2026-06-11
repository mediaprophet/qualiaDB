use qualia_core_db::webizen_bytecode::{execute_program, VmError};
use qualia_core_db::NQuin;

fn make_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

#[test]
fn test_truncated_bytecode() {
    let db = [make_quin(1, 2, 3)];
    let mut out = [NQuin::default(); 10];

    // OP_MATCH_SUBJECT is 0x0A or something? Wait, what are the opcodes?
    // Let me check mini_parser for exact opcodes if needed, but I can just use the byte value.
    // Actually, let's use compile_ntriples_to_bytecode to get valid bytecode and then truncate it.
    let mut prog = [0u8; 1024];
    qualia_core_db::mini_parser::compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog)
        .unwrap();

    // Truncate the program in the middle of an operand
    // The first instruction is usually OP_MATCH_SUBJECT (1 byte) + 8 byte operand
    // If we pass only the first 5 bytes, it should return InvalidProgram
    let truncated = &prog[0..5];

    let res = execute_program(truncated, &db, &mut out);
    assert_eq!(res, Err(VmError::InvalidProgram));
}

#[test]
fn test_invalid_opcode() {
    let db = [make_quin(1, 2, 3)];
    let mut out = [NQuin::default(); 10];

    // Create a program with an invalid opcode. 0xFF is likely invalid.
    let prog = [0xFF, 0, 0, 0, 0, 0, 0, 0, 0];

    let res = execute_program(&prog, &db, &mut out);
    assert_eq!(res, Err(VmError::InvalidProgram));
}

#[test]
fn test_output_buffer_full() {
    let db = [make_quin(1, 2, 3), make_quin(4, 5, 6)];
    let mut out = [NQuin::default(); 1]; // Buffer can only hold 1 item

    let mut prog = [0u8; 1024];
    // Wildcard query matches everything
    qualia_core_db::mini_parser::compile_ntriples_to_bytecode(b"?s ?p ?o", &mut prog).unwrap();

    let res = execute_program(&prog, &db, &mut out);
    assert_eq!(res, Err(VmError::OutputBufferFull));
}

#[test]
fn test_scalar_match_logic() {
    // We will test if the VM correctly evaluates the MATCH opcodes
    let db = [
        make_quin(100, 200, 300),
        make_quin(100, 200, 400),
        make_quin(101, 201, 301),
    ];
    let mut out = [NQuin::default(); 10];

    // Construct bytecode manually or using compiler
    // Let's use the compiler since it's safer
    let mut prog = [0u8; 1024];
    // This requires knowing how q_hash works, but we can just use strings and match the hashed strings
    // But since make_quin takes raw u64, we can just compile a string query and make_quin with hashed strings.
    let alice = qualia_core_db::q_hash("Alice");
    let knows = qualia_core_db::q_hash("knows");
    let bob = qualia_core_db::q_hash("Bob");

    let db_str = [
        make_quin(alice, knows, bob),
        make_quin(alice, knows, qualia_core_db::q_hash("Carol")),
    ];

    qualia_core_db::mini_parser::compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog)
        .unwrap();

    let (match_count, cycles) = execute_program(&prog, &db_str, &mut out).unwrap();
    assert_eq!(match_count, 1);
    assert!(cycles > 0);
    assert_eq!(out[0].subject, alice);
    assert_eq!(out[0].object, bob);
}

#[test]
fn test_mcp_tool_call_allocation_firewall() {
    // 1. Initialize the DHAT profiler to watch the memory space
    let _profiler = dhat::Profiler::builder().testing().build();

    // 2. Mock a highly complex, deeply nested parameters byte stream
    let simulated_payload = b"{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"query_graph\",\"arguments\":{\"query\":\"MATCH (s)-[p]->(o) WHERE s=did:q42:human\",\"sanctuary_override\":\"MISSING\"}}}";

    // 3. Process the frame through the mcp endpoint execution line
    let result =
        unsafe { qualia_core_db::mcp_server::parse_and_evaluate_mcp_stream(simulated_payload) };

    // 4. Assert that the fiduciary engine successfully caught the violation
    assert!(result.is_err());

    // 5. Invariant Assertion: Ensure no global allocations occurred inside tools/call processing
    let stats = dhat::HeapStats::get();
    assert_eq!(
        stats.curr_blocks, 0,
        "Fiduciary Failure: Dynamic heap memory allocated within the tool execution path."
    );
}
