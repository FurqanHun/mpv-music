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
        *parts.get(0).unwrap_or(&0),
        *parts.get(1).unwrap_or(&0),
        *parts.get(2).unwrap_or(&0),
    )
}

#[cfg(feature = "update")]
pub fn update_self() -> Result<()> {
    let current_ver_str = env!("CARGO_PKG_VERSION");

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

    let (cur_major, cur_minor, cur_patch) = parse_version(current_ver_str);
    let (rem_major, rem_minor, rem_patch) = parse_version(remote_ver_str);

    let update_available = if rem_major > cur_major {
        true
    } else if rem_major == cur_major && rem_minor > cur_minor {
        true
    } else if rem_major == cur_major && rem_minor == cur_minor && rem_patch > cur_patch {
        true
    } else {
        false
    };

    println!("\n--- Version Info ---");
    println!("Current Version:  v{}", current_ver_str);
    println!("Latest Stable:    v{}", remote_ver_str);

    if update_available {
        println!("Update Available: \x1b[32mYES\x1b[0m"); // Green "YES"
        println!("\nTo update, run this command:");
        println!(
            "\x1b[1mcurl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash\x1b[0m"
        );
        println!("\nOr download manually:");
        println!("https://github.com/FurqanHun/mpv-music/releases/latest");
    } else {
        if current_ver_str.contains("dev") {
            println!("Update Status:    \x1b[33mDevelopment Build\x1b[0m (Newer than stable)");
        } else {
            println!("Update Status:    \x1b[32mUp to date\x1b[0m");
        }
    }
    println!("--------------------\n");

    Ok(())
}
