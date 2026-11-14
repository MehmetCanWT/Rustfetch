use super::*;

#[test]
fn test_default_config_has_all_modules() {
    let config = Config::default();
    assert_eq!(config.modules.len(), 18);

    let names: Vec<&str> = config.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "os", "host", "board", "kernel", "uptime", "packages", "shell", "desktop", "font",
            "terminal", "cpu", "gpu", "memory", "swap", "disk", "battery", "local_ip", "locale"
        ]
    );

    assert_eq!(config.general.separator, ":");
    assert_eq!(config.general.padding, 1);
    assert!(config.general.logo.enabled);
    assert_eq!(config.general.logo.distro, "auto");

    for m in &config.modules {
        assert!(m.label.is_none());
        assert!(m.icon.is_none());
        assert!(m.color.is_none());
    }
}

#[test]
fn test_parse_toml_string() {
    let toml_str = r#"
[general]
separator = " ->"
padding = 4

[general.logo]
enabled = false
distro = "fedora"

[[modules]]
name = "os"
label = "Distro"
icon = ""
color = "blue"

[[modules]]
name = "cpu"
icon = "󰍛"
"#;

    let config: Config = Config::from_toml(toml_str).expect("failed to parse valid TOML");
    assert_eq!(config.general.separator, " ->");
    assert_eq!(config.general.padding, 4);
    assert!(!config.general.logo.enabled);
    assert_eq!(config.general.logo.distro, "fedora");

    assert_eq!(config.modules.len(), 2);
    assert_eq!(config.modules[0].name, "os");
    assert_eq!(config.modules[0].label.as_deref(), Some("Distro"));
    assert_eq!(config.modules[0].icon.as_deref(), Some(""));
    assert_eq!(config.modules[0].color.as_deref(), Some("blue"));

    assert_eq!(config.modules[1].name, "cpu");
    assert_eq!(config.modules[1].label, None);
    assert_eq!(config.modules[1].icon.as_deref(), Some("󰍛"));
    assert_eq!(config.modules[1].color, None);
}

#[test]
fn test_parse_partial_toml_uses_defaults() {
    let toml_str = r#"
[general]
separator = " >"
"#;
    let config: Config = Config::from_toml(toml_str).expect("failed to parse partial TOML");
    assert_eq!(config.general.separator, " >");
    assert_eq!(config.general.padding, 1);
    assert!(config.general.logo.enabled);
    assert_eq!(config.general.logo.distro, "auto");
    assert_eq!(config.modules.len(), 18);
    assert_eq!(config.modules[0].name, "os");
}

#[test]
fn test_invalid_toml_gives_error() {
    let bad_syntax = "general = [unclosed bracket";
    let err = Config::from_toml(bad_syntax);
    assert!(err.is_err());
    let err_msg = err.unwrap_err().to_string();
    assert!(
        err_msg.contains("line") || err_msg.contains("error"),
        "Error message should mention location or error: {err_msg}"
    );

    let bad_type = r#"
[general]
separator = 12345
"#;
    let err2 = Config::from_toml(bad_type);
    assert!(err2.is_err());
    let err2_msg = err2.unwrap_err().to_string();
    assert!(
        err2_msg.contains("invalid type") || err2_msg.contains("expected a string"),
        "Error message should explain what was expected: {err2_msg}"
    );
}

#[test]
fn test_to_default_toml_round_trips() {
    let default_config = Config::default();
    let toml_str = Config::to_default_toml();

    assert!(!toml_str.is_empty());
    assert!(toml_str.contains("separator = \":\""));
    assert!(toml_str.contains("padding = 1"));
    assert!(toml_str.contains("enabled = true"));
    assert!(toml_str.contains("distro = \"auto\""));
    assert!(toml_str.contains("name = \"os\""));
    assert!(toml_str.contains("name = \"kernel\""));

    let deserialized: Config =
        Config::from_toml(&toml_str).expect("round-trip deserialization failed");
    assert_eq!(deserialized, default_config);
}

