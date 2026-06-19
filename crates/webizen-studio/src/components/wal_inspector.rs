use dioxus::prelude::*;

#[component]
pub fn WalInspector() -> Element {
    rsx! {
        div { style: "padding: 24px; background: #fafafa; color: #333; height: 100%; font-family: 'Inter', sans-serif;",
            h1 { style: "margin-top: 0; color: #111;", "Write-Ahead Log (WAL) Inspector" }

            div { style: "display: flex; gap: 12px; margin-bottom: 24px;",
                div { style: "padding: 16px; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); flex: 1;",
                    div { style: "font-size: 12px; color: #666; text-transform: uppercase;", "Current LSN" }
                    div { style: "font-size: 24px; font-weight: 700;", "0x8A4B_F192" }
                }
                div { style: "padding: 16px; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); flex: 1;",
                    div { style: "font-size: 12px; color: #666; text-transform: uppercase;", "Unflushed Bytes" }
                    div { style: "font-size: 24px; font-weight: 700; color: #d97706;", "4.2 MB" }
                }
                div { style: "padding: 16px; background: white; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); flex: 1;",
                    div { style: "font-size: 12px; color: #666; text-transform: uppercase;", "Sync Mode" }
                    div { style: "font-size: 24px; font-weight: 700; color: #059669;", "O_DIRECT" }
                }
            }

            h3 { "Recent Transactions" }
            table { style: "width: 100%; border-collapse: collapse; background: white; box-shadow: 0 1px 3px rgba(0,0,0,0.1); border-radius: 8px; overflow: hidden;",
                thead { style: "background: #f1f5f9; border-bottom: 1px solid #e2e8f0;",
                    tr {
                        th { style: "padding: 12px 16px; text-align: left; font-size: 14px;", "TXID" }
                        th { style: "padding: 12px 16px; text-align: left; font-size: 14px;", "Opcode" }
                        th { style: "padding: 12px 16px; text-align: left; font-size: 14px;", "Payload" }
                        th { style: "padding: 12px 16px; text-align: left; font-size: 14px;", "Status" }
                    }
                }
                tbody {
                    tr { style: "border-bottom: 1px solid #f1f5f9;",
                        td { style: "padding: 12px 16px; font-family: monospace;", "TX-991" }
                        td { style: "padding: 12px 16px;", "INSERT_QUIN" }
                        td { style: "padding: 12px 16px; font-family: monospace; font-size: 12px;", "[did:q42:alice] [knows] [did:q42:bob]" }
                        td { style: "padding: 12px 16px;", span { style: "background: #def7ec; color: #03543f; padding: 2px 8px; border-radius: 12px; font-size: 12px;", "FLUSHED" } }
                    }
                    tr { style: "border-bottom: 1px solid #f1f5f9;",
                        td { style: "padding: 12px 16px; font-family: monospace;", "TX-992" }
                        td { style: "padding: 12px 16px;", "DELETE_GRAPH" }
                        td { style: "padding: 12px 16px; font-family: monospace; font-size: 12px;", "context_hash: 0x123...abc" }
                        td { style: "padding: 12px 16px;", span { style: "background: #fdf6b2; color: #723b13; padding: 2px 8px; border-radius: 12px; font-size: 12px;", "PENDING" } }
                    }
                }
            }
        }
    }
}
