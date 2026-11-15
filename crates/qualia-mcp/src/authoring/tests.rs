use super::*;
use serde_json::{json, Value};
use std::io::{Cursor, Write};

#[derive(Default)]
struct FixtureBackend {
    calls: usize,
    failure: bool,
    oversized: bool,
}
impl AuthoringBackend for FixtureBackend {
    fn tools(&self) -> Vec<AuthoringTool> {
        vec![AuthoringTool {
            name: "fixture.echo".into(),
            description: "Explicit test fixture".into(),
            input_schema: json!({"type":"object", "properties":{"message":{"type":"string"}}}),
        }]
    }
    fn call(&mut self, name: &str, args: Value) -> Result<Value, ToolError> {
        assert_eq!(name, "fixture.echo");
        self.calls += 1;
        if self.failure {
            Err(ToolError::new(
                "permission_denied",
                "Native authority is unbound",
            ))
        } else if self.oversized {
            Ok(json!({"payload":"x".repeat(1024 * 1024)}))
        } else {
            Ok(json!({"received":args, "calls":self.calls}))
        }
    }
}
fn initialize(version: &str) -> Value {
    json!({"jsonrpc":"2.0", "id":"init", "method":"initialize", "params":{
        "protocolVersion":version,"capabilities":{},"clientInfo":{"name":"test fixture", "version":"0"}}})
}
fn req(method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":4, "method":method, "params":params})
}
fn call() -> Value {
    req(
        "tools/call",
        json!({"name":"fixture.echo", "arguments":{"message":"test"}}),
    )
}
fn process(server: &mut AuthoringServer<FixtureBackend>, value: Value) -> Value {
    server
        .process(&value.to_string())
        .expect("request response")
}
fn ready(backend: FixtureBackend) -> AuthoringServer<FixtureBackend> {
    let mut server = AuthoringServer::new(backend, ProtocolLimits::default()).unwrap();
    process(&mut server, initialize(AUTHORING_PROTOCOL_VERSION));
    assert!(server
        .process(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
        .is_none());
    assert_eq!(server.lifecycle(), Lifecycle::Ready);
    server
}

#[test]
fn lifecycle_requires_initialize_then_initialized_and_negotiates_only_pinned_version() {
    let mut server =
        AuthoringServer::new(FixtureBackend::default(), ProtocolLimits::default()).unwrap();
    assert_eq!(process(&mut server, call())["error"]["code"], -32002);
    server.process(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert_eq!(server.lifecycle(), Lifecycle::Uninitialized);
    let init = process(&mut server, initialize("future-unsupported-version"));
    assert_eq!(
        init["result"]["protocolVersion"],
        AUTHORING_PROTOCOL_VERSION
    );
    assert_eq!(
        init["result"]["capabilities"],
        json!({"tools":{"listChanged":false}})
    );
    assert_eq!(process(&mut server, call())["error"]["code"], -32002);
    assert_eq!(
        process(&mut server, initialize(AUTHORING_PROTOCOL_VERSION))["error"]["code"],
        -32600
    );
    server.process(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert_eq!(process(&mut server, call())["result"]["isError"], false);
    assert_eq!(server.into_backend().calls, 1);
}

#[test]
fn invalid_initialize_does_not_activate_and_ping_does_not_change_state() {
    let mut server =
        AuthoringServer::new(FixtureBackend::default(), ProtocolLimits::default()).unwrap();
    for params in [
        Value::Null,
        json!({}),
        json!({"protocolVersion":"2025-11-25", "clientInfo":{}, "capabilities":{}}),
    ] {
        assert_eq!(
            process(&mut server, req("initialize", params))["error"]["code"],
            -32602
        );
        assert_eq!(server.lifecycle(), Lifecycle::Uninitialized);
    }
    assert_eq!(
        process(&mut server, req("ping", json!({})))["result"],
        json!({})
    );
    assert_eq!(server.lifecycle(), Lifecycle::Uninitialized);
}

#[test]
fn notifications_never_call_tools_or_respond() {
    let mut server = ready(FixtureBackend::default());
    for method in [
        "tools/call",
        "tools/list",
        "initialize",
        "unknown",
        "notifications/cancelled",
    ] {
        assert!(server
            .process(
                &json!({"jsonrpc":"2.0","method":method,
            "params":{"name":"fixture.echo", "arguments":{}}})
                .to_string()
            )
            .is_none());
    }
    assert_eq!(server.into_backend().calls, 0);
}

#[test]
fn only_string_and_integral_ids_are_accepted_and_echoed() {
    let mut server = ready(FixtureBackend::default());
    for id in [json!("case"), json!(-5), json!(u64::MAX)] {
        let mut request = call();
        request["id"] = id.clone();
        assert_eq!(process(&mut server, request)["id"], id);
    }
    for id in [Value::Null, json!(false), json!(1.5), json!([]), json!({})] {
        let mut request = call();
        request["id"] = id;
        let response = process(&mut server, request);
        assert_eq!(response["error"]["code"], -32600);
        assert!(response["id"].is_null());
    }
    assert_eq!(server.into_backend().calls, 3);
}

#[test]
fn malformed_batch_and_wrong_envelopes_never_dispatch() {
    let mut server = ready(FixtureBackend::default());
    assert_eq!(server.process("{").unwrap()["error"]["code"], -32700);
    for value in [
        json!([]),
        json!([call()]),
        json!(true),
        json!({"id":3,"method":"tools/call","params":{"name":"fixture.echo"}}),
        json!({"jsonrpc":"1.0","id":3,"method":"tools/call"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","result":{}}),
    ] {
        assert_eq!(process(&mut server, value)["error"]["code"], -32600);
    }
    assert_eq!(server.into_backend().calls, 0);
}

#[test]
fn supported_catalog_only_and_unknown_tools_methods_or_bad_args_do_not_dispatch() {
    let mut server = ready(FixtureBackend::default());
    let listing = process(&mut server, req("tools/list", json!({})));
    assert_eq!(listing["result"]["tools"].as_array().unwrap().len(), 1);
    assert_eq!(listing["result"]["tools"][0]["name"], "fixture.echo");
    assert_eq!(
        process(&mut server, req("poet.synthesized", json!({})))["error"]["code"],
        -32601
    );
    for params in [
        json!({"name":"unknown"}),
        json!({"name":"fixture.echo","arguments":[]}),
        json!({}),
        json!([]),
    ] {
        assert_eq!(
            process(&mut server, req("tools/call", params))["error"]["code"],
            -32602
        );
    }
    assert_eq!(
        process(
            &mut server,
            req("tools/list", json!({"cursor":"not-supported"}))
        )["error"]["code"],
        -32602
    );
    assert_eq!(server.into_backend().calls, 0);
}

#[test]
fn backend_domain_errors_are_actual_tool_errors_with_no_success_fallback() {
    let mut server = ready(FixtureBackend {
        failure: true,
        ..Default::default()
    });
    let response = process(&mut server, call());
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["error"]["code"],
        "permission_denied"
    );
    assert_eq!(
        response["result"]["structuredContent"]["error"]["message"],
        "Native authority is unbound"
    );
    assert!(response["error"].is_null());
    assert_eq!(server.into_backend().calls, 1);
}

#[test]
fn successful_backend_object_roundtrips_structured_and_text_output() {
    let mut server = ready(FixtureBackend::default());
    let response = process(&mut server, call());
    let result = &response["result"];
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, result["structuredContent"]);
    assert_eq!(text["received"]["message"], "test");
    assert_eq!(result["isError"], false);
}

#[test]
fn stdio_has_only_valid_newline_protocol_messages_and_notifications_are_silent() {
    let input = format!(
        "{}\r\n{}\n{}\n{}\n",
        initialize(AUTHORING_PROTOCOL_VERSION),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        req("tools/list", json!({})),
        call()
    );
    let mut output = Vec::new();
    serve(
        Cursor::new(input.into_bytes()),
        &mut output,
        FixtureBackend::default(),
        ProtocolLimits::default(),
    )
    .unwrap();
    let text = std::str::from_utf8(&output).unwrap();
    assert!(text.ends_with('\n'));
    let messages: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2]["result"]["isError"], false);
}

#[test]
fn oversized_invalid_utf8_and_unterminated_frames_fail_closed_without_output() {
    for (bytes, expected) in [
        (vec![b'x'; 513], "FrameTooLarge"),
        (vec![255, b'\n'], "InvalidUtf8"),
        (
            br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#.to_vec(),
            "UnterminatedFrame",
        ),
    ] {
        let mut output = Vec::new();
        let error = serve(
            Cursor::new(bytes),
            &mut output,
            FixtureBackend::default(),
            ProtocolLimits {
                max_message_bytes: 512,
                max_id_bytes: 16,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(format!("{error:?}").contains(expected));
        assert!(output.is_empty());
    }
    let mut output = Vec::new();
    serve(
        Cursor::new(Vec::<u8>::new()),
        &mut output,
        FixtureBackend::default(),
        ProtocolLimits::default(),
    )
    .unwrap();
    assert!(output.is_empty());
}

#[test]
fn result_budget_failure_does_not_invent_success_and_mentions_operation_state() {
    let mut server = ready(FixtureBackend {
        oversized: true,
        ..Default::default()
    });
    let response = process(&mut server, call());
    assert_eq!(response["error"]["code"], -32603);
    assert!(response["result"].is_null());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("operation state"));
    assert_eq!(server.into_backend().calls, 1);
}

#[test]
fn transport_propagates_safe_io_kind_and_never_prints_error_text() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "private fixture detail",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = serve(
        Cursor::new(format!("{}\n", initialize(AUTHORING_PROTOCOL_VERSION))),
        FailingWriter,
        FixtureBackend::default(),
        ProtocolLimits::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "MCP authoring transport failed: Io(BrokenPipe)"
    );
}

#[test]
fn duplicate_envelope_or_nested_argument_keys_are_rejected_before_dispatch() {
    let mut server = ready(FixtureBackend::default());
    for line in [
        r#"{"jsonrpc":"2.0","id":1,"id":2,"method":"tools/call","params":{"name":"fixture.echo"}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"fixture.echo","arguments":{"revision":1,"revision":2}}}"#,
    ] {
        assert_eq!(server.process(line).unwrap()["error"]["code"], -32700);
    }
    assert_eq!(server.into_backend().calls, 0);
}

#[test]
fn initialized_and_tools_list_accept_protocol_metadata_but_not_task_calls() {
    let mut server =
        AuthoringServer::new(FixtureBackend::default(), ProtocolLimits::default()).unwrap();
    process(&mut server, initialize(AUTHORING_PROTOCOL_VERSION));
    server.process(&json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{"_meta":{"fixture":"test"}}}).to_string());
    assert_eq!(server.lifecycle(), Lifecycle::Ready);
    assert!(process(
        &mut server,
        req("tools/list", json!({"_meta":{"fixture":"test"}}))
    )["result"]["tools"]
        .is_array());
    let mut task_call = call();
    task_call["params"]["task"] = json!({"ttl":1000});
    assert_eq!(process(&mut server, task_call)["error"]["code"], -32602);
    assert_eq!(server.into_backend().calls, 0);
}