#[test]
fn test_config_dir_precedence() {
    let dir1 = Config::config_dir_from_env(|var| match var {
        "XDG_CONFIG_HOME" => Some("/custom/config".into()),
        "HOME" => Some("/home/user".into()),
        _ => None,
    });
    assert_eq!(dir1, PathBuf::from("/custom/config"));

    let dir2 = Config::config_dir_from_env(|var| match var {
        "XDG_CONFIG_HOME" => Some("   ".into()),
        "HOME" => Some("/home/user".into()),
        _ => None,
    });
    assert_eq!(dir2, PathBuf::from("/home/user/.config"));

    let dir3 = Config::config_dir_from_env(|var| match var {
        "XDG_CONFIG_HOME" => None,
        "HOME" => Some("/home/user".into()),
        _ => None,
    });
    assert_eq!(dir3, PathBuf::from("/home/user/.config"));

    let dir4 = Config::config_dir_from_env(|_| None);
    assert_eq!(dir4, PathBuf::from(".config"));
    assert!(!dir4.to_string_lossy().contains('~'));

    // config_path() must build on config_dir() and never emit a literal tilde.
    let path = Config::config_path();
    assert!(path.ends_with("rustfetch/config.toml"));
    assert!(!path.to_string_lossy().contains('~'));
}

