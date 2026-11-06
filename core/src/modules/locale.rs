use crate::{Info, Module};

pub struct Locale;

impl Module for Locale {
    fn name(&self) -> &'static str {
        "locale"
    }

    fn detect(&self) -> Option<Info> {
        let locale_conf = std::fs::read_to_string("/etc/locale.conf").ok();
        let locale = parse_locale_settings(
            std::env::var("LC_ALL").ok().as_deref(),
            std::env::var("LC_MESSAGES").ok().as_deref(),
            std::env::var("LC_CTYPE").ok().as_deref(),
            std::env::var("LANG").ok().as_deref(),
            std::env::var("LANGUAGE").ok().as_deref(),
            locale_conf.as_deref(),
        )?;
        Some(Info::new("Locale", locale))
    }
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
}

/// Locale shown to the user, in the same precedence `locale(1)` uses for the
/// message category: LC_ALL > LC_MESSAGES > LC_CTYPE > LANG.
///
/// `LANGUAGE` (a colon-separated fallback list) and `/etc/locale.conf` are used
/// as last resorts, which keeps the value usable from cron/systemd units where
/// the login environment is not exported.
pub fn parse_locale_settings(
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lc_ctype: Option<&str>,
    lang: Option<&str>,
    language: Option<&str>,
    locale_conf: Option<&str>,
) -> Option<String> {
    if let Some(value) = nonempty(lc_all) {
        return Some(value);
    }
    if let Some(value) = nonempty(lc_messages) {
        return Some(value);
    }
    if let Some(value) = nonempty(lc_ctype) {
        return Some(value);
    }
    if let Some(value) = nonempty(lang) {
        return Some(value);
    }
    if let Some(value) = nonempty(language) {
        let first = value.split(':').find(|part| !part.trim().is_empty());
        if let Some(first) = first {
            return Some(first.trim().to_string());
        }
    }
    let config = locale_conf?;
    parse_locale_conf(config)
}

/// Parses `/etc/locale.conf` (KEY=value lines, quotes optional).
pub fn parse_locale_conf(content: &str) -> Option<String> {
    let mut lang = None;
    let mut lc_all = None;
    let mut lc_messages = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches(['"', '\'']).trim();
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "LC_ALL" => lc_all = Some(value.to_string()),
            "LC_MESSAGES" => lc_messages = Some(value.to_string()),
            "LANG" => lang = Some(value.to_string()),
            _ => {}
        }
    }
    lc_all.or(lc_messages).or(lang)
}

/// Backwards-compatible two-value helper (LC_ALL > LANG).
pub fn parse_locale(lc_all: Option<&str>, lang: Option<&str>) -> Option<String> {
    parse_locale_settings(lc_all, None, None, lang, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_locale_lc_all_precedence() {
        assert_eq!(
            parse_locale(Some("C.UTF-8"), Some("en_US.UTF-8")).as_deref(),
            Some("C.UTF-8")
        );
    }

    #[test]
    fn test_parse_locale_lang_fallback() {
        assert_eq!(
            parse_locale(None, Some("en_US.UTF-8")).as_deref(),
            Some("en_US.UTF-8")
        );
        assert_eq!(
            parse_locale(None, Some("tr_TR.UTF-8")).as_deref(),
            Some("tr_TR.UTF-8")
        );
    }

    #[test]
    fn test_parse_locale_empty_handling() {
        assert_eq!(
            parse_locale(Some("   "), Some("en_US.UTF-8")).as_deref(),
            Some("en_US.UTF-8")
        );
        assert_eq!(parse_locale(Some(""), Some("")), None);
        assert_eq!(parse_locale(None, None), None);
    }

    #[test]
    fn test_parse_locale_settings_precedence() {
        assert_eq!(
            parse_locale_settings(
                None,
                Some("de_DE.UTF-8"),
                Some("en_GB.UTF-8"),
                Some("tr_TR.UTF-8"),
                None,
                None
            )
            .as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(
            parse_locale_settings(
                None,
                None,
                Some("en_GB.UTF-8"),
                Some("tr_TR.UTF-8"),
                None,
                None
            )
            .as_deref(),
            Some("en_GB.UTF-8")
        );
        assert_eq!(
            parse_locale_settings(None, None, None, None, Some("tr:en"), None).as_deref(),
            Some("tr")
        );
    }

    #[test]
    fn test_parse_locale_conf_fallback() {
        let conf = "# comment\nLANG=\"tr_TR.UTF-8\"\n";
        assert_eq!(
            parse_locale_settings(None, None, None, None, None, Some(conf)).as_deref(),
            Some("tr_TR.UTF-8")
        );
        assert_eq!(
            parse_locale_conf("LC_ALL=de_DE.UTF-8\nLANG=en_US.UTF-8\n").as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(parse_locale_conf(""), None);
    }
}
