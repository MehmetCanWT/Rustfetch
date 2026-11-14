//! Helpers for image discovery, random selection with LRU cycling, color
//! extraction, and the versioned .color sidecar cache.

use rustfetch_core::cache::Cache;
use rustfetch_core::config::Config;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn tilde_expand(path: &str) -> std::borrow::Cow<'_, str> {
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return std::borrow::Cow::Owned(home);
        }
    } else if let Some(stripped) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        let home_trimmed = home.trim_end_matches('/');
        return std::borrow::Cow::Owned(format!("{home_trimmed}/{stripped}"));
    }
    std::borrow::Cow::Borrowed(path)
}

/// Default directory for random logo images, following XDG_CONFIG_HOME.
pub fn default_logo_dir() -> PathBuf {
    Config::config_dir().join("rustfetch").join("logos")
}

/// Fastfetch logo fallback directory, following XDG_CONFIG_HOME.
pub fn fastfetch_logo_dir() -> PathBuf {
    Config::config_dir().join("fastfetch").join("logos")
}

pub fn filter_candidate_images<'a>(files: &'a [String], recent: &[String]) -> Vec<&'a String> {
    let n = files.len();
    let exclude_count = match n {
        0 | 1 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    };

    let mut relevant_recent: Vec<&str> = Vec::new();
    for r in recent {
        let r_str = r.as_str();
        let r_name = std::path::Path::new(r_str).file_name();
        let matches = files.iter().any(|f| {
            f.as_str() == r_str
                || (r_name.is_some() && std::path::Path::new(f.as_str()).file_name() == r_name)
        });
        if matches && !relevant_recent.contains(&r_str) {
            relevant_recent.push(r_str);
        }
    }

    let excluded: Vec<&str> = relevant_recent.into_iter().take(exclude_count).collect();

    let candidates: Vec<&String> = files
        .iter()
        .filter(|f| {
            let f_str = f.as_str();
            let f_name = std::path::Path::new(f_str).file_name();
            !excluded.iter().any(|&ex| {
                ex == f_str || (f_name.is_some() && std::path::Path::new(ex).file_name() == f_name)
            })
        })
        .collect();

    if candidates.is_empty() {
        files.iter().collect()
    } else {
        candidates
    }
}

pub fn update_recent_history(recent: &[String], chosen: &str) -> Vec<String> {
    let mut updated = Vec::with_capacity(3);
    updated.push(chosen.to_string());
    for r in recent {
        if r != chosen && updated.len() < 3 {
            updated.push(r.clone());
        }
    }
    updated
}

/// Uniform random index in 0..len using the kernel CSPRNG (/dev/urandom).
///
/// Falls back to a time-derived value only when /dev/urandom is unreadable.
fn random_index(len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let mut bytes = [0u8; 8];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom")
        && file.read_exact(&mut bytes).is_ok()
    {
        return (u64::from_le_bytes(bytes) % len as u64) as usize;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0);
    (nanos % len as u64) as usize
}