#[test]
fn test_load_from_file_and_errors() {
    let temp_dir = std::env::temp_dir().join(format!("rustfetch_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let valid_file = temp_dir.join("valid.toml");
    std::fs::write(
        &valid_file,
        "[general]\npadding = 3\nseparator = \" -> \"\n",
    )
    .expect("write file");

    let loaded = Config::load_from(&valid_file).expect("load_from valid file");
    assert_eq!(loaded.general.padding, 3);
    assert_eq!(loaded.general.separator, " -> ");

    let invalid_file = temp_dir.join("invalid.toml");
    std::fs::write(&invalid_file, "[general]\npadding = \"not_a_number\"\n").expect("write file");

    let err = Config::load_from(&invalid_file).unwrap_err();
    assert!(err.contains("Failed to parse config file"));
    assert!(err.contains("line"));

    let nonexistent = temp_dir.join("nonexistent.toml");
    let not_found_err = Config::load_from(&nonexistent).unwrap_err();
    assert!(not_found_err.contains("Failed to read config file"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_strip_jsonc_comments_and_trailing_commas() {
    let input = r#"{
            // This is a line comment
            "name": "https://example.com/test//not_comment",
            /* This is a
               block comment */
            "items": [
                1,
                2,
                3, // trailing comma here
            ],
            "enabled": true,
        }"#;

    let cleaned = strip_jsonc(input);
    assert!(!cleaned.contains("This is a line comment"));
    assert!(!cleaned.contains("block comment"));
    assert!(cleaned.contains("https://example.com/test//not_comment"));

    let parsed: serde_json::Value =
        serde_json::from_str(&cleaned).expect("should parse valid json");
    assert_eq!(parsed["name"], "https://example.com/test//not_comment");
    assert_eq!(parsed["items"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["enabled"], true);
}

#[test]
fn test_import_from_fastfetch() {
    let ff_jsonc = r#"{
            "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/master/doc/json_schema.json",
            "logo": {
                "type": "kitty",
                "source": "~/.config/fastfetch/logos/current.png",
                "width": 60,
                "padding": {
                    "left": 47
                }
            },
            "display": {
                "separator": ": "
            },
            "modules": [
                {
                    "type": "custom",
                    "format": "\n\n\n\n\n\n"
                },
                {
                    "type": "title",
                    "keyIcon": ""
                },
                {
                    "type": "separator"
                },
                {
                    "type": "os",
                    "keyIcon": ""
                },
                {
                    "type": "cpu",
                    "keyIcon": ""
                },
                {
                    "type": "de",
                    "keyIcon": ""
                },
                {
                    "type": "wm",
                    "keyIcon": ""
                },
                {
                    "type": "localip",
                    "keyIcon": "󰩟"
                }
            ]
        }"#;

    let config = Config::import_from_fastfetch(ff_jsonc).expect("should import fastfetch config");
    assert_eq!(config.general.separator, ":");
    assert!(config.general.center);
    assert!(config.general.logo.random_image);
    assert_eq!(
        config.general.logo.image_dir.as_deref(),
        Some("~/.config/fastfetch/logos")
    );

    let mod_names: Vec<&str> = config.modules.iter().map(|m| m.name.as_str()).collect();
    // custom and divider preserved, de and wm deduplicated to a single desktop module
    assert_eq!(
        mod_names,
        vec!["custom", "divider", "os", "cpu", "desktop", "local_ip"]
    );

    let os_mod = config.modules.iter().find(|m| m.name == "os").unwrap();
    assert_eq!(os_mod.icon.as_deref(), Some(""));

    let cpu_mod = config.modules.iter().find(|m| m.name == "cpu").unwrap();
    assert_eq!(cpu_mod.icon.as_deref(), Some(""));

    let desktop_mod = config.modules.iter().find(|m| m.name == "desktop").unwrap();
    assert_eq!(desktop_mod.icon.as_deref(), Some(""));

    let local_ip_mod = config
        .modules
        .iter()
        .find(|m| m.name == "local_ip")
        .unwrap();
    assert_eq!(local_ip_mod.icon.as_deref(), Some("󰩟"));
}

#[test]
fn test_from_preset() {
    let minimal = Config::from_preset("minimal").unwrap();
    assert!(!minimal.general.icons);
    assert!(!minimal.general.logo.enabled);

    let card = Config::from_preset("card").unwrap();
    assert!(card.general.border);

    let modern = Config::from_preset("modern").unwrap();
    assert!(modern.general.colors.block);

    let compact = Config::from_preset("compact").unwrap();
    assert_eq!(compact.modules.len(), 7);

    assert!(Config::from_preset("nonexistent").is_none());
}

#[test]
fn test_module_config_text_and_logo_aliases() {
    let toml_str = r#"
        [[modules]]
        name = "os"
        text = "System"
        logo = ">_"

        [[modules]]
        name = "cpu"
        title = "Processor"
        symbol = ""
        "#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.modules[0].label.as_deref(), Some("System"));
    assert_eq!(config.modules[0].icon.as_deref(), Some(">_"));
    assert_eq!(config.modules[1].label.as_deref(), Some("Processor"));
    assert_eq!(config.modules[1].icon.as_deref(), Some(""));
}

#[test]
fn test_three_d_config_boolean_and_table() {
    let toml_bool = r#"
        [general.logo]
        3d = true
        "#;
    let cfg1: Config = toml::from_str(toml_bool).unwrap();
    assert!(cfg1.general.logo.three_d.enabled);

    let toml_table = r##"
        [general.logo.three_d]
        enabled = true
        speed = 2.0
        width = 60
        outer_color = "#3b82f6"
        "##;
    let cfg2: Config = toml::from_str(toml_table).unwrap();
    assert!(cfg2.general.logo.three_d.enabled);
    assert_eq!(cfg2.general.logo.three_d.speed, 2.0);
    assert_eq!(cfg2.general.logo.three_d.width, Some(60));
    assert_eq!(
        cfg2.general.logo.three_d.outer_color.as_deref(),
        Some("#3b82f6")
    );

    let toml_gen = r#"
        [general]
        3d = true
        "#;
    let cfg3: Config = toml::from_str(toml_gen).unwrap();
    assert_eq!(cfg3.general.three_d, Some(true));
}

#[test]
fn test_border_images_config_and_active_list() {
    let toml_str = r#"
        [general]
        border_image = "legacy_border.png"
        border_image_width = 30
        border_image_position = "left"

        [[general.border_images]]
        path = "card_backdrop.png"
        position = "background"
        width = 50
        height = 20
        offset_x = 2
        offset_y = -1
        z_index = -1
        protocol = "kitty"

        [[general.border_images]]
        path = "side_ornament.png"
        position = "right"
        "#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    let active = cfg.general.active_border_images();
    assert_eq!(active.len(), 3);

    // Legacy image normalized
    assert_eq!(active[0].path, "legacy_border.png");
    assert_eq!(active[0].position, "left");
    assert_eq!(active[0].width, Some(30));
    assert!(!active[0].is_background());

    // Multi-image 1 (background)
    assert_eq!(active[1].path, "card_backdrop.png");
    assert_eq!(active[1].position, "background");
    assert_eq!(active[1].width, Some(50));
    assert_eq!(active[1].height, Some(20));
    assert_eq!(active[1].offset_x, Some(2));
    assert_eq!(active[1].offset_y, Some(-1));
    assert_eq!(active[1].z_index, -1);
    assert_eq!(active[1].protocol, "kitty");
    assert!(active[1].is_background());

    // Multi-image 2 (right)
    assert_eq!(active[2].path, "side_ornament.png");
    assert_eq!(active[2].position, "right");
    assert_eq!(active[2].z_index, -1);
    assert!(!active[2].is_background());
}

#[test]
fn test_grouped_border_and_border_image_tables() {
    let toml_str = r#"
        [general]
        separator = " => "

        [general.border]
        enabled = true
        style = "double"
        color = "magenta"
        title = "MyBox"
        padding = 2

        [general.border_image]
        path = "wallpaper.png"
        position = "behind"
        width = 40
        height = 20
        offset_x = 1
        offset_y = 2
        z_index = -2
        protocol = "sixel"
        "#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.general.separator, " => ");
    assert!(cfg.general.border);
    assert_eq!(cfg.general.border_style.as_deref(), Some("double"));
    assert_eq!(cfg.general.border_color.as_deref(), Some("magenta"));
    assert_eq!(cfg.general.border_title.as_deref(), Some("MyBox"));
    assert_eq!(cfg.general.box_padding, 2);

    assert_eq!(cfg.general.border_image.as_deref(), Some("wallpaper.png"));
    assert_eq!(cfg.general.border_image_position, "behind");
    assert_eq!(cfg.general.border_image_width, Some(40));
    assert_eq!(cfg.general.border_image_height, Some(20));
    assert_eq!(cfg.general.border_image_offset_x, Some(1));
    assert_eq!(cfg.general.border_image_offset_y, Some(2));
    assert_eq!(cfg.general.border_image_z_index, Some(-2));
    assert_eq!(cfg.general.border_image_protocol.as_deref(), Some("sixel"));

    let active = cfg.general.active_border_images();
    assert_eq!(active.len(), 1);
    assert!(active[0].is_background());
}

#[test]
fn test_top_level_border_and_border_image_tables() {
    let toml_str = r#"
        [general]
        padding = 3

        [border]
        enabled = true
        style = "thick"

        [border_image]
        path = "bg.png"
        "#;

    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.general.padding, 3);
    assert!(cfg.general.border);
    assert_eq!(cfg.general.border_style.as_deref(), Some("thick"));
    assert_eq!(cfg.general.border_image.as_deref(), Some("bg.png"));
    assert_eq!(cfg.general.border_image_position, "behind");
}

#[test]
fn test_border_image_default_position_is_behind() {
    let cfg = Config::default();
    assert_eq!(cfg.general.border_image_position, "behind");
    let bi = BorderImageConfig::default();
    assert_eq!(bi.position, "behind");
    assert!(bi.is_background());
}

#[test]
fn test_check_updates_config_and_aliases() {
    let cfg = Config::default();
    assert!(cfg.general.check_updates);

    let disabled: Config = toml::from_str("[general]\ncheck_updates = false\n").unwrap();
    assert!(!disabled.general.check_updates);

    let alias1: Config = toml::from_str("[general]\nupdate_check = false\n").unwrap();
    assert!(!alias1.general.check_updates);

    let alias2: Config = toml::from_str("[general]\nnotify_updates = false\n").unwrap();
    assert!(!alias2.general.check_updates);
}

#[test]
fn test_write_if_not_exists_never_overwrites_user_config() {
    let dir = std::env::temp_dir().join(format!("rustfetch_cfg_preserve_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cfg_path = dir.join("config.toml");

    let user_custom = "[general]\nseparator = \" -> \"\ntheme = \"catppuccin\"\n";
    assert!(Config::write_if_not_exists(&cfg_path, user_custom));
    assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), user_custom);

    // A subsequent call (e.g. after an update) MUST NOT overwrite the existing file
    assert!(!Config::write_if_not_exists(
        &cfg_path,
        &Config::to_default_toml()
    ));
    assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), user_custom);

    // And backup_if_exists creates config.toml.bak
    let bak = Config::backup_if_exists(&cfg_path).expect("backup should be created");
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), user_custom);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_salvage_partial_toml_and_shorthand_modules() {
    // Shorthand string modules + border style string
    let shorthand = r#"
        [general]
        separator = " :: "
        border = "rounded"
        modules = ["os", "kernel", "cpu"]
        "#;
    // Note: modules at top level vs under general
    let shorthand_top = r#"
        modules = ["os", "kernel", "cpu"]
        [general]
        separator = " :: "
        border = "rounded"
        "#;
    let cfg = Config::from_toml(shorthand_top).unwrap();
    assert_eq!(cfg.general.separator, " :: ");
    assert!(cfg.general.border);
    assert_eq!(cfg.general.border_style.as_deref(), Some("rounded"));
    assert_eq!(cfg.modules.len(), 3);
    assert_eq!(cfg.modules[0].name, "os");
    let _ = shorthand;

    // Fault-tolerant salvage when one field has an incompatible type
    let partially_broken = r#"
        [general]
        separator = " >> "
        theme = "dracula"
        padding = "invalid_number_type"
        center = true

        [[modules]]
        name = "os"

        [[modules]]
        name = "cpu"
        "#;
    let salvaged = Config::from_toml(partially_broken).expect("should salvage valid fields");
    assert_eq!(salvaged.general.separator, " >> ");
    assert_eq!(salvaged.general.theme.as_deref(), Some("dracula"));
    assert!(salvaged.general.center);
    assert_eq!(salvaged.modules.len(), 2);
    assert_eq!(salvaged.modules[0].name, "os");
    assert_eq!(salvaged.modules[1].name, "cpu");
}

