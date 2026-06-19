use dioxus::prelude::*;

#[component]
pub fn P2pDashboard() -> Element {
    rsx! {
        div { style: "padding: 30px; background-color: #121212; color: #e0e0e0; min-height: 100vh; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;",
            h1 { style: "color: #bb86fc; border-bottom: 2px solid #333; padding-bottom: 10px;", "P2P Network Topology" }

            div { style: "display: flex; gap: 20px; margin-top: 20px;",
                div { style: "flex: 1; background: #1e1e1e; padding: 20px; border-radius: 10px;",
                    h2 { style: "margin-top: 0; font-size: 18px; color: #03dac6;", "Node Identity" }
                    p { "DID: did:q42:8f7e...9a2b" }
                    p { "Libp2p PeerId: QmRw2...XyZ" }
                    p { "Role: Full Validator Node" }
                }
                div { style: "flex: 2; background: #1e1e1e; padding: 20px; border-radius: 10px;",
                    h2 { style: "margin-top: 0; font-size: 18px; color: #cf6679;", "Connection Pool" }
                    table { style: "width: 100%; text-align: left; border-collapse: collapse;",
                        thead {
                            tr { style: "border-bottom: 1px solid #444;", th { "Peer ID" } th { "Latency" } th { "Protocol" } }
                        }
                        tbody {
                            tr { style: "border-bottom: 1px solid #333;", td { "QmAx3...1" } td { "24ms" } td { "/qualia/sync/1.0" } }
                            tr { style: "border-bottom: 1px solid #333;", td { "QmBx5...2" } td { "145ms" } td { "/qualia/gossip/1.0" } }
                            tr { style: "border-bottom: 1px solid #333;", td { "QmCx7...3" } td { "8ms" } td { "/qualia/sync/1.0" } }
                        }
                    }
                }
            }

            div { style: "margin-top: 20px; background: #1e1e1e; padding: 20px; border-radius: 10px;",
                h2 { style: "margin-top: 0; font-size: 18px; color: #03dac6;", "DHT Routing Table" }
                div { style: "display: grid; grid-template-columns: repeat(16, 1fr); gap: 4px;",
                    for i in 0..128 {
                        div { style: "height: 20px; background: #333; border-radius: 2px;", title: "Bucket {i}" }
                    }
                }
                p { style: "font-size: 12px; color: #888; margin-top: 10px;", "K-Buckets visualization. Green indicates active peers." }
            }
        }
    }
}