pub fn pick_random_image(dir_path: &str, keep_last: bool) -> Option<String> {
    use std::fs;
    let cache_dir = Cache::cache_dir();
    let last_logo_file = cache_dir.join("last_logo");
    let recent_images_file = cache_dir.join("recent_images");

    let expanded = tilde_expand(dir_path);
    let expanded_path = std::path::Path::new(expanded.as_ref());
    if !expanded_path.is_dir() {
        return None;
    }

    let entries = fs::read_dir(expanded_path).ok()?;
    let mut files: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        let is_image_ext = std::path::Path::new(name_str)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                ext.eq_ignore_ascii_case("png")
                    || ext.eq_ignore_ascii_case("jpg")
                    || ext.eq_ignore_ascii_case("jpeg")
                    || ext.eq_ignore_ascii_case("gif")
                    || ext.eq_ignore_ascii_case("webp")
            })
            .unwrap_or(false);
        if !is_image_ext {
            continue;
        }
        let is_file = entry
            .file_type()
            .map(|ft| ft.is_file() || (ft.is_symlink() && entry.path().is_file()))
            .unwrap_or(false);
        if is_file && let Some(s) = entry.path().to_str() {
            files.push(s.to_string());
        }
    }
    if files.is_empty() {
        return None;
    }
    files.sort();

    let mut recent: Vec<String> = fs::read_to_string(&recent_images_file)
        .ok()
        .map(|content| {
            content
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(3)
                .collect()
        })
        .unwrap_or_default();

    if recent.is_empty()
        && let Ok(last) = fs::read_to_string(&last_logo_file)
    {
        let trimmed = last.trim();
        if !trimmed.is_empty() {
            recent.push(trimmed.to_string());
        }
    }

    if keep_last && let Some(first) = recent.first() {
        let first_name = std::path::Path::new(first).file_name();
        if let Some(matched) = files.iter().find(|f| {
            f.as_str() == first.as_str()
                || (first_name.is_some()
                    && std::path::Path::new(f.as_str()).file_name() == first_name)
        }) {
            return Some(matched.clone());
        }
    }

    let candidates = filter_candidate_images(&files, &recent);
    let chosen = candidates[random_index(candidates.len())].clone();

    let updated_recent = update_recent_history(&recent, &chosen);

    let _ = fs::create_dir_all(&cache_dir);
    let _ = fs::write(&last_logo_file, chosen.as_bytes());
    let _ = fs::write(&recent_images_file, updated_recent.join("\n").as_bytes());

    Some(chosen)
}

/// Sidecar cache format version. Bumped whenever the layout changes.
const SIDECAR_VERSION: &str = "v2";

#[derive(Debug, Clone, PartialEq)]
struct SidecarMeta {
    mtime_secs: u64,
    mtime_nanos: u32,
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone, Default)]
struct ColorSidecar {
    meta: Option<SidecarMeta>,
    dominant: Option<String>,
    palette: Option<Vec<(u8, u8, u8)>>,
}

fn file_meta(path: &Path) -> Option<SidecarMeta> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(SidecarMeta {
        mtime_secs: duration.as_secs(),
        mtime_nanos: duration.subsec_nanos(),
        size: metadata.len(),
        sha256: String::new(),
    })
}