#[test]
fn test_gpu_config_formats() {
    // 1. [gpu] top-level table
    let toml_gpu_top = r#"
        [gpu]
        type = "igpu"
        "#;
    let cfg1 = Config::from_toml(toml_gpu_top).unwrap();
    assert_eq!(cfg1.gpu.r#type, "igpu");
    let gpu_mod1 = cfg1.modules.iter().find(|m| m.name == "gpu").unwrap();
    assert_eq!(gpu_mod1.gpu_type.as_deref(), Some("igpu"));

    // 2. [modules.gpu] sub-table
    let toml_modules_gpu = r#"
        [modules.gpu]
        type = "all"
        "#;
    let cfg2 = Config::from_toml(toml_modules_gpu).unwrap();
    assert_eq!(cfg2.gpu.r#type, "all");
    let gpu_mod2 = cfg2.modules.iter().find(|m| m.name == "gpu").unwrap();
    assert_eq!(gpu_mod2.gpu_type.as_deref(), Some("all"));

    // 3. [[modules]] table with name = "gpu" and type = "igpu"
    let toml_item_type = r#"
        [[modules]]
        name = "gpu"
        type = "igpu"
        "#;
    let cfg3 = Config::from_toml(toml_item_type).unwrap();
    assert_eq!(cfg3.modules[0].name, "gpu");
    assert_eq!(cfg3.modules[0].gpu_type.as_deref(), Some("igpu"));

    // 4. [[modules]] table with name = "gpu" and gpu_type = "dgpu"
    let toml_item_gpu_type = r#"
        [[modules]]
        name = "gpu"
        gpu_type = "dgpu"
        "#;
    let cfg4 = Config::from_toml(toml_item_gpu_type).unwrap();
    assert_eq!(cfg4.modules[0].name, "gpu");
    assert_eq!(cfg4.modules[0].gpu_type.as_deref(), Some("dgpu"));

    // 5. Dedicated dgpu and igpu modules
    let toml_dedicated = r#"
        [[modules]]
        name = "igpu"
        label = "Internal Graphics"

        [[modules]]
        name = "dgpu"
        label = "Dedicated Graphics"
        "#;
    let cfg5 = Config::from_toml(toml_dedicated).unwrap();
    assert_eq!(cfg5.modules.len(), 2);
    assert_eq!(cfg5.modules[0].name, "igpu");
    assert_eq!(cfg5.modules[0].label.as_deref(), Some("Internal Graphics"));
    assert_eq!(cfg5.modules[1].name, "dgpu");
    assert_eq!(cfg5.modules[1].label.as_deref(), Some("Dedicated Graphics"));

    // 6. [[modules]] followed by [modules.gpu]
    let toml_modules_gpu_nested = r#"
        [[modules]]
        name = "os"

        [[modules]]
        name = "gpu"

        [modules.gpu]
        type = "igpu"
        "#;
    let cfg6 = Config::from_toml(toml_modules_gpu_nested).unwrap();
    assert_eq!(cfg6.modules.len(), 2);
    assert_eq!(cfg6.modules[0].name, "os");
    assert_eq!(cfg6.modules[1].name, "gpu");
    assert_eq!(cfg6.modules[1].gpu_type.as_deref(), Some("igpu"));

    // 7. filter and display aliases
    let toml_filter = r#"
        [[modules]]
        name = "gpu"
        filter = "all"
        "#;
    let cfg7 = Config::from_toml(toml_filter).unwrap();
    assert_eq!(cfg7.modules[0].gpu_type.as_deref(), Some("all"));

    let toml_display = r#"
        [[modules]]
        name = "gpu"
        display = "discrete"
        "#;
    let cfg8 = Config::from_toml(toml_display).unwrap();
    assert_eq!(cfg8.modules[0].gpu_type.as_deref(), Some("discrete"));

    // 8. Icons for dgpu and igpu
    assert_eq!(default_icon_for_module("gpu"), Some("󰢮"));
    assert_eq!(default_icon_for_module("dgpu"), Some("󰢮"));
    assert_eq!(default_icon_for_module("igpu"), Some("󰢮"));
}

#[test]
fn test_clamp_values_from_toml() {
    let toml_str = r#"
        [general]
        padding = 999
        box_padding = 99
        border_image_width = 0
        border_image_height = 4096
        border_image_offset_x = -999999
        border_image_z_index = 999

        [general.colors]
        rows = 9

        [general.logo]
        image_width_cols = 9999

        [general.logo.three_d]
        size = 9.0
        speed = 0.001
        depth = 99.0
        width = 99999
        height = 0

        [general.logo.animation]
        fps = 999.0

        [[modules]]
        name = "memory"
        bar_width = 0
        "#;
    let cfg = Config::from_toml(toml_str).expect("valid TOML");
    assert_eq!(cfg.general.padding, 64);
    assert_eq!(cfg.general.box_padding, 16);
    assert_eq!(cfg.general.colors.rows, 2);
    assert_eq!(cfg.general.logo.image_width_cols, 256);
    assert_eq!(cfg.general.logo.three_d.size, 3.0);
    assert_eq!(cfg.general.logo.three_d.speed, 0.05);
    assert_eq!(cfg.general.logo.three_d.depth, 5.0);
    assert_eq!(cfg.general.logo.three_d.width, Some(512));
    assert_eq!(cfg.general.logo.three_d.height, Some(1));
    assert_eq!(cfg.general.logo.animation.fps, 120.0);
    assert_eq!(cfg.general.border_image_width, Some(1));
    assert_eq!(cfg.general.border_image_height, Some(512));
    assert_eq!(cfg.general.border_image_offset_x, Some(-10_000));
    assert_eq!(cfg.general.border_image_z_index, Some(100));
    assert_eq!(cfg.modules[0].bar_width, Some(1));
    let warnings = cfg.warnings();
    assert!(warnings.iter().any(|w| w.contains("general.padding")));
    assert!(warnings.iter().any(|w| w.contains("bar_width")));
}

#[test]
fn test_non_finite_floats_fall_back_to_defaults() {
    let toml_str = r#"
        [general.logo.three_d]
        size = nan
        speed = inf
        "#;
    let cfg = Config::from_toml(toml_str).expect("valid TOML");
    assert_eq!(cfg.general.logo.three_d.size, 1.0);
    assert_eq!(cfg.general.logo.three_d.speed, 1.0);
    let warnings = cfg.warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("general.logo.three_d.size") && w.contains("finite"))
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("general.logo.three_d.speed") && w.contains("finite"))
    );
}

