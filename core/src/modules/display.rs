use crate::{Info, Module};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Display;

impl Module for Display {
    fn name(&self) -> &'static str {
        "display"
    }

    fn detect(&self) -> Option<Info> {
        let displays = detect_displays();
        if displays.is_empty() {
            None
        } else {
            Some(Info::new("Display", displays.join(", ")))
        }
    }
}

/// One EDID detailed timing descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdidTiming {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

pub fn detect_displays() -> Vec<String> {
    detect_displays_from(Path::new("/sys/class/drm"))
}

/// Reads every connected DRM connector below the given root.
pub fn detect_displays_from(drm_root: &Path) -> Vec<String> {
    let mut results = Vec::new();
    let Ok(entries) = fs::read_dir(drm_root) else {
        return results;
    };

    let mut connectors: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    connectors.sort_by(|a, b| {
        let a_str = a
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let b_str = b
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        a_str.cmp(&b_str)
    });

    for path in connectors {
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !fname.starts_with("card") || !fname.contains('-') {
            continue;
        }

        let status = fs::read_to_string(path.join("status"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if status != "connected" {
            continue;
        }

        let connector_name = fname
            .split_once('-')
            .map(|(_, name)| name.to_string())
            .unwrap_or(fname);

        // Preferred: parse the EDID detailed timing descriptor.
        let mut display_spec = fs::read(path.join("edid"))
            .ok()
            .and_then(|bytes| parse_edid(&bytes))
            .map(|timing| format_timing(&timing));

        // Fallback: the first advertised mode (no refresh rate available).
        if display_spec.is_none() {
            display_spec = fs::read_to_string(path.join("modes"))
                .ok()
                .and_then(|modes| {
                    modes
                        .lines()
                        .map(str::trim)
                        .find(|line| !line.is_empty())
                        .map(str::to_string)
                });
        }

        if let Some(spec) = display_spec {
            results.push(format!("{spec} ({connector_name})"));
        }
    }

    results
}

fn format_timing(timing: &EdidTiming) -> String {
    if timing.refresh_hz > 0 {
        format!(
            "{}x{} @ {}Hz",
            timing.width, timing.height, timing.refresh_hz
        )
    } else {
        format!("{}x{}", timing.width, timing.height)
    }
}

const EDID_HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
const EDID_BLOCK_LEN: usize = 128;
const DTD_LEN: usize = 18;
const DTD_SLOTS: [usize; 4] = [54, 72, 90, 108];
const CTA_861_TAG: u8 = 0x02;

/// Validates the 128-byte block checksum (all bytes must sum to 0 mod 256).
pub fn edid_checksum_ok(block: &[u8]) -> bool {
    block.len() >= EDID_BLOCK_LEN
        && block[..EDID_BLOCK_LEN]
            .iter()
            .fold(0u8, |acc, byte| acc.wrapping_add(*byte))
            == 0
}

/// Parses the first valid detailed timing from an EDID 1.x blob.
///
/// The base block (four DTD slots) is tried first; when it contains only
/// monitor descriptors (pixel clock 0), CTA-861 extension blocks are searched
/// so monitors that put their preferred timing in an extension still report a
/// resolution and refresh rate.
pub fn parse_edid(data: &[u8]) -> Option<EdidTiming> {
    if data.len() < EDID_BLOCK_LEN {
        return None;
    }
    if data[..8] != EDID_HEADER {
        return None;
    }
    // EDID major version must be 1 (1.3 / 1.4 are what Linux exposes).
    if data[18] != 1 {
        return None;
    }
    if !edid_checksum_ok(&data[..EDID_BLOCK_LEN]) {
        return None;
    }

    for slot in DTD_SLOTS {
        let end = slot + DTD_LEN;
        if end <= EDID_BLOCK_LEN {
            if let Some(timing) = parse_dtd(&data[slot..end]) {
                return Some(timing);
            }
        }
    }

    let extension_count = data[126] as usize;
    for index in 1..=extension_count {
        let start = index * EDID_BLOCK_LEN;
        let end = start + EDID_BLOCK_LEN;
        if end > data.len() {
            break;
        }
        let block = &data[start..end];
        if !edid_checksum_ok(block) || block[0] != CTA_861_TAG {
            continue;
        }
        let dtd_offset = block[2] as usize;
        if !(4..EDID_BLOCK_LEN).contains(&dtd_offset) {
            continue;
        }
        let mut slot = start + dtd_offset;
        while slot + DTD_LEN <= end {
            if let Some(timing) = parse_dtd(&data[slot..slot + DTD_LEN]) {
                return Some(timing);
            }
            slot += DTD_LEN;
        }
    }

    None
}

fn parse_dtd(dtd: &[u8]) -> Option<EdidTiming> {
    if dtd.len() < DTD_LEN {
        return None;
    }
    let pixel_clock = (dtd[0] as u32 | ((dtd[1] as u32) << 8)) * 10_000;
    if pixel_clock == 0 {
        return None;
    }

    let h_active = dtd[2] as u32 | (((dtd[4] as u32) & 0xF0) << 4);
    let h_blank = dtd[3] as u32 | (((dtd[4] as u32) & 0x0F) << 8);
    let v_active = dtd[5] as u32 | (((dtd[7] as u32) & 0xF0) << 4);
    let v_blank = dtd[6] as u32 | (((dtd[7] as u32) & 0x0F) << 8);

    let h_total = h_active + h_blank;
    let v_total = v_active + v_blank;

    if h_active == 0 || v_active == 0 || h_total == 0 || v_total == 0 {
        return None;
    }
    if h_active > 8192 || v_active > 8192 {
        return None;
    }

    let refresh_hz = (pixel_clock as f64 / (h_total * v_total) as f64).round() as u32;
    if !(1..=1000).contains(&refresh_hz) {
        return None;
    }

    Some(EdidTiming {
        width: h_active,
        height: v_active,
        refresh_hz,
    })
}

/// Backwards compatible string formatter.
pub fn parse_edid_resolution_and_rate(data: &[u8]) -> Option<String> {
    parse_edid(data).map(|timing| format_timing(&timing))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_display_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn base_edid() -> Vec<u8> {
        let mut data = vec![0u8; EDID_BLOCK_LEN];
        data[..8].copy_from_slice(&EDID_HEADER);
        data[18] = 1;
        data[19] = 4;
        data
    }

    fn set_checksum(block: &mut [u8]) {
        let sum = block[..EDID_BLOCK_LEN - 1]
            .iter()
            .fold(0u8, |acc, byte| acc.wrapping_add(*byte));
        block[EDID_BLOCK_LEN - 1] = 0u8.wrapping_sub(sum);
    }

    fn put_dtd(block: &mut [u8], slot: usize, clock_10khz: u16, h: u16, hb: u16, v: u16, vb: u16) {
        block[slot] = (clock_10khz & 0xFF) as u8;
        block[slot + 1] = (clock_10khz >> 8) as u8;
        block[slot + 2] = (h & 0xFF) as u8;
        block[slot + 3] = (hb & 0xFF) as u8;
        block[slot + 4] = (((h >> 8) & 0xF) as u8) << 4 | (((hb >> 8) & 0xF) as u8);
        block[slot + 5] = (v & 0xFF) as u8;
        block[slot + 6] = (vb & 0xFF) as u8;
        block[slot + 7] = (((v >> 8) & 0xF) as u8) << 4 | (((vb >> 8) & 0xF) as u8);
    }

    fn full_hd_60() -> Vec<u8> {
        let mut data = base_edid();
        put_dtd(&mut data, 54, 14850, 1920, 280, 1080, 45);
        set_checksum(&mut data);
        data
    }

    #[test]
    fn test_display_module_name() {
        assert_eq!(Display.name(), "display");
    }

    #[test]
    fn test_parse_edid_short_data() {
        assert!(parse_edid(&[]).is_none());
        assert!(parse_edid(&[0u8; 10]).is_none());
        assert!(parse_edid(&[0u8; 72]).is_none());
    }

    #[test]
    fn test_parse_edid_valid_synthetic() {
        let data = full_hd_60();
        assert_eq!(
            parse_edid(&data),
            Some(EdidTiming {
                width: 1920,
                height: 1080,
                refresh_hz: 60
            })
        );
        assert_eq!(
            parse_edid_resolution_and_rate(&data),
            Some("1920x1080 @ 60Hz".to_string())
        );
    }

    #[test]
    fn test_parse_edid_rejects_bad_header() {
        let mut data = full_hd_60();
        data[0] = 0xAA;
        assert!(parse_edid(&data).is_none());
    }

    #[test]
    fn test_parse_edid_rejects_bad_checksum() {
        let mut data = full_hd_60();
        data[127] = data[127].wrapping_add(1);
        assert!(!edid_checksum_ok(&data));
        assert!(parse_edid(&data).is_none());
    }

    #[test]
    fn test_parse_edid_rejects_unknown_version() {
        let mut data = full_hd_60();
        data[18] = 0;
        assert!(parse_edid(&data).is_none());
    }

    #[test]
    fn test_parse_edid_monitor_descriptor_first_slot() {
        let mut data = base_edid();
        // Slot 0 is a display range limits descriptor: pixel clock 0, tag 0xFD.
        data[54] = 0;
        data[55] = 0;
        data[59] = 0xFD;
        // The real timing lives in the second slot.
        put_dtd(&mut data, 72, 14850, 1920, 280, 1080, 45);
        set_checksum(&mut data);
        assert_eq!(
            parse_edid(&data),
            Some(EdidTiming {
                width: 1920,
                height: 1080,
                refresh_hz: 60
            })
        );
    }

    #[test]
    fn test_parse_edid_prefers_first_slot_over_extension() {
        let mut data = full_hd_60();
        data[126] = 1;
        data.extend(vec![0u8; EDID_BLOCK_LEN]);
        let ext_start = EDID_BLOCK_LEN;
        data[ext_start] = CTA_861_TAG;
        data[ext_start + 1] = 0x03;
        data[ext_start + 2] = 4;
        put_dtd(&mut data, ext_start + 4, 7425, 1280, 370, 720, 30);
        set_checksum(&mut data[..EDID_BLOCK_LEN]);
        set_checksum(&mut data[ext_start..]);
        assert_eq!(
            parse_edid(&data).map(|timing| (timing.width, timing.height)),
            Some((1920, 1080))
        );
    }

    #[test]
    fn test_parse_edid_extension_only_timing() {
        let mut data = base_edid();
        data[126] = 1;
        set_checksum(&mut data);
        data.extend(vec![0u8; EDID_BLOCK_LEN]);
        let ext_start = EDID_BLOCK_LEN;
        data[ext_start] = CTA_861_TAG;
        data[ext_start + 1] = 0x03;
        data[ext_start + 2] = 4;
        put_dtd(&mut data, ext_start + 4, 7425, 1280, 370, 720, 30);
        set_checksum(&mut data[ext_start..]);
        assert_eq!(
            parse_edid_resolution_and_rate(&data),
            Some("1280x720 @ 60Hz".to_string())
        );
    }

    #[test]
    fn test_parse_edid_extension_bad_checksum_is_ignored() {
        let mut data = base_edid();
        data[126] = 1;
        set_checksum(&mut data);
        data.extend(vec![0u8; EDID_BLOCK_LEN]);
        let ext_start = EDID_BLOCK_LEN;
        data[ext_start] = CTA_861_TAG;
        data[ext_start + 2] = 4;
        // Deliberately do not fix the extension checksum.
        assert!(parse_edid(&data).is_none());
    }

    #[test]
    fn test_parse_edid_rejects_zero_active_pixels() {
        let mut data = base_edid();
        put_dtd(&mut data, 54, 14850, 0, 280, 1080, 45);
        set_checksum(&mut data);
        assert!(parse_edid(&data).is_none());
    }

    #[test]
    fn test_detect_displays_from_fake_drm() {
        let root = unique_dir("drm");
        let edid_connector = root.join("card1-eDP-1");
        fs::create_dir_all(&edid_connector).unwrap();
        fs::write(edid_connector.join("status"), "connected\n").unwrap();
        fs::write(edid_connector.join("edid"), full_hd_60()).unwrap();

        let modes_connector = root.join("card1-HDMI-A-1");
        fs::create_dir_all(&modes_connector).unwrap();
        fs::write(modes_connector.join("status"), "connected\n").unwrap();
        fs::write(modes_connector.join("modes"), "2560x1440\n1920x1080\n").unwrap();

        let disconnected = root.join("card1-DP-1");
        fs::create_dir_all(&disconnected).unwrap();
        fs::write(disconnected.join("status"), "disconnected\n").unwrap();
        fs::write(disconnected.join("edid"), full_hd_60()).unwrap();

        let missing_status = root.join("card1-DP-2");
        fs::create_dir_all(&missing_status).unwrap();

        let displays = detect_displays_from(&root);
        assert_eq!(
            displays,
            vec![
                "1920x1080 @ 60Hz (eDP-1)".to_string(),
                "2560x1440 (HDMI-A-1)".to_string(),
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_displays_from_missing_root() {
        assert!(detect_displays_from(Path::new("/nonexistent-drm")).is_empty());
    }
}
