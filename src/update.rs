#[cfg(feature = "update")]
use anyhow::{Context, Result};
#[cfg(feature = "update")]
use std::env;

#[cfg(feature = "update")]
fn parse_version(v: &str) -> (u32, u32, u32) {
    let v = v.trim_start_matches('v');
    let base = v.split('-').next().unwrap_or(v);
    let parts: Vec<u32> = base.split('.').map(|s| s.parse().unwrap_or(0)).collect();

    (
        *parts.first().unwrap_or(&0),
        *parts.get(1).unwrap_or(&0),
        *parts.get(2).unwrap_or(&0),
    )
}

#[cfg(feature = "update")]
fn fetch_json_from_url(url: &str) -> Result<serde_json::Value> {
    log::info!("Fetching update information from: {}", url);
    match ureq::get(url).call() {
        Ok(mut response) => {
            log::debug!("Native request successful, parsing JSON...");
            let json: serde_json::Value = serde_json::from_reader(response.body_mut().as_reader())
                .context("Failed to parse JSON from GitHub via ureq")?;
            Ok(json)
        }
        Err(e) => {
            log::warn!("Native request failed ({}). Attempting curl fallback...", e);
            let output = std::process::Command::new("curl")
                .args(["-sL", url])
                .output()
                .context("Failed to run curl fallback. Are you connected to the internet?")?;

            if !output.status.success() {
                anyhow::bail!("Curl fallback failed with status: {}", output.status);
            }

            log::debug!("Curl fallback request successful, parsing JSON...");
            let json: serde_json::Value = serde_json::from_slice(&output.stdout)
                .context("Failed to parse JSON from GitHub via curl")?;
            Ok(json)
        }
    }
}

#[cfg(feature = "update")]
#[allow(unused_variables)]
fn prompt_and_update(is_dev: bool, latest_tag: &str, auto_confirm: bool) {
    let mut confirmed = auto_confirm;

    if !confirmed {
        print!("\nDo you want to update now? [Y/n]: ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        if input.trim().eq_ignore_ascii_case("y") {
            confirmed = true;
        }
    }

    if confirmed {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            println!("Starting update...");
            let script_args = if is_dev {
                format!("--dev --update --tag {}", latest_tag)
            } else {
                format!("--update --tag {}", latest_tag)
            };
            let cmd_str = format!("curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash -s -- {}", script_args);
            
            let status = std::process::Command::new("bash")
                .arg("-c")
                .arg(&cmd_str)
                .status();
                
            if let Err(e) = status {
                log::error!("Failed to launch updater: {}", e);
            }
        }
        #[cfg(target_os = "windows")]
        {
            println!("Starting update...");
            
            let mut ps_args = vec!["-Update".to_string(), format!("-Tag '{}'", latest_tag)];
            if is_dev {
                ps_args.push("-Dev".to_string());
            }
            
            let args_str = ps_args.join(" ");
            let ps_cmd = format!(
                "& ([scriptblock]::Create((iwr https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.ps1 -UseBasicParsing).Content)) {}",
                args_str
            );
            
            let status = std::process::Command::new("powershell")
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_cmd])
                .status();
                
            if let Err(e) = status {
                log::error!("Failed to launch updater: {}", e);
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            println!("Please download the update manually from: https://github.com/FurqanHun/mpv-music");
        }
    } else {
        println!("Update aborted. You can run the update later.");
    }
}