#[test]
fn test_shading_mode_and_bar_style_whitelists() {
    let toml_str = r#"
        [general.logo.three_d]
        shading_mode = "hologram"

        [[modules]]
        name = "memory"
        bar_style = "rainbow"

        [[modules]]
        name = "disk"
        bar_style = "line"
        "#;
    let cfg = Config::from_toml(toml_str).expect("valid TOML");
    assert_eq!(cfg.general.logo.three_d.shading_mode, "ascii");
    assert_eq!(cfg.modules[0].bar_style, None);
    assert_eq!(cfg.modules[1].bar_style.as_deref(), Some("line"));
    let warnings = cfg.warnings();
    assert!(warnings.iter().any(|w| w.contains("shading_mode")));
    assert!(warnings.iter().any(|w| w.contains("bar_style")));
}

#[test]
fn test_border_chars_compact_string_and_table() {
    let compact = "[general]\nborder_chars = \"┌─┐│┘─└│├─┤\"\n";
    let cfg = Config::from_toml(compact).expect("compact border_chars must parse");
    let chars = cfg.general.border_chars.expect("border_chars set");
    assert_eq!(chars.top_left, "┌");
    assert_eq!(chars.top_right, "┐");
    assert_eq!(chars.bottom_right, "┘");
    assert_eq!(chars.divider_right, "┤");

    let table = r#"
        [general.border_chars]
        top_left = "X"
        top = "y"
        "#;
    let cfg_table = Config::from_toml(table).expect("table border_chars must parse");
    let chars_table = cfg_table.general.border_chars.expect("border_chars set");
    assert_eq!(chars_table.top_left, "X");
    assert_eq!(chars_table.divider_right, "┤");

    let invalid = "[general]\nborder_chars = \"┌─┐\"\n";
    assert!(Config::from_toml(invalid).is_err());
}

