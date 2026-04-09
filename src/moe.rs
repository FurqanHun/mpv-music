use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// OP 0: Hello
#[derive(Deserialize, Debug)]
struct HelloData {
    #[allow(dead_code)]
    message: Option<String>,
    heartbeat: u64,
}

// OP 1: Track update
#[derive(Deserialize, Debug)]
struct TrackData {
    song: Option<Song>,
    listeners: Option<u32>,
}

// Generic envelope — d is raw JSON, we deserialize based on op
#[derive(Deserialize, Debug)]
struct MoePayload {
    op: u8,
    t: Option<String>,
    d: Option<serde_json::Value>,
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

async fn send_ipc_title(socket_path: &str, title: &str) {
    let payload = serde_json::json!({
        "command": ["set_property", "force-media-title", title]
    });
    let msg = format!("{}\n", payload.to_string());

    #[cfg(unix)]
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        match UnixStream::connect(socket_path).await {
            Ok(mut stream) => {
                let _ = stream.write_all(msg.as_bytes()).await;
                // drain the response buffer
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf).await;
            }
            Err(e) => {
                log::warn!("IPC socket not ready at '{}': {}", socket_path, e);
            }
        }
    }

    #[cfg(windows)]
    {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        match OpenOptions::new().write(true).open(socket_path).await {
            Ok(mut file) => {
                let _ = file.write_all(msg.as_bytes()).await;
            }
            Err(e) => {
                log::warn!("IPC pipe not ready at '{}': {}", socket_path, e);
            }
        }
    }
}

// Inner WS loop — runs until the connection drops, then returns so the
// outer reconnect loop in start_radio_sync can retry.
async fn run_ws_loop(
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    ipc_socket: &str,
) {
    let (mut write, mut read) = ws_stream.split();

    let mut interval = time::interval(Duration::from_secs(30));
    let mut hb_active = false;

    loop {
        tokio::select! {
            _ = interval.tick(), if hb_active => {
                if write.send(Message::Text(r#"{"op": 9}"#.into())).await.is_err() {
                    log::warn!("Failed to send heartbeat. Connection likely dropped.");
                    break;
                }
                log::debug!("Sent OP 9 Heartbeat");
            }

            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<MoePayload>(&text) {
                            Ok(payload) => {
                                handle_payload(payload, &mut interval, &mut hb_active, ipc_socket).await;
                            }
                            Err(e) => {
                                log::debug!("Failed to parse WS payload: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // respond to server pings
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(_)) => {
                        // binary frames etc, ignore
                    }
                    Some(Err(e)) => {
                        log::warn!("WS error: {}", e);
                        break;
                    }
                    None => {
                        log::warn!("WS stream closed by server.");
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_payload(
    payload: MoePayload,
    interval: &mut time::Interval,
    hb_active: &mut bool,
    ipc_socket: &str,
) {
    match payload.op {
        0 => {
            // OP 0: Hello — server sends heartbeat interval
            if let Some(raw) = payload.d {
                match serde_json::from_value::<HelloData>(raw) {
                    Ok(hello) => {
                        log::debug!(
                            "OP 0 Hello: {:?} | Heartbeat interval: {}ms",
                            hello.message,
                            hello.heartbeat
                        );
                        *interval = time::interval(Duration::from_millis(hello.heartbeat));
                        interval.tick().await; // consume the immediate tick
                        *hb_active = true;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse OP 0 Hello data: {}", e);
                    }
                }
            }
        }

        1 => {
            // OP 1: Track update — fires on both TRACK_UPDATE and TRACK_UPDATE_REQUEST
            let is_track_event = matches!(
                payload.t.as_deref(),
                Some("TRACK_UPDATE") | Some("TRACK_UPDATE_REQUEST")
            );

            if !is_track_event {
                return;
            }

            if let Some(raw) = payload.d {
                match serde_json::from_value::<TrackData>(raw) {
                    Ok(data) => {
                        if let Some(listeners) = data.listeners {
                            log::debug!("Listeners: {}", listeners);
                        }

                        if let Some(song) = data.song {
                            let title = if song.artists.is_empty() {
                                // instrumental / no artist tag
                                song.title.clone()
                            } else {
                                let artists = song
                                    .artists
                                    .iter()
                                    .map(|a| a.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                format!("{} - {}", artists, song.title)
                            };

                            log::debug!("Now Playing: {}", title);
                            // send_ipc_title(ipc_socket, &title).await;
                            let sock = ipc_socket.to_string();
                            let t = title.clone();
                            tokio::spawn(async move {
                                send_ipc_title(&sock, &t).await;
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse OP 1 Track data: {}", e);
                    }
                }
            }
        }

        9 => {
            // OP 9: our own heartbeat echo — shouldn't arrive but handle gracefully
            log::debug!("OP 9 received (unexpected echo)");
        }

        10 => {
            // OP 10: Heartbeat ACK from server
            log::debug!("OP 10: Heartbeat ACK");
        }

        op => {
            log::debug!("Unhandled OP {}: {:?}", op, payload.t);
        }
    }
}

pub async fn start_radio_sync(target_url: &str, ipc_socket: String) -> Result<()> {
    let ws_url = if target_url.contains("kpop") {
        "wss://listen.moe/kpop/gateway_v2"
    } else {
        "wss://listen.moe/gateway_v2"
    };

    let mut backoff = Duration::from_secs(2);

    loop {
        log::info!("Connecting to LISTEN.moe WebSocket: {}", ws_url);

        match connect_async(ws_url).await {
            Ok((ws_stream, _)) => {
                log::info!("WS connected.");
                backoff = Duration::from_secs(2); // reset backoff on success
                run_ws_loop(ws_stream, &ipc_socket).await;
                log::warn!("WS loop exited. Reconnecting...");
            }
            Err(e) => {
                log::warn!("WS connect failed: {}. Retrying in {:?}...", e, backoff);
            }
        }

        time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60)); // exponential backoff, cap 60s
    }
}
