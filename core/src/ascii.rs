use std::fs;
use std::io;
use std::path::Path;

/// Natural sort key chunk.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum KeyChunk {
    Num(u64),
    Str(String),
}

/// Computes a natural sorting key for a filename (e.g. "frame_2.txt" < "frame_10.txt").
fn natural_sort_key(s: &str) -> Vec<KeyChunk> {
    let mut chunks = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            let mut num_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    num_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            let n = num_str.parse::<u64>().unwrap_or(0);
            chunks.push(KeyChunk::Num(n));
        } else {
            let mut text = String::new();
            while let Some(&t) = chars.peek() {
                if !t.is_ascii_digit() {
                    text.push(t.to_ascii_lowercase());
                    chars.next();
                } else {
                    break;
                }
            }
            chunks.push(KeyChunk::Str(text));
        }
    }

    chunks
}

/// Reads a static ASCII art file into lines.
pub fn load_ascii_lines(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Determines whether a line acts as a frame separator in multi-frame ASCII animation files.
pub fn is_frame_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.starts_with("===")
        || trimmed.starts_with("---")
        || trimmed == "\x0c"
        || trimmed == "[frame]"
        || trimmed == "===FRAME==="
        || trimmed == "---FRAME---"
}

/// Parses multi-frame ASCII content from a single string.
pub fn parse_ascii_frames_from_str(content: &str) -> Vec<Vec<String>> {
    // Empty or whitespace-only content has no frames at all; callers treat this
    // as "nothing to render" instead of a single empty frame.
    if content.trim().is_empty() {
        return Vec::new();
    }

    // If form-feed (\x0c) exists, split by form-feed first
    if content.contains('\x0c') {
        let parts: Vec<&str> = content.split('\x0c').collect();
        let frames: Vec<Vec<String>> = parts
            .into_iter()
            .map(|part| part.lines().map(|s| s.to_string()).collect())
            .filter(|lines: &Vec<String>| !lines.is_empty())
            .collect();
        if !frames.is_empty() {
            return frames;
        }
    }

    let mut frames = Vec::new();
    let mut current_frame = Vec::new();
    let mut has_delimiters = false;

    for line in content.lines() {
        if is_frame_delimiter(line) {
            has_delimiters = true;
            if !current_frame.is_empty() {
                frames.push(std::mem::take(&mut current_frame));
            }
        } else {
            current_frame.push(line.to_string());
        }
    }

    if !current_frame.is_empty() {
        frames.push(current_frame);
    }

    // If no delimiters were encountered, return the single frame (or empty if file was empty)
    if !has_delimiters && frames.is_empty() {
        return vec![content.lines().map(|s| s.to_string()).collect()];
    }

    frames
}

/// Loads ASCII animation frames from either a single multi-frame file or a directory of frame files.
pub fn load_ascii_frames(path: &Path) -> io::Result<Vec<Vec<String>>> {
    if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                if let Ok(ft) = e.file_type() {
                    ft.is_file()
                } else {
                    false
                }
            })
            .collect();

        if entries.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No animation frames found in directory: {}", path.display()),
            ));
        }

        // Sort entries naturally (e.g. frame_1.txt before frame_10.txt)
        entries.sort_by_key(|e| natural_sort_key(&e.file_name().to_string_lossy()));

        let mut frames = Vec::with_capacity(entries.len());
        for entry in entries {
            let lines = load_ascii_lines(&entry.path())?;
            if !lines.is_empty() {
                frames.push(lines);
            }
        }

        if frames.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "All frame files in directory were empty: {}",
                    path.display()
                ),
            ));
        }

        Ok(frames)
    } else {
        let content = fs::read_to_string(path)?;
        let frames = parse_ascii_frames_from_str(&content);
        if frames.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("No valid ASCII frames parsed from: {}", path.display()),
            ));
        }
        Ok(frames)
    }
}

/// Parses a neofetch color placeholder at the start of `rest`.
///
/// Accepts the short form `$1` and the long form `${c1}`. Returns the
/// 1-based color number and the number of bytes consumed by the token.
fn parse_color_token(rest: &str) -> Option<(usize, usize)> {
    if let Some(after) = rest.strip_prefix("${c") {
        let end = after.find('}')?;
        let digits = &after[..end];
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let n = digits.parse::<usize>().ok()?;
        return Some((n, after[..end].len() + 4));
    }

    let after = rest.strip_prefix('$')?;
    let digits_len = after
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    if digits_len == 0 {
        return None;
    }
    let n = after[..digits_len].parse::<usize>().ok()?;
    Some((n, digits_len + 1))
}

