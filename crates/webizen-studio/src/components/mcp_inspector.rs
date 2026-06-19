use dioxus::prelude::*;

#[component]
pub fn McpInspector() -> Element {
    rsx! {
        div { style: "display: flex; height: 100vh; background: #282c34; color: #abb2bf; font-family: 'Consolas', monospace;",
            // Sidebar
            div { style: "width: 300px; border-right: 1px solid #3e4451; display: flex; flex-direction: column;",
                div { style: "padding: 16px; background: #21252b; font-weight: bold; color: #e5c07b;", "MCP Server Registry" }
                div { style: "padding: 10px 16px; cursor: pointer; background: #3e4451; color: #fff;", "▶ GitHub Tools [Local]" }
                div { style: "padding: 10px 16px; cursor: pointer;", "▶ Jira MCP [Remote]" }
                div { style: "padding: 10px 16px; cursor: pointer;", "▶ Qualia Knowledge Graph" }
            }
            // Main content
            div { style: "flex: 1; display: flex; flex-direction: column;",
                div { style: "padding: 16px; background: #21252b; display: flex; gap: 16px; border-bottom: 1px solid #3e4451;",
                    button { style: "background: #98c379; color: #282c34; border: none; padding: 6px 12px; border-radius: 4px; font-weight: bold; cursor: pointer;", "Execute Call" }
                    button { style: "background: #e06c75; color: #282c34; border: none; padding: 6px 12px; border-radius: 4px; font-weight: bold; cursor: pointer;", "Disconnect" }
                }
                div { style: "padding: 20px; flex: 1; overflow-y: auto;",
                    h2 { style: "color: #61afef; margin-top: 0;", "GitHub Tools / get_issue" }
                    div { style: "background: #1e2227; padding: 16px; border-radius: 6px; margin-bottom: 20px;",
                        div { style: "color: #d19a66; margin-bottom: 8px;", "Input Schema:" }
                        pre { style: "margin: 0; color: #98c379;", "{{\n  \"repo\": \"string\",\n  \"issue_number\": \"integer\"\n}}" }
                    }
                    div { style: "background: #1e2227; padding: 16px; border-radius: 6px;",
                        div { style: "color: #d19a66; margin-bottom: 8px;", "Live Traffic Trace:" }
                        div { style: "color: #c678dd; margin-bottom: 4px;", "[14:20:01] -> JSON-RPC Request: id=42, method=tools/call" }
                        div { style: "color: #56b6c2; padding-left: 16px;", "params: {{ name: 'get_issue', args: {{ repo: 'qualia/core', issue_number: 12 }} }}" }
                        div { style: "color: #98c379; margin-top: 8px; margin-bottom: 4px;", "[14:20:02] <- JSON-RPC Response: id=42" }
                        div { style: "color: #abb2bf; padding-left: 16px;", "result: {{ content: [{{ type: 'text', text: 'Issue title: Fix LTL semantics...' }}] }}" }
                    }
                }
            }
        }
    }
}
