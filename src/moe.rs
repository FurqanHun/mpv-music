use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
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

fn send_ipc_title(socket_path: &str, title: &str) {
    let payload = serde_json::json!({
        "command": ["set_property", "force-media-title", title]
    });
    let msg = format!("{}\n", payload.to_string());

    #[cfg(unix)]
    {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        if let Ok(mut stream) = UnixStream::connect(socket_path) {
            let _ = stream.write_all(msg.as_bytes());

            // listen to mpv success msg so it doesnt throw a broken pipe
            let mut buf = [0; 64];
            let _ = stream.read(&mut buf);
        }
    }

    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut file) = OpenOptions::new().write(true).open(socket_path) {
            let _ = file.write_all(msg.as_bytes());
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
    let (write_pipe, mut read) = ws_stream.split();

    let write = Arc::new(Mutex::new(write_pipe));

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(payload) = serde_json::from_str::<MoePayload>(&text) {
                match payload.op {
                    0 => {
                        if let Some(data) = payload.d {
                            if let Some(hb) = data.heartbeat {
                                log::debug!("Received heartbeat interval: {}ms", hb);
                                let write_clone = write.clone();

                                tokio::spawn(async move {
                                    let mut interval = time::interval(Duration::from_millis(hb));
                                    loop {
                                        interval.tick().await;
                                        let mut w = write_clone.lock().await;
                                        let _ = w.send(Message::Text(r#"{"op": 9}"#.into())).await;

                                        log::debug!("Sent OP 9 Heartbeat");
                                    }
                                });
                            }
                        }
                    }
                    1 => {
                        if payload.t.as_deref() == Some("TRACK_UPDATE") {
                            if let Some(data) = payload.d {
                                if let Some(song) = data.song {
                                    let artists = song
                                        .artists
                                        .iter()
                                        .map(|a| a.name.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let title = format!("{} - {}", artists, song.title);

                                    log::info!("Now Playing: {}", title);
                                    send_ipc_title(&ipc_socket, &title);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
