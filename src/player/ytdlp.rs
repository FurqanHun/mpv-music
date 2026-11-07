use std::process::{Command, Stdio};

pub fn has_runtime(ytdlp_bin: &str, cmd: &str) -> bool {
    let binary_name = if cfg!(windows) {
        format!("{}.exe", cmd)
    } else {
        cmd.to_string()
    };

    let p = std::path::Path::new(ytdlp_bin);
    if p.parent()
        .is_some_and(|parent| parent.join(&binary_name).exists() || parent.join(cmd).exists())
    {
        return true;
    }

    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(output) = Command::new(check_cmd).arg(ytdlp_bin).output()
        && output.status.success()
    {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let first_line = path_str.lines().next().unwrap_or("").trim();
        let ytdlp_path = std::path::Path::new(first_line);

        if ytdlp_path
            .parent()
            .is_some_and(|p| p.join(&binary_name).exists() || p.join(cmd).exists())
        {
            return true;
        }
    }

    has_command(cmd)
}

pub fn detect_js_runtime(ytdlp_bin: &str) -> Option<&'static str> {
    const RUNTIMES: &[(&str, &str)] = &[
        ("node", "node"),
        ("qjs", "quickjs"),
        ("quickjs", "quickjs"),
        ("bun", "bun"),
        ("deno", "deno"),
    ];

    for &(cmd, ytdl_name) in RUNTIMES {
        if has_runtime(ytdlp_bin, cmd) {
            return Some(ytdl_name);
        }
    }
    None
}

pub fn has_command(cmd: &str) -> bool {
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    let exists = Command::new(check_cmd)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    log::debug!("Command check: {} exists = {}", cmd, exists);
    exists
}

pub fn check_ytdlp_status(ytdlp_bin: &str) {
    log::info!("Attempting {} self-update ({} -U)...", ytdlp_bin, ytdlp_bin);
    let sp = crate::ui::spinner(format!(
        "Checking {} update status ({} -U)...",
        ytdlp_bin, ytdlp_bin
    ));
    let output = match Command::new(ytdlp_bin).arg("-U").output() {
        Ok(o) => {
            sp.finish_and_clear();
            o
        }
        Err(_) => {
            sp.finish_and_clear();
            log::error!("{} executable not found in PATH", ytdlp_bin);
            return;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, String::from_utf8_lossy(&output.stderr));

    if combined.contains("is up to date") {
        log::info!("{} is verified up to date.", ytdlp_bin);
    } else if combined.contains("Latest version:") || combined.contains("Available version:") {
        log::warn!("{} update available. Local version is outdated.", ytdlp_bin);
    } else {
        log::debug!(
            "{} status check returned unexpected output:\n{}",
            ytdlp_bin,
            combined
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_command_invalid() {
        assert!(!has_command("this_command_definitely_does_not_exist_12345"));
    }

    #[test]
    fn test_has_runtime_invalid() {
        assert!(!has_runtime(
            "yt-dlp",
            "this_runtime_definitely_does_not_exist_12345"
        ));
    }

    #[test]
    fn test_detect_js_runtime_prefers_node_over_deno() {
        if has_command("node") {
            let detected = detect_js_runtime("yt-dlp");
            assert_eq!(detected, Some("node"));
        }
    }
}
