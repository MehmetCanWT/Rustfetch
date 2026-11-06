use crate::proc::{run_with_timeout, OPTIONAL_SOURCE_TIMEOUT};
use crate::{Info, Module};

pub struct Media;

impl Module for Media {
    fn name(&self) -> &'static str {
        "media"
    }

    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        get_mpris_media()
    }
}

pub fn parse_playerctl_output(stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed == "-" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn get_mpris_media() -> Option<Info> {
    let output = run_with_timeout(
        "playerctl",
        &["metadata", "--format", "{{artist}} - {{title}}"],
        OPTIONAL_SOURCE_TIMEOUT,
    )?;
    if !output.success() {
        return None;
    }
    let text = output.stdout_string();
    parse_playerctl_output(&text).map(|val| Info::new("Media", val))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_module_name() {
        assert_eq!(Media.name(), "media");
    }

    #[test]
    fn test_parse_playerctl_output() {
        assert_eq!(
            parse_playerctl_output("Queen - Bohemian Rhapsody\n"),
            Some("Queen - Bohemian Rhapsody".into())
        );
        assert_eq!(parse_playerctl_output(""), None);
        assert_eq!(parse_playerctl_output("   "), None);
        assert_eq!(parse_playerctl_output("-"), None);
    }
}
