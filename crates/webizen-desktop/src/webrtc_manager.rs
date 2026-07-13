use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use std::sync::Arc;

pub async fn handle_webrtc_offer(offer_sdp: &str) -> Result<String, String> {
    let api = APIBuilder::new().build();
    let config = RTCConfiguration {
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await.map_err(|e| e.to_string())?);

    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");
        Box::pin(async {})
    }));

    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();
        let d_id = d.id();
        println!("New DataChannel {d_label} {d_id}");

        let d2 = Arc::clone(&d);
        d.on_open(Box::new(move || {
            println!("Data channel '{d_label}'-'{d_id}' open");
            let d3 = Arc::clone(&d2);
            Box::pin(async move {
                // Send PWA manifest/worker
                let pwa_data = r#"{"manifest":{"name":"QApp"},"service_worker":"console.log('SW installed');"}"#;
                let _ = d3.send_text(pwa_data.to_string()).await;
            })
        }));

        Box::pin(async {})
    }));

    let offer = RTCSessionDescription::offer(offer_sdp.to_string()).map_err(|e| e.to_string())?;
    peer_connection.set_remote_description(offer).await.map_err(|e| e.to_string())?;

    let answer = peer_connection.create_answer(None).await.map_err(|e| e.to_string())?;
    peer_connection.set_local_description(answer.clone()).await.map_err(|e| e.to_string())?;

    Ok(answer.sdp)
}
