use std::path::Path;
use std::fs;
use std::process;

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
    res == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        
        let mut exit_code = 0;
        let success = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        
        if success != 0 {
            exit_code == STILL_ACTIVE as u32
        } else {
            false
        }
    }
}

fn is_any_mpv_music_running() -> bool {
    true
}

pub fn cleanup_stale_queue_files(data_dir: &Path) {
    let current_pid = process::id();
    
    let entries = match fs::read_dir(data_dir) {
        Ok(dir) => dir,
        Err(e) => {
            log::debug!("Failed to read data_dir for cleanup: {}", e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with("queue_") && file_name.ends_with(".m3u8") {
                let pid_str = file_name
                    .strip_prefix("queue_")
                    .unwrap_or("")
                    .strip_suffix(".m3u8")
                    .unwrap_or("");
                
                if let Ok(pid) = pid_str.parse::<u32>() {
                    if pid == current_pid {
                        continue;
                    }
                    
                    if !is_pid_alive(pid) {
                        log::info!("Removing orphaned queue file: {:?}", file_name);
                        let _ = fs::remove_file(&path);
                    }
                }
            } else if file_name == "queue.m3u8" {
                if !is_any_mpv_music_running() {
                    log::info!("Removing legacy generic queue file: {:?}", file_name);
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cleanup_stale_queue_files() {
        let dir = tempdir().unwrap();
        let current_pid = process::id();
        
        let active_file = dir.path().join(format!("queue_{}.m3u8", current_pid));
        fs::write(&active_file, "active").unwrap();
        
        let dead_file = dir.path().join(format!("queue_{}.m3u8", 9999999));
        fs::write(&dead_file, "dead").unwrap();
        
        let normal_file = dir.path().join("config.toml");
        fs::write(&normal_file, "cfg").unwrap();
        
        let other_queue = dir.path().join("queue_not_a_pid.m3u8");
        fs::write(&other_queue, "invalid").unwrap();
        
        let legacy_file = dir.path().join("queue.m3u8");
        fs::write(&legacy_file, "legacy").unwrap();

        cleanup_stale_queue_files(dir.path());

        assert!(active_file.exists(), "Active queue file should not be deleted");
        assert!(!dead_file.exists(), "Dead queue file should be deleted");
        assert!(normal_file.exists(), "Normal file should not be deleted");
        assert!(other_queue.exists(), "Invalid PID queue file should not be deleted");
        assert!(legacy_file.exists(), "Legacy queue file should not be deleted in tests where fallback is true");
    }
}
