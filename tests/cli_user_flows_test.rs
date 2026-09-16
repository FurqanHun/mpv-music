use std::fs;
use std::process::Command;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_mpv-music")
}

#[test]
fn test_user_flow_help_screens() {
    let output_long = Command::new(bin_path())
        .arg("--help")
        .output()
        .expect("Failed to execute mpv-music --help");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(stdout_long.contains("--genre"));
    assert!(stdout_long.contains("--artist"));
    assert!(stdout_long.contains("--album"));
    assert!(stdout_long.contains("--title"));
    assert!(stdout_long.contains("--remove-log"));
    assert!(stdout_long.contains("--config"));
    assert!(stdout_long.contains("--manage-dirs"));
    assert!(stdout_long.contains("--radio"));

    let output_short = Command::new(bin_path())
        .arg("-h")
        .output()
        .expect("Failed to execute mpv-music -h");

    assert!(output_short.status.success());
}

#[test]
fn test_user_flow_version_flags() {
    let output_long = Command::new(bin_path())
        .arg("--version")
        .output()
        .expect("Failed to execute mpv-music --version");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(stdout_long.starts_with("mpv-music "));

    let output_short = Command::new(bin_path())
        .arg("-V")
        .output()
        .expect("Failed to execute mpv-music -V");

    assert!(output_short.status.success());
    let stdout_short = String::from_utf8_lossy(&output_short.stdout);
    assert!(stdout_short.starts_with("mpv-music "));
}

#[test]
fn test_user_flow_invalid_flags() {
    let output = Command::new(bin_path())
        .arg("--completely-invalid-flag-999")
        .output()
        .expect("Failed to run mpv-music");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: unexpected argument"));
}

#[test]
fn test_user_flow_isolated_dir_management() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_config_dir = temp.path().join("config");
    let fake_data_dir = temp.path().join("data");
    fs::create_dir_all(&fake_config_dir).unwrap();
    fs::create_dir_all(&fake_data_dir).unwrap();

    let fake_music_path = temp.path().join("my_music_collection");
    fs::create_dir_all(&fake_music_path).unwrap();

    // 1. Add directory flow
    let add_output = Command::new(bin_path())
        .arg("--add-dir")
        .arg(fake_music_path.to_str().unwrap())
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --add-dir");

    assert!(add_output.status.success());
    let add_stdout = String::from_utf8_lossy(&add_output.stdout);
    assert!(add_stdout.contains("Added directory") || add_stdout.contains("my_music_collection"));

    // 2. Remove directory flow with alias --rm-dir
    let rm_output = Command::new(bin_path())
        .arg("--rm-dir")
        .arg(fake_music_path.to_str().unwrap())
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --rm-dir");

    assert!(rm_output.status.success());
    let rm_stdout = String::from_utf8_lossy(&rm_output.stdout);
    assert!(rm_stdout.contains("Removed directory") || rm_stdout.contains("my_music_collection"));
}

#[test]
fn test_user_flow_isolated_log_deletion() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_data_dir = temp.path().join("data");
    let logs_dir = fake_data_dir.join("mpv-music").join("logs");
    fs::create_dir_all(&logs_dir).unwrap();

    // Populate fake session logs
    let log1 = logs_dir.join("session_20260101_100000_111.log");
    let log2 = logs_dir.join("session_20260101_110000_222.log");
    let log3 = logs_dir.join("session_20260101_120000_333.log");
    fs::write(&log1, "log 1").unwrap();
    fs::write(&log2, "log 2").unwrap();
    fs::write(&log3, "log 3").unwrap();

    // 1. Delete oldest 1 log via --rm-log 1
    let rm_oldest_output = Command::new(bin_path())
        .arg("--rm-log")
        .arg("1")
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --rm-log 1");

    assert!(rm_oldest_output.status.success());
    let rm_oldest_stdout = String::from_utf8_lossy(&rm_oldest_output.stdout);
    assert!(rm_oldest_stdout.contains("Deleted 1 oldest log file(s)."));

    // 2. Delete all remaining logs via --remove-log
    let rm_all_output = Command::new(bin_path())
        .arg("--remove-log")
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --remove-log");

    assert!(rm_all_output.status.success());
    let rm_all_stdout = String::from_utf8_lossy(&rm_all_output.stdout);
    assert!(rm_all_stdout.contains("Deleted all"));

    // 3. Delete again when empty: should show friendly warning on stderr
    let rm_empty_output = Command::new(bin_path())
        .arg("--remove-log")
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --remove-log empty");

    assert!(rm_empty_output.status.success());
    let rm_empty_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&rm_empty_output.stdout),
        String::from_utf8_lossy(&rm_empty_output.stderr)
    );
    assert!(rm_empty_combined.contains("No log files available to delete."));
}

#[test]
fn test_user_flow_isolated_config_removal() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_config_dir = temp.path().join("config");
    let app_config_dir = fake_config_dir.join("mpv-music");
    fs::create_dir_all(&app_config_dir).unwrap();

    let config_file = app_config_dir.join("config.toml");
    fs::write(&config_file, "volume = 90\nshuffle = false\n").unwrap();
    assert!(config_file.exists());

    let rm_conf_output = Command::new(bin_path())
        .arg("--rm-conf")
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run --rm-conf");

    assert!(rm_conf_output.status.success());
    assert!(!config_file.exists());
}

#[test]
fn test_user_flow_empty_library_play_all() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_config_dir = temp.path().join("config");
    let fake_data_dir = temp.path().join("data");
    fs::create_dir_all(&fake_config_dir).unwrap();
    fs::create_dir_all(&fake_data_dir).unwrap();

    let output = Command::new(bin_path())
        .arg("--play-all")
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run mpv-music --play-all");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("No music found in local library"));
}

#[test]
fn test_user_flow_empty_directory_target() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_config_dir = temp.path().join("config");
    let fake_data_dir = temp.path().join("data");
    let empty_dir = temp.path().join("empty_folder");
    fs::create_dir_all(&fake_config_dir).unwrap();
    fs::create_dir_all(&fake_data_dir).unwrap();
    fs::create_dir_all(&empty_dir).unwrap();

    let output = Command::new(bin_path())
        .arg(empty_dir.to_str().unwrap())
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run mpv-music <empty_dir>");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("No music files found in:"));
}

#[test]
fn test_user_flow_cli_filter_with_no_tracks() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let fake_config_dir = temp.path().join("config");
    let fake_data_dir = temp.path().join("data");
    fs::create_dir_all(&fake_config_dir).unwrap();
    fs::create_dir_all(&fake_data_dir).unwrap();

    let output = Command::new(bin_path())
        .arg("--genre")
        .arg("CyberpunkRock")
        .env("XDG_CONFIG_HOME", &fake_config_dir)
        .env("XDG_DATA_HOME", &fake_data_dir)
        .env("HOME", temp.path())
        .output()
        .expect("Failed to run mpv-music --genre");

    // Must not crash/panic, status should be clean or normal exit
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("No match") || combined.contains("No music found"));
}
