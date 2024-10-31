use std::fs;

use hyprparser::{parse_config, HyprlandConfig};

const TEST_CONFIG_FILENAME_0: &str = "tests/test_config_0.conf";
const TEST_CONFIG_FILENAME_1: &str = "tests/test_config_1.conf";

#[test]
fn config_parsing() {
    let _ = parse_config(&fs::read_to_string(TEST_CONFIG_FILENAME_0).unwrap());
}

#[test]
fn entry_adding() {
    let config_parsed = parse_config(&fs::read_to_string(TEST_CONFIG_FILENAME_0).unwrap());
    let mut config = HyprlandConfig::new();

    config.add_entry("envcursor", "no_hardware_cursors = true");
    config.add_entry("input", "accel_profile = flat");
    config.add_entry("general", "gaps_in = 5");
    config.add_entry("general", "gaps_out = 20");
    config.add_entry("general", "col.active_border = rgb(BDBDBD)");
    config.add_entry("general", "allow_tearing = false");
    config.add_entry("general", "layout = master");
    config.add_entry_headless("$terminal", "kitty");
    config.add_entry_headless("", "");
    config.add_entry_headless("$mainMod", "super");
    config.add_entry_headless("", "");
    config.add_entry_headless("bind", "$mainMod, RETURN, exec, $terminal");
    config.add_entry_headless("", "");
    config.add_entry_headless("bind", "$mainMod, 1, workspace, 1");
    config.add_entry_headless("", "");
    config.add_entry_headless("bind", "$mainMod SHIFT, 1, movetoworkspacesilent, 1");
    config.add_entry_headless("", "");
    config.add_entry_headless("bindm", "$mainMod, mouse:272, movewindow");
    config.add_entry_headless("bindm", "$mainMod, mouse:273, resizewindow");

    assert_eq!(config_parsed, config)
}

#[test]
fn color_parsing() {
    let config = HyprlandConfig::new();

    let expected = Some((0.11764706, 0.27450982, 0.19607843, 1.0));

    let rgba_parsed = config.parse_color("rgba(1E4632FF)");
    let rgb_parsed = config.parse_color("rgb(1E4632)");
    let argb_parsed = config.parse_color("0xFF1E4632");

    assert_eq!(expected, rgba_parsed);
    assert_eq!(expected, rgb_parsed);
    assert_eq!(expected, argb_parsed);
}

#[test]
fn file_sourcing() {
    let config_parsed = parse_config(&fs::read_to_string(TEST_CONFIG_FILENAME_1).unwrap());
    let mut config = HyprlandConfig::new();

    config.add_entry("xwayland", "force_zero_scaling = true");
    config.add_entry_headless(
        "source",
        &format!("{}/tests/test_config_2.conf", env!("CARGO_MANIFEST_DIR")),
    );

    assert_eq!(config_parsed, config)
}
