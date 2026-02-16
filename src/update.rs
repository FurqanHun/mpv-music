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
pub fn update_self() -> Result<()> {
    let current_ver_str = env!("CARGO_PKG_VERSION");
    let is_dev = current_ver_str.contains("dev");

    println!("Checking for updates...");

    let mut response =
        ureq::get("https://api.github.com/repos/FurqanHun/mpv-music/releases/latest")
            .call()
            .map_err(|e| anyhow::anyhow!("Failed to check GitHub: {}", e))?;

    let json: serde_json::Value = serde_json::from_reader(response.body_mut().as_reader())
        .context("Failed to parse JSON from GitHub")?;

    let remote_tag = json["tag_name"]
        .as_str()
        .context("Release missing tag_name")?;
    let remote_ver_str = remote_tag.trim_start_matches('v');

    println!("\n--- Version Info ---");
    println!("Current Version:  v{}", current_ver_str);
    println!("Latest Stable:    v{}", remote_ver_str);

    if !is_dev {
        let current_semver = parse_version(current_ver_str);
        let remote_semver = parse_version(remote_ver_str);

        if remote_semver > current_semver {
            println!("Update Available: \x1b[32mYES\x1b[0m");
            println!("\nTo update, run this command:");
            println!(
                "\x1b[1mcurl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash\x1b[0m"
            );
            println!("\nOr download manually:");
            println!("https://github.com/FurqanHun/mpv-music/releases/latest");
        } else {
            println!("Update Status:    \x1b[32mUp to date\x1b[0m");
        }
    } else {
        let mut dev_resp =
            ureq::get("https://api.github.com/repos/FurqanHun/mpv-music/releases?per_page=1")
                .call()
                .map_err(|e| anyhow::anyhow!("Failed to check dev updates: {}", e))?;

        let dev_list: Vec<serde_json::Value> =
            serde_json::from_reader(dev_resp.body_mut().as_reader())
                .context("Failed to parse releases list")?;

        if let Some(latest_obj) = dev_list.first() {
            let latest_tag = latest_obj["tag_name"].as_str().unwrap_or("?");
            let latest_ver = latest_tag.trim_start_matches('v');

            println!("Latest Release:   v{}", latest_ver);

            let current_semver = parse_version(current_ver_str);
            let latest_semver = parse_version(latest_ver);

            // if update is strictly NEWER
            let update_available = if latest_semver > current_semver {
                true
            } else if latest_semver == current_semver {
                // base versions match, we must check suffixes
                if latest_ver.contains("dev") && current_ver_str.contains("dev") {
                    // extract number after last dot (e.g. "dev.16" -> 16)
                    let get_num = |s: &str| -> u32 {
                        s.rsplit('.').next().unwrap_or("0").parse().unwrap_or(0)
                    };
                    get_num(latest_ver) > get_num(current_ver_str)
                } else {
                    // with same base, stable is "newer"
                    !latest_ver.contains("dev") && current_ver_str.contains("dev")
                }
            } else {
                false
            };

            if update_available {
                println!("Update Status:    \x1b[32mYES\x1b[0m (Development Build)");
                println!("\nTo update, run this command:");
                println!(
                    "\x1b[1mcurl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash -s -- --dev \x1b[0m"
                );
                println!("\nLinks:");
                println!("Stable: https://github.com/FurqanHun/mpv-music/releases/latest");
                println!(
                    "Latest: https://github.com/FurqanHun/mpv-music/releases/tag/{}",
                    latest_tag
                );
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