/// Replaces Neofetch style color placeholders ($1..$N, ${c1}..${cN}) with ANSI escape codes.
///
/// Matching is token based: the whole digit run after `$` is consumed, so `$10`
/// means the tenth color rather than color 1 followed by a literal zero. An
/// out-of-range placeholder is preserved verbatim.
pub fn apply_neofetch_colors(content: &str, colors: &[&str]) -> String {
    let mut result = String::with_capacity(content.len());
    let mut offset = 0;

    while offset < content.len() {
        let rest = &content[offset..];
        if let Some((n, consumed)) = parse_color_token(rest) {
            if n >= 1 && n <= colors.len() {
                result.push_str(colors[n - 1]);
                offset += consumed;
                continue;
            }
        }
        let ch = rest.chars().next().unwrap_or('\0');
        result.push(ch);
        offset += ch.len_utf8();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort_order() {
        let mut list = vec![
            "frame_10.txt",
            "frame_1.txt",
            "frame_2.txt",
            "frame_20.txt",
            "frame_03.txt",
        ];
        list.sort_by_key(|a| natural_sort_key(a));
        assert_eq!(
            list,
            vec![
                "frame_1.txt",
                "frame_2.txt",
                "frame_03.txt",
                "frame_10.txt",
                "frame_20.txt"
            ]
        );
    }

    #[test]
    fn test_parse_ascii_frames_delimiter() {
        let content = "Frame 1 Line 1\nFrame 1 Line 2\n===FRAME===\nFrame 2 Line 1\n===FRAME===\nFrame 3 Line 1";
        let frames = parse_ascii_frames_from_str(content);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0], vec!["Frame 1 Line 1", "Frame 1 Line 2"]);
        assert_eq!(frames[1], vec!["Frame 2 Line 1"]);
        assert_eq!(frames[2], vec!["Frame 3 Line 1"]);
    }

    #[test]
    fn test_parse_ascii_frames_form_feed() {
        let content = "Page 1\nLine 2\x0cPage 2\nLine 2";
        let frames = parse_ascii_frames_from_str(content);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], vec!["Page 1", "Line 2"]);
        assert_eq!(frames[1], vec!["Page 2", "Line 2"]);
    }

    #[test]
    fn test_parse_single_frame() {
        let content = "Single Frame\nLine 2";
        let frames = parse_ascii_frames_from_str(content);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], vec!["Single Frame", "Line 2"]);
    }

    #[test]
    fn test_load_ascii_frames_dir() {
        let temp = std::env::temp_dir().join("rustfetch_test_anim_dir");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        fs::write(temp.join("02.txt"), "Frame 2\n").unwrap();
        fs::write(temp.join("01.txt"), "Frame 1\n").unwrap();
        fs::write(temp.join("10.txt"), "Frame 10\n").unwrap();

        let frames = load_ascii_frames(&temp).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0], vec!["Frame 1"]);
        assert_eq!(frames[1], vec!["Frame 2"]);
        assert_eq!(frames[2], vec!["Frame 10"]);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_apply_neofetch_colors() {
        let input = "${c1}Red text $2Blue text$1 Normal text";
        let colors = vec!["\x1b[31m", "\x1b[34m"];
        let result = apply_neofetch_colors(input, &colors);
        assert_eq!(
            result,
            "\x1b[31mRed text \x1b[34mBlue text\x1b[31m Normal text"
        );
    }

    #[test]
    fn test_apply_neofetch_colors_multi_digit_token() {
        let colors: Vec<String> = (0..12).map(|i| format!("\x1b[3{i}m")).collect();
        let refs: Vec<&str> = colors.iter().map(String::as_str).collect();
        assert_eq!(apply_neofetch_colors("$10", &refs), "\x1b[39m");
        assert_eq!(apply_neofetch_colors("${c11}", &refs), "\x1b[310m");
        // With only two colors "$10" is not color 1 plus "0"; it stays literal.
        let two = vec!["A", "B"];
        assert_eq!(apply_neofetch_colors("$10", &two), "$10");
        assert_eq!(apply_neofetch_colors("${c99}", &two), "${c99}");
    }

    #[test]
    fn test_apply_neofetch_colors_preserves_literal_dollar() {
        assert_eq!(apply_neofetch_colors("$ price", &["X"]), "$ price");
        assert_eq!(apply_neofetch_colors("$0", &["X"]), "$0");
        assert_eq!(apply_neofetch_colors("", &["X"]), "");
    }

    #[test]
    fn test_parse_empty_content_has_no_frames() {
        assert!(parse_ascii_frames_from_str("").is_empty());
        assert!(parse_ascii_frames_from_str("   \n\t\n").is_empty());
        assert!(parse_ascii_frames_from_str("\x0c").is_empty());
    }

    #[test]
    fn test_load_ascii_frames_empty_file_is_error() {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_ascii_test_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("empty.txt");
        fs::write(&file, "").unwrap();
        let result = load_ascii_frames(&file);
        assert!(result.is_err(), "empty single file must not yield a frame");
        let _ = fs::remove_dir_all(&dir);
    }
}
