use dioxus::prelude::*;

#[component]
pub fn WebtorrentSeeder() -> Element {
    rsx! {
        div { style: "padding: 24px; background: #2d3748; color: white; height: 100%; overflow: auto;",
            div { style: "display: flex; align-items: center; justify-content: space-between; border-bottom: 2px solid #4a5568; padding-bottom: 16px; margin-bottom: 24px;",
                h2 { style: "margin: 0; color: #4fd1c5; display: flex; align-items: center; gap: 12px;",
                    "🌐 WebTorrent Swarm Seeder"
                }
                div { style: "background: #2b6cb0; padding: 6px 16px; border-radius: 20px; font-size: 14px; font-weight: bold;", "Swarm: HEALTHY" }
            }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 24px;",
                div { style: "background: #1a202c; padding: 20px; border-radius: 12px;",
                    h3 { style: "margin-top: 0; color: #a0aec0;", "Local Seed Metrics" }
                    div { style: "display: flex; justify-content: space-between; margin-bottom: 12px;", span { "Up Speed:" } span { style: "color: #48bb78;", "1.2 MB/s" } }
                    div { style: "display: flex; justify-content: space-between; margin-bottom: 12px;", span { "Peers Connected:" } span { style: "color: #ed8936;", "42" } }
                    div { style: "display: flex; justify-content: space-between; margin-bottom: 12px;", span { "Ratio:" } span { style: "color: #ecc94b;", "3.14" } }
                }
                div { style: "background: #1a202c; padding: 20px; border-radius: 12px;",
                    h3 { style: "margin-top: 0; color: #a0aec0;", "Active Graph Hashes" }
                    div { style: "font-family: monospace; font-size: 12px; color: #718096; display: flex; flex-direction: column; gap: 8px;",
                        div { "QmYwAPJzv5CZsnA625s3Xf2ep5ixgi8X14K3... [Deontic Base]" }
                        div { "QmZTR5bcpQD7cFgTorxqcpNd81q2Xy... [Llama3 Q4_K_M]" }
                        div { "QmdfQn1A5... [Medical Ontology]" }
                    }
                }
            }

            h3 { style: "margin-top: 32px; color: #e2e8f0;", "Peer Traffic Radar" }
            div { style: "height: 200px; background: radial-gradient(circle, #2d3748 0%, #1a202c 100%); border: 1px solid #4a5568; border-radius: 12px; position: relative; overflow: hidden;",
                div { style: "position: absolute; top: 30%; left: 40%; width: 8px; height: 8px; background: #4fd1c5; border-radius: 50%; box-shadow: 0 0 10px #4fd1c5;" }
                div { style: "position: absolute; top: 60%; left: 70%; width: 6px; height: 6px; background: #4fd1c5; border-radius: 50%; box-shadow: 0 0 10px #4fd1c5;" }
                div { style: "position: absolute; top: 20%; left: 60%; width: 10px; height: 10px; background: #ed8936; border-radius: 50%; box-shadow: 0 0 10px #ed8936;" }
            }
        }
    }
}