#[test]
fn test_preset_layering_and_unknown_preset() {
    let toml_str = r#"
        preset = "card"

        [general]
        padding = 3
        separator = "!"
        "#;
    let cfg = Config::from_toml(toml_str).expect("valid TOML");
    assert!(cfg.general.border);
    assert!(cfg.general.center);
    assert_eq!(cfg.general.padding, 3);
    assert_eq!(cfg.general.separator, "!");
    assert!(cfg.modules.iter().any(|m| m.name == "host"));
    assert!(cfg.warnings().is_empty(), "warnings: {:?}", cfg.warnings());

    let unknown = "preset = \"neon-void\"\n";
    let cfg_unknown = Config::from_toml(unknown).expect("valid TOML");
    assert!(cfg_unknown.warnings().iter().any(|w| w.contains("preset")));
    assert_eq!(cfg_unknown.modules, Config::default().modules);
}

#[test]
fn test_unknown_top_level_key_warns() {
    let toml_str = r#"
        unknown_thing = 1

        [general]
        padding = 1
        "#;
    let cfg = Config::from_toml(toml_str).expect("unknown keys must not fail parsing");
    let warnings = cfg.warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("unknown top-level key 'unknown_thing'"))
    );
}

#[test]
fn test_unknown_module_and_theme_warnings() {
    let toml_str = r#"
        [general]
        theme = "neon-void"

        [[modules]]
        name = "not_a_module"

        [[modules]]
        name = "disk:/home"
        "#;
    let cfg = Config::from_toml(toml_str).expect("valid TOML");
    let warnings = cfg.warnings();
    assert!(warnings.iter().any(|w| w.contains("not_a_module")));
    assert!(warnings.iter().any(|w| w.contains("neon-void")));
    assert!(is_known_module_name("disk:/home"));
    assert!(is_known_theme_name("tokyonight"));
    assert!(is_known_theme_name("rose_pine"));
    assert!(!is_known_theme_name("definitely-not-a-theme"));
}