#[cfg(feature = "update")]
pub fn update_self(auto_confirm: bool) -> Result<()> {
    let current_ver_str = env!("CARGO_PKG_VERSION");
    let is_dev = current_ver_str.contains("dev");

    println!("Checking for updates...");

    let json = fetch_json_from_url("https://furqanhun.github.io/mpv-music/latest.json")?;

    let remote_tag = json["stable"]["tag_name"]
        .as_str()
        .context("Release missing stable tag_name")?;
    let remote_ver_str = remote_tag.trim_start_matches('v');
    log::debug!("Parsed stable version: v{}", remote_ver_str);

    println!("\n--- Version Info ---");
    println!("Current Version:  v{}", current_ver_str);
    println!("Latest Stable:    v{}", remote_ver_str);

    if !is_dev {
        let current_semver = parse_version(current_ver_str);
        let remote_semver = parse_version(remote_ver_str);

        if remote_semver > current_semver {
            println!("Update Available: \x1b[32mYES\x1b[0m");
            prompt_and_update(false, remote_tag, auto_confirm);
        } else {
            println!("Update Status:    \x1b[32mUp to date\x1b[0m");
        }
    } else {
        if let Some(latest_obj) = json.get("dev") {
            let latest_tag = latest_obj["tag_name"].as_str().unwrap_or("?");
            let latest_ver = latest_tag.trim_start_matches('v');
            log::debug!("Parsed dev version: v{}", latest_ver);

            println!("Latest Release:   v{}", latest_ver);

            let current_semver = parse_version(current_ver_str);
            let latest_semver = parse_version(latest_ver);

            let is_latest_stable = !latest_ver.contains('-');

            // if update is strictly NEWER
            let update_available = if latest_semver > current_semver {
                true
            } else if latest_semver == current_semver {
                // base versions match, we must check suffixes
                if !is_latest_stable && current_ver_str.contains("dev") {
                    // extract number after last dot (e.g. "dev.16" -> 16)
                    let get_num = |s: &str| -> u32 {
                        s.rsplit('.').next().unwrap_or("0").parse().unwrap_or(0)
                    };
                    get_num(latest_ver) > get_num(current_ver_str)
                } else {
                    // with same base, stable is "newer"
                    is_latest_stable && current_ver_str.contains("dev")
                }
            } else {
                false
            };

            if update_available {
                let build_type = if is_latest_stable {
                    "Stable Build"
                } else {
                    "Development Build"
                };
                println!("Update Status:    \x1b[32mYES\x1b[0m ({})", build_type);
                prompt_and_update(!is_latest_stable, latest_tag, auto_confirm);
            } else {
                println!("Update Status:    \x1b[33mUp to date\x1b[0m (Development Build)");
            }
        }
    }
    println!("--------------------\n");

    Ok(())
}

#[cfg(test)]
#[cfg(feature = "update")]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_simple() {
        assert_eq!(parse_version("1.2.3"), (1, 2, 3));
        assert_eq!(parse_version("0.24.0"), (0, 24, 0));
        assert_eq!(parse_version("2.0.1"), (2, 0, 1));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        assert_eq!(parse_version("v1.2.3"), (1, 2, 3));
        assert_eq!(parse_version("v0.25.0"), (0, 25, 0));
    }

    #[test]
    fn test_parse_version_dev() {
        // Should parse base version, ignoring -dev suffix
        assert_eq!(parse_version("0.25.0-dev.1"), (0, 25, 0));
        assert_eq!(parse_version("1.0.0-dev"), (1, 0, 0));
    }

    #[test]
    fn test_parse_version_missing_parts() {
        assert_eq!(parse_version("1.2"), (1, 2, 0));
        assert_eq!(parse_version("5"), (5, 0, 0));
    }

    #[test]
    fn test_parse_version_invalid() {
        // Should handle gracefully with 0s
        assert_eq!(parse_version("abc"), (0, 0, 0));
        assert_eq!(parse_version("1.x.3"), (1, 0, 3));
    }

    #[test]
    fn test_version_comparison() {
        let v1 = parse_version("0.24.0");
        let v2 = parse_version("0.25.0");

        assert!(v2 > v1);
    }

    #[test]
    fn test_version_equality() {
        let v1 = parse_version("1.0.0");
        let v2 = parse_version("v1.0.0");

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_version_major_diff() {
        let v1 = parse_version("1.0.0");
        let v2 = parse_version("2.0.0");

        assert!(v2 > v1);
    }

    #[test]
    fn test_version_minor_diff() {
        let v1 = parse_version("1.5.0");
        let v2 = parse_version("1.6.0");

        assert!(v2 > v1);
    }

    #[test]
    fn test_version_patch_diff() {
        let v1 = parse_version("1.0.1");
        let v2 = parse_version("1.0.2");

        assert!(v2 > v1);
    }
}
