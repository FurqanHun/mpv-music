use std::process::{Command, Stdio};

pub fn check_deno_availability(ytdlp_bin: &str) -> bool {
    let p = std::path::Path::new(ytdlp_bin);
    if p.parent().is_some_and(|parent| {
        parent
            .join(if cfg!(windows) { "deno.exe" } else { "deno" })
            .exists()
            || parent.join("deno").exists()
    }) {
        return true;
    }

    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    let Ok(output) = Command::new(check_cmd).arg(ytdlp_bin).output() else {
        return has_command("deno");
    };

    if !output.status.success() {
        return has_command("deno");
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let first_line = path_str.lines().next().unwrap_or("").trim();
    let ytdlp_path = std::path::Path::new(first_line);

    if ytdlp_path.parent().is_some_and(|p| {
        p.join(if cfg!(windows) { "deno.exe" } else { "deno" })
            .exists()
            || p.join("deno").exists()
    }) {
        return true;
    }

    has_command("deno")
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
    let output = match Command::new(ytdlp_bin).arg("-U").output() {
        Ok(o) => o,
        Err(_) => {
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
}