#[test]
fn test_default_config_has_no_warnings_and_public_clamp() {
    let mut cfg = Config::default();
    assert!(cfg.warnings().is_empty(), "warnings: {:?}", cfg.warnings());
    cfg.general.padding = 1000;
    cfg.general.logo.three_d.shading_mode = "bogus".to_string();
    cfg.clamp();
    assert_eq!(cfg.general.padding, 64);
    assert_eq!(cfg.general.logo.three_d.shading_mode, "ascii");
    assert!(!cfg.warnings().is_empty());
}

#[test]
fn test_to_toml_string_round_trip_with_tables_and_optional_scalars() {
    let mut cfg = Config::default();
    cfg.general.border_images.push(BorderImageConfig {
        path: "/tmp/example.png".to_string(),
        position: "top".to_string(),
        width: Some(12),
        ..BorderImageConfig::default()
    });
    cfg.general.gradient = Some(vec!["#ff0000".to_string(), "#00ff00".to_string()]);
    cfg.general.theme = Some("catppuccin".to_string());
    cfg.general.three_d = Some(true);

    let serialized = cfg
        .to_toml_string()
        .expect("serialization must not fail with ValueAfterTable");
    let parsed = Config::from_toml(&serialized).expect("round-trip TOML must parse");
    assert_eq!(parsed.general.three_d, Some(true));
    assert_eq!(
        parsed.general.gradient.as_deref(),
        Some(&["#ff0000".to_string(), "#00ff00".to_string()][..])
    );
    assert_eq!(parsed.general.theme.as_deref(), Some("catppuccin"));
    assert_eq!(parsed.general.border_images.len(), 1);
}

#[test]
fn test_aliases_and_salvage_preserve_preset_and_modules_list() {
    assert_eq!(canonical_shading_mode("halfblock"), Some("blocks"));
    assert!(is_known_module_name("network"));

    let broken_toml = r#"
preset = "card"

[general]
padding = "not-a-number"

[modules]
list = [
    { name = "os" },
    { name = "network" },
]
"#;
    let salvaged = Config::from_toml(broken_toml).expect("broken TOML should salvage");
    assert!(salvaged.general.border);
    assert_eq!(salvaged.modules.len(), 2);
    assert_eq!(salvaged.modules[0].name, "os");
    assert_eq!(salvaged.modules[1].name, "network");
}