#[test]
fn invalid_or_duplicate_catalog_is_rejected_and_empty_real_catalog_is_honest() {
    struct Catalog(Vec<AuthoringTool>);
    impl AuthoringBackend for Catalog {
        fn tools(&self) -> Vec<AuthoringTool> {
            self.0.clone()
        }
        fn call(&mut self, _: &str, _: Value) -> Result<Value, ToolError> {
            panic!("must never dispatch")
        }
    }
    let tool = FixtureBackend::default().tools().remove(0);
    for tools in [
        vec![tool.clone(), tool.clone()],
        vec![AuthoringTool {
            name: "bad name".into(),
            ..tool.clone()
        }],
        vec![AuthoringTool {
            input_schema: Value::Null,
            ..tool.clone()
        }],
    ] {
        assert!(matches!(
            AuthoringServer::new(Catalog(tools), ProtocolLimits::default()),
            Err(ProtocolError::InvalidCatalog)
        ));
    }
    assert!(matches!(
        AuthoringServer::new(
            Catalog(vec![tool]),
            ProtocolLimits {
                max_tools: 0,
                ..Default::default()
            }
        ),
        Err(ProtocolError::InvalidCatalog)
    ));
    let mut server = AuthoringServer::new(Catalog(vec![]), ProtocolLimits::default()).unwrap();
    server.process(&initialize(AUTHORING_PROTOCOL_VERSION).to_string());
    server.process(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let listed = server
        .process(&req("tools/list", json!({})).to_string())
        .unwrap();
    assert_eq!(listed["result"]["tools"], json!([]));
}

#[test]
fn maximum_frame_and_escaped_id_fit_the_configured_response_budget() {
    let limits = ProtocolLimits {
        max_message_bytes: 512,
        max_id_bytes: 16,
        ..Default::default()
    };
    let mut ping = req("ping", json!({}));
    ping["id"] = json!("\u{0}".repeat(16));
    let mut input = ping.to_string();
    input.push_str(&" ".repeat(512 - input.len()));
    input.push('\n');
    let mut output = Vec::new();
    serve(
        Cursor::new(input),
        &mut output,
        FixtureBackend::default(),
        limits,
    )
    .unwrap();
    assert!(output.len() <= 513);
    let response: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(response["id"], ping["id"]);
}
