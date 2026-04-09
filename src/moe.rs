use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Deserialize, Debug)]
struct MoePayload {
    op: u8,
    t: Option<String>,
    d: Option<MoeData>,
}

#[derive(Deserialize, Debug)]
struct MoeData {
    heartbeat: Option<u64>,
    song: Option<Song>,
}

#[derive(Deserialize, Debug)]
struct Song {
    title: String,
    artists: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
struct Artist {
    name: String,
}

// Changed to async to prevent blocking the Tokio executor
async fn send_ipc_title(socket_path: &str, title: &str) {
    let payload = serde_json::json!({
        "command": ["set_property", "force-media-title", title]
    });
    let msg = format!("{}\n", payload.to_string());

    #[cfg(unix)]
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        if let Ok(mut stream) = UnixStream::connect(socket_path).await {
            let _ = stream.write_all(msg.as_bytes()).await;

            // Read more than 64 bytes to ensure the buffer is drained
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf).await;
        }
    }

    #[cfg(windows)]
    {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        if let Ok(mut file) = OpenOptions::new().write(true).open(socket_path).await {
            let _ = file.write_all(msg.as_bytes()).await;
        }
    }
}

pub async fn start_radio_sync(target_url: &str, ipc_socket: String) -> Result<()> {
    let ws_url = if target_url.contains("kpop") {
        "wss://listen.moe/kpop/gateway_v2"
    } else {
        "wss://listen.moe/gateway_v2"
    };

    log::info!("Connecting to LISTEN.moe WebSocket: {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Heartbeat management without spawning zombie tasks
    let mut interval = time::interval(Duration::from_secs(30));
    let mut hb_active = false;

    loop {
        tokio::select! {
            // Handle Heartbeats
            _ = interval.tick(), if hb_active => {
                if let Err(_) = write.send(Message::Text(r#"{"op": 9}"#.into())).await {
                    break;
                }
                log::debug!("Sent OP 9 Heartbeat");
            }

            // Handle Incoming Messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(payload) = serde_json::from_str::<MoePayload>(&text) {
                            match payload.op {
                                0 => { // Hello / Heartbeat Init
                                    if let Some(data) = payload.d {
                                        if let Some(hb) = data.heartbeat {
                                            log::debug!("Received heartbeat interval: {}ms", hb);
                                            interval = time::interval(Duration::from_millis(hb));
                                            interval.tick().await; // consume immediate tick
                                            hb_active = true;
                                        }
                                    }
                                }
                                1 => { // Track Update
                                    if payload.t.as_deref() == Some("TRACK_UPDATE") {
                                        if let Some(song) = payload.d.and_then(|d| d.song) {
                                            let artists = song.artists.iter()
                                                .map(|a| a.name.as_str())
                                                .collect::<Vec<_>>()
                                                .join(", ");
                                            let title = format!("{} - {}", artists, song.title);

                                            log::debug!("Now Playing: {}", title);
                                            send_ipc_title(&ipc_socket, &title).await;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => break,
                }
            }
        }
    }

    log::warn!("Radio sync lost connection to WebSocket.");
    Ok(())
}
