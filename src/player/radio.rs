use crate::config::Config;
use crate::tui::Icons;
use std::process::Command;

pub fn is_radio_station(target: &str) -> bool {
    crate::radio::RADIO_STATIONS
        .iter()
        .any(|(_, url, _)| *url == target)
}

pub fn generate_ipc_socket() -> String {
    let pid = std::process::id();
    if cfg!(windows) {
        format!(r"\\.\pipe\mpv-music-ipc-{}", pid)
    } else {
        format!("/tmp/mpv-music-ipc-{}.sock", pid)
    }
}

pub fn apply_radio_args(cmd: &mut Command, target: &str, ipc_socket: &str, config: &Config) {
    cmd.arg(format!("--input-ipc-server={}", ipc_socket));

    let (station_name, _) = crate::radio::RADIO_STATIONS
        .iter()
        .find(|(_, url, _)| *url == target)
        .map(|(name, _, is_moe)| {
            let clean_name = name.split(") ").nth(1).unwrap_or(name).to_uppercase();
            (clean_name, *is_moe)
        })
        .unwrap_or_else(|| ("RADIO".to_string(), false));

    let icons = Icons::new(config.nerd_fonts);
    cmd.arg(format!(
        "--term-status-msg= {} ${{media-title}} • ${{time-pos}} • [ {} ]",
        icons.play(),
        station_name
    ));
}

pub fn spawn_radio_sync_if_needed(target: &str, ipc_socket: &str) {
    let is_listen_moe = crate::radio::RADIO_STATIONS
        .iter()
        .find(|(_, url, _)| *url == target)
        .map(|(_, _, is_moe)| *is_moe)
        .unwrap_or(false);

    if is_listen_moe {
        let target_clone = target.to_string();
        let socket_clone = ipc_socket.to_string();
        std::thread::spawn(move || {
            let p = std::path::Path::new(&socket_clone);
            let mut attempts = 0;

            // Wait up to 5 seconds (50 * 100ms) for mpv to create the socket
            while !p.exists() && attempts < 50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                attempts += 1;
            }
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let _ =
                    crate::radio::listen_moe::start_radio_sync(&target_clone, socket_clone).await;
            });
        });
    }
}
