use crate::proc::{OPTIONAL_SOURCE_TIMEOUT, run_with_timeout};
use crate::{Info, Module};
use std::fs;

pub struct Sound;

impl Module for Sound {
    fn name(&self) -> &'static str {
        "sound"
    }

    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        if let Some(vol) = get_wpctl_volume() {
            return Some(Info::new("Sound", vol));
        }
        let card = parse_asound_cards(&fs::read_to_string("/proc/asound/cards").ok()?)?;
        Some(Info::new("Sound (card)", card))
    }
}

pub fn parse_wpctl_output(output: &str) -> Option<String> {
    let line = output.lines().next()?;
    let rest = line.strip_prefix("Volume:")?.trim();
    let mut parts = rest.split_whitespace();
    let vol_float: f64 = parts.next()?.parse().ok()?;
    if !vol_float.is_finite() || vol_float < 0.0 {
        return None;
    }
    let pct = (vol_float * 100.0).round().clamp(0.0, 500.0) as u32;
    if rest.contains("[MUTED]") {
        Some(format!("{pct}% [MUTED]"))
    } else {
        Some(format!("{pct}%"))
    }
}

/// Parses the first entry of /proc/asound/cards into its card description.
///
/// Sample line:
///   0 [PCH            ]: HDA-Intel - HDA Intel PCH
/// The driver token (HDA-Intel) contains a dash of its own, so the split must
/// happen on the " - " separator and not on the first dash.
pub fn parse_asound_cards(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some((_, right)) = line.split_once("]: ")
            && let Some((_, desc)) = right.split_once(" - ")
        {
            let desc = desc.trim();
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

fn get_wpctl_volume() -> Option<String> {
    let output = run_with_timeout(
        "wpctl",
        &["get-volume", "@DEFAULT_AUDIO_SINK@"],
        OPTIONAL_SOURCE_TIMEOUT,
    )?;
    if output.success() {
        parse_wpctl_output(&output.stdout_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_module_name() {
        assert_eq!(Sound.name(), "sound");
    }

    #[test]
    fn test_parse_wpctl_output() {
        assert_eq!(parse_wpctl_output("Volume: 0.65"), Some("65%".into()));
        assert_eq!(
            parse_wpctl_output("Volume: 1.00 [MUTED]"),
            Some("100% [MUTED]".into())
        );
        assert_eq!(parse_wpctl_output("Invalid"), None);
    }

    #[test]
    fn test_parse_asound_cards_hda() {
        let content = " 0 [NVidia         ]: HDA-Intel - HDA NVidia\n                      HDA NVidia at 0x53000000 irq 17\n";
        assert_eq!(parse_asound_cards(content), Some("HDA NVidia".into()));
    }

    #[test]
    fn test_parse_asound_cards_usb() {
        let content = " 1 [Device         ]: USB-Audio - Plantronics Blackwire 3220\n";
        assert_eq!(
            parse_asound_cards(content),
            Some("Plantronics Blackwire 3220".into())
        );
    }

    #[test]
    fn test_parse_asound_cards_empty() {
        assert_eq!(parse_asound_cards(""), None);
        assert_eq!(parse_asound_cards("--- no soundcards ---\n"), None);
    }
}
