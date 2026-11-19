use super::*;

fn temp_dir(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "rustfetch-tools-{tag}-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn test_validate_installer_script_checks_shebang_and_repo() {
    assert!(validate_installer_script("#!/usr/bin/env bash\nREPO=MehmetCanWT/rustfetch\n").is_ok());
    assert!(validate_installer_script("#!/usr/bin/env bash\nREPO=MehmetCanWT/Rustfetch\n").is_ok());
    assert!(
        validate_installer_script("#!/usr/bin/env bash\nREPO=MehmetCanWT/rustfetch\0\n").is_err()
    );
    assert!(validate_installer_script("").is_err());
    assert!(validate_installer_script("<html>404</html>").is_err());
    assert!(validate_installer_script("#!/bin/sh\necho hi\n").is_err());
}

#[test]
fn test_build_installer_command_preserves_config() {
    let cmd = build_installer_command(
        Path::new("/tmp/install.sh"),
        "0.2.0",
        Some(Path::new("/home/user/.local/bin")),
    );
    let envs: Vec<_> = cmd.get_envs().collect();
    assert!(
        envs.iter()
            .any(|(k, v)| *k == "NONINTERACTIVE" && *v == Some(std::ffi::OsStr::new("1")))
    );
    assert!(
        envs.iter().any(|(k, v)| {
            *k == "RUSTFETCH_FORCE_CONFIG" && *v == Some(std::ffi::OsStr::new("0"))
        })
    );
    assert!(
        envs.iter()
            .any(|(k, v)| *k == "RUSTFETCH_VERSION" && *v == Some(std::ffi::OsStr::new("0.2.0")))
    );
    assert!(envs.iter().any(|(k, v)| {
        *k == "RUSTFETCH_INSTALL_DIR" && *v == Some(std::ffi::OsStr::new("/home/user/.local/bin"))
    }));
}

#[test]
fn test_module_and_preset_descriptions_are_valid() {
    for &(name, description) in MODULE_DESCRIPTIONS {
        assert!(!description.is_empty());
        assert!(
            module_names().contains(&name),
            "Module '{name}' in MODULE_DESCRIPTIONS must be a canonical module name"
        );
    }
    for &name in module_names() {
        assert!(
            module_by_name(name).is_some(),
            "Canonical module '{name}' must resolve via module_by_name"
        );
    }
    for &(preset, description) in PRESET_DESCRIPTIONS {
        assert!(!description.is_empty());
        assert!(
            Config::from_preset(preset).is_some(),
            "Preset '{preset}' in PRESET_DESCRIPTIONS must resolve via Config::from_preset"
        );
    }
}

#[test]
fn test_theme_choices_use_the_renderer_registry() {
    let choices = theme_choices();
    assert_eq!(choices.len(), rustfetch_render::theme_names().len() + 1);
    assert_eq!(choices.first().map(String::as_str), Some("none"));
    for name in rustfetch_render::theme_names() {
        assert!(
            rustfetch_render::get_theme(name).is_some(),
            "Theme '{name}' must resolve via get_theme"
        );
        assert!(choices.iter().any(|choice| choice == name));
    }
}

#[test]
fn test_curl_args_are_https_only_and_bounded() {
    let arguments = curl_download_args(
        INSTALL_SCRIPT_URL,
        Path::new("/tmp/install.sh"),
        Duration::from_secs(30),
    );
    assert!(arguments.iter().any(|argument| argument == "--proto"));
    assert!(arguments.iter().any(|argument| argument == "=https"));
    assert!(arguments.iter().any(|argument| argument == "--tlsv1.2"));
    assert!(arguments.iter().any(|argument| argument == "--max-redirs"));
    assert!(arguments.contains(&MAX_SCRIPT_BYTES.to_string()));
    assert!(arguments.iter().any(|argument| argument == "--max-time"));
    assert!(
        arguments
            .iter()
            .any(|argument| argument == "--max-filesize")
    );
    assert!(arguments.iter().any(|argument| argument == "--url"));
    assert_eq!(
        arguments.last().map(String::as_str),
        Some(INSTALL_SCRIPT_URL)
    );
}

#[test]
fn test_update_status_line_without_network() {
    assert_eq!(
        update_status_line(false, None, "0.1.4"),
        "skipped (--no-update-check)"
    );
    assert!(
        update_status_line(
            false,
            Some(&("9.9.9".to_string(), "X".to_string())),
            "0.1.4"
        )
        .contains("skipped")
    );
    let newer = ("9.9.9".to_string(), "Test".to_string());
    assert!(update_status_line(true, Some(&newer), "0.1.4").contains("Update available"));
    assert!(update_status_line(true, None, "0.1.4").contains("Offline"));
}

#[test]
fn test_save_config_creates_backup_and_valid_config() {
    let directory = temp_dir("save-config");
    let path = directory.join("config.toml");
    std::fs::write(&path, "# user config\n[general]\npadding = 3\n").unwrap();

    let backup = save_config(&Config::default(), &path).unwrap();
    assert!(backup.is_some(), "existing config must be backed up");
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(Config::from_toml(&written).is_ok());

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_save_config_reports_write_errors() {
    let directory = temp_dir("save-error");
    let blocker = directory.join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let path = blocker.join("config.toml");
    let error = save_config(&Config::default(), &path).unwrap_err();
    assert!(error.contains("Failed to create") || error.contains("Failed to back up"));
    let _ = std::fs::remove_dir_all(&directory);
}