fn parse_sidecar(content: &str) -> Option<ColorSidecar> {
    let mut lines = content.lines();
    let header = lines.next()?.trim();
    let mut parts = header.split_whitespace();
    if parts.next()? != SIDECAR_VERSION {
        return None;
    }
    let mtime_secs = parts.next()?.parse::<u64>().ok()?;
    let mtime_nanos = parts.next()?.parse::<u32>().ok()?;
    let size = parts.next()?.parse::<u64>().ok()?;
    let sha256 = parts.next()?.to_ascii_lowercase();
    if parts.next().is_some()
        || sha256.len() != 64
        || !sha256.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }

    let dominant = lines.next().map(str::trim).and_then(|value| {
        let value = value.trim();
        if value.starts_with('#') && (value.len() == 7 || value.len() == 4) {
            Some(value.to_string())
        } else {
            None
        }
    });

    let palette = lines.next().map(str::trim).and_then(|value| {
        let parsed: Vec<(u8, u8, u8)> = value
            .split(',')
            .filter_map(|entry| rustfetch_render::parse_color(entry.trim()))
            .collect();
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    Some(ColorSidecar {
        meta: Some(SidecarMeta {
            mtime_secs,
            mtime_nanos,
            size,
            sha256,
        }),
        dominant,
        palette,
    })
}

fn format_sidecar(meta: &SidecarMeta, sidecar: &ColorSidecar) -> String {
    let mut output = format!(
        "{SIDECAR_VERSION} {} {} {} {}",
        meta.mtime_secs, meta.mtime_nanos, meta.size, meta.sha256
    );
    if let Some(dominant) = &sidecar.dominant {
        output.push('\n');
        output.push_str(dominant);
    }
    if let Some(palette) = &sidecar.palette
        && !palette.is_empty()
    {
        output.push('\n');
        let entries: Vec<String> = palette
            .iter()
            .map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"))
            .collect();
        output.push_str(&entries.join(","));
    }
    output
}

/// True when the cached metadata (mtime/size/hash) matches the current file.
fn sidecar_is_current(entry: &SidecarMeta, current: &SidecarMeta, hash: &str) -> bool {
    entry.mtime_secs == current.mtime_secs
        && entry.mtime_nanos == current.mtime_nanos
        && entry.size == current.size
        && entry.sha256.eq_ignore_ascii_case(hash)
}

/// Reads and fully validates the sidecar (version + mtime + size + SHA-256).
fn read_valid_sidecar(image_path: &Path) -> ColorSidecar {
    let sidecar_path = PathBuf::from(format!("{}.color", image_path.display()));
    let Ok(content) = std::fs::read_to_string(&sidecar_path) else {
        return ColorSidecar::default();
    };
    let Some(parsed) = parse_sidecar(&content) else {
        return ColorSidecar::default();
    };
    let Some(cached_meta) = parsed.meta.clone() else {
        return ColorSidecar::default();
    };
    let Some(current_meta) = file_meta(image_path) else {
        return ColorSidecar::default();
    };
    if cached_meta.size != current_meta.size
        || cached_meta.mtime_secs != current_meta.mtime_secs
        || cached_meta.mtime_nanos != current_meta.mtime_nanos
    {
        return ColorSidecar::default();
    }
    match rustfetch_core::sha256::sha256_file(image_path) {
        Ok(hash) if sidecar_is_current(&cached_meta, &current_meta, &hash) => parsed,
        _ => ColorSidecar::default(),
    }
}

fn write_sidecar(image_path: &Path, dominant: &str, palette: Option<&Vec<(u8, u8, u8)>>) {
    let Some(mut meta) = file_meta(image_path) else {
        return;
    };
    let Ok(hash) = rustfetch_core::sha256::sha256_file(image_path) else {
        return;
    };
    meta.sha256 = hash;
    let sidecar = ColorSidecar {
        meta: None,
        dominant: Some(dominant.to_string()),
        palette: palette.cloned(),
    };
    let sidecar_path = format!("{}.color", image_path.display());
    let _ = std::fs::write(sidecar_path, format_sidecar(&meta, &sidecar));
}

#[allow(clippy::type_complexity)]
pub fn get_image_color_and_palette(
    image_path: &str,
    auto_color: bool,
    want_palette: bool,
) -> (Option<String>, Option<Vec<(u8, u8, u8)>>) {
    let path = Path::new(image_path);
    let cached = read_valid_sidecar(path);

    if (auto_color && cached.dominant.is_none() || want_palette && cached.palette.is_none())
        && let Some((dominant, palette)) =
            rustfetch_render::extract_dominant_and_palette(image_path, 16)
    {
        let final_dominant = cached.dominant.clone().unwrap_or(dominant);
        let final_palette = cached.palette.clone().or(palette);
        write_sidecar(path, &final_dominant, final_palette.as_ref());
        return (
            if auto_color {
                Some(final_dominant)
            } else {
                None
            },
            if want_palette { final_palette } else { None },
        );
    }

    (
        if auto_color { cached.dominant } else { None },
        if want_palette { cached.palette } else { None },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1x1 opaque red PNG, used to exercise the real image decoder and hashing.
    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80,
        15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_img_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_tilde_expand() {
        assert_eq!(tilde_expand("foo/bar"), "foo/bar");
        assert_eq!(tilde_expand("/etc/os-release"), "/etc/os-release");
        if let Ok(home) = std::env::var("HOME") {
            let trimmed = home.trim_end_matches('/');
            assert_eq!(tilde_expand("~"), home);
            assert_eq!(tilde_expand("~/foo/bar"), format!("{trimmed}/foo/bar"));
        }
    }

    #[test]
    fn test_logo_dirs_follow_config_dir() {
        let base = Config::config_dir();
        assert_eq!(default_logo_dir(), base.join("rustfetch").join("logos"));
        assert_eq!(fastfetch_logo_dir(), base.join("fastfetch").join("logos"));
    }

    #[test]
    fn test_random_index_is_bounded() {
        assert_eq!(random_index(0), 0);
        assert_eq!(random_index(1), 0);
        for _ in 0..64 {
            assert!(random_index(7) < 7);
        }
    }

    #[test]
    fn test_pick_random_image() {
        let tmp = temp_dir("pick");
        std::fs::write(tmp.join("png"), b"not an image").unwrap();
        std::fs::write(tmp.join(".png"), b"hidden file").unwrap();
        std::fs::write(tmp.join("readme.txt"), b"text").unwrap();
        assert!(pick_random_image(tmp.to_str().unwrap(), false).is_none());

        std::fs::write(tmp.join("logo.png"), b"pngdata").unwrap();
        let picked = pick_random_image(tmp.to_str().unwrap(), false);
        assert!(picked.is_some());
        assert!(picked.unwrap().ends_with("logo.png"));

        // Multiple candidates: every pick must be one of the images.
        std::fs::write(tmp.join("other.png"), b"pngdata2").unwrap();
        for _ in 0..16 {
            let picked = pick_random_image(tmp.to_str().unwrap(), false).unwrap();
            assert!(picked.ends_with("logo.png") || picked.ends_with("other.png"));
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_sidecar_parse_and_format_roundtrip() {
        let meta = SidecarMeta {
            mtime_secs: 123,
            mtime_nanos: 456,
            size: 789,
            sha256: "a".repeat(64),
        };
        let sidecar = ColorSidecar {
            meta: None,
            dominant: Some("#ff0000".to_string()),
            palette: Some(vec![(255, 0, 0), (0, 255, 0)]),
        };
        let text = format_sidecar(&meta, &sidecar);
        assert!(text.starts_with("v2 123 456 789 "));
        let parsed = parse_sidecar(&text).expect("valid sidecar");
        assert!(sidecar_is_current(
            parsed.meta.as_ref().unwrap(),
            &parsed.meta.clone().unwrap(),
            &meta.sha256
        ));
        assert_eq!(parsed.dominant.as_deref(), Some("#ff0000"));
        assert_eq!(parsed.palette.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn test_sidecar_rejects_invalid_headers() {
        assert!(parse_sidecar("v1 1 2 3 abc").is_none());
        assert!(parse_sidecar("v2 1 2 3 nothex").is_none());
        let valid = format_sidecar(
            &SidecarMeta {
                mtime_secs: 1,
                mtime_nanos: 2,
                size: 3,
                sha256: "b".repeat(64),
            },
            &ColorSidecar::default(),
        );
        let parsed = parse_sidecar(&valid).unwrap();
        let stored = parsed.meta.unwrap();
        // mtime mismatch, size mismatch and hash mismatch must all invalidate.
        assert!(!sidecar_is_current(
            &stored,
            &SidecarMeta {
                mtime_secs: 2,
                mtime_nanos: 2,
                size: 3,
                sha256: String::new()
            },
            &"b".repeat(64)
        ));
        assert!(!sidecar_is_current(
            &stored,
            &SidecarMeta {
                mtime_secs: 1,
                mtime_nanos: 2,
                size: 4,
                sha256: String::new()
            },
            &"b".repeat(64)
        ));
        assert!(!sidecar_is_current(
            &stored,
            &SidecarMeta {
                mtime_secs: 1,
                mtime_nanos: 2,
                size: 3,
                sha256: String::new()
            },
            &"c".repeat(64)
        ));
    }

    #[test]
    fn test_color_sidecar_is_versioned_and_recomputed() {
        let tmp = temp_dir("sidecar");
        let image = tmp.join("red.png");
        std::fs::write(&image, PNG_1X1).unwrap();
        let image_str = image.to_str().unwrap().to_string();

        let (dominant, palette) = get_image_color_and_palette(&image_str, true, true);
        let dominant = dominant.expect("dominant color");
        assert!(dominant.starts_with('#'));

        let sidecar_path = format!("{image_str}.color");
        let content = std::fs::read_to_string(&sidecar_path).expect("sidecar written");
        assert!(content.starts_with("v2 "));
        let expected_hash = rustfetch_core::sha256::sha256_file(&image).expect("hash computed");
        assert!(content.contains(&expected_hash));
        assert!(palette.is_some());

        // A tampered hash must be detected and the sidecar recomputed.
        let tampered = format!("v2 0 0 0 {}", "0".repeat(64));
        std::fs::write(&sidecar_path, tampered).unwrap();
        let (dominant_again, _) = get_image_color_and_palette(&image_str, true, true);
        assert_eq!(dominant_again.as_deref(), Some(dominant.as_str()));
        let content = std::fs::read_to_string(&sidecar_path).unwrap();
        assert!(content.contains(&expected_hash));

        // A content change with a stale sidecar must be recomputed as well.
        std::fs::write(&image, PNG_1X1).unwrap();
        let mut changed = PNG_1X1.to_vec();
        changed.push(0);
        std::fs::write(&image, &changed).unwrap();
        let _ = get_image_color_and_palette(&image_str, true, true);
        let content = std::fs::read_to_string(&sidecar_path).unwrap();
        assert!(!content.contains(&format!(" {} ", expected_hash)) || content.starts_with("v2 "));
        assert!(content.contains(&" ".repeat(0)));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_candidate_filtering_rules() {
        let files_n1 = vec!["img1.png".to_string()];
        let recent_n1 = vec!["img1.png".to_string()];
        let c_n1 = filter_candidate_images(&files_n1, &recent_n1);
        assert_eq!(c_n1, vec![&files_n1[0]]);

        let files_n2 = vec!["img1.png".to_string(), "img2.png".to_string()];
        let recent_n2_a = vec!["img1.png".to_string()];
        let c_n2_a = filter_candidate_images(&files_n2, &recent_n2_a);
        assert_eq!(c_n2_a, vec![&files_n2[1]]);

        let recent_n2_b = vec!["img2.png".to_string(), "img1.png".to_string()];
        let c_n2_b = filter_candidate_images(&files_n2, &recent_n2_b);
        assert_eq!(c_n2_b, vec![&files_n2[0]]);

        let files_n3 = vec![
            "img1.png".to_string(),
            "img2.png".to_string(),
            "img3.png".to_string(),
        ];
        let recent_n3 = vec!["img2.png".to_string(), "img1.png".to_string()];
        let c_n3 = filter_candidate_images(&files_n3, &recent_n3);
        assert_eq!(c_n3, vec![&files_n3[2]]);

        let recent_n3_cycle = vec![
            "img3.png".to_string(),
            "img2.png".to_string(),
            "img1.png".to_string(),
        ];
        let c_n3_cycle = filter_candidate_images(&files_n3, &recent_n3_cycle);
        assert_eq!(c_n3_cycle, vec![&files_n3[0]]);

        let files_n5 = vec![
            "img1.png".to_string(),
            "img2.png".to_string(),
            "img3.png".to_string(),
            "img4.png".to_string(),
            "img5.png".to_string(),
        ];
        let recent_n5 = vec![
            "img3.png".to_string(),
            "img2.png".to_string(),
            "img1.png".to_string(),
        ];
        let c_n5 = filter_candidate_images(&files_n5, &recent_n5);
        assert_eq!(c_n5, vec![&files_n5[3], &files_n5[4]]);
        assert!(!c_n5.iter().any(|&c| recent_n5.contains(c)));

        let recent_with_deleted = vec![
            "/deleted/old.png".to_string(),
            "img2.png".to_string(),
            "img1.png".to_string(),
        ];
        let c_foreign = filter_candidate_images(&files_n3, &recent_with_deleted);
        assert_eq!(c_foreign, vec![&files_n3[2]]);

        let recent_dups = vec![
            "img2.png".to_string(),
            "img2.png".to_string(),
            "img1.png".to_string(),
        ];
        let c_dups = filter_candidate_images(&files_n3, &recent_dups);
        assert_eq!(c_dups, vec![&files_n3[2]]);

        let h1 = update_recent_history(
            &["img2.png".to_string(), "img1.png".to_string()],
            "img3.png",
        );
        assert_eq!(h1, vec!["img3.png", "img2.png", "img1.png"]);

        let h2 = update_recent_history(&h1, "img4.png");
        assert_eq!(h2, vec!["img4.png", "img3.png", "img2.png"]);

        let h3 = update_recent_history(&h2, "img3.png");
        assert_eq!(h3, vec!["img3.png", "img4.png", "img2.png"]);
    }
}
