//! On-disk cache for expensive module values.
//!
//! The cache directory is user-private (0700). When neither `XDG_CACHE_HOME`
//! nor `HOME` is available (cron jobs, systemd units, containers) the runtime
//! directory or a user-specific temporary directory is used instead of a
//! shared, predictable `/tmp/rustfetch-cache`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub struct Cache {
    dir: PathBuf,
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Returns a stable, user-specific suffix for the temporary-directory fallback.
fn user_suffix() -> String {
    #[cfg(unix)]
    {
        // SAFETY: geteuid() is a pure syscall wrapper that cannot fail.
        let uid = unsafe { libc::geteuid() };
        format!("{uid}")
    }
    #[cfg(not(unix))]
    {
        non_empty_env("USERNAME")
            .or_else(|| non_empty_env("USER"))
            .unwrap_or_else(|| "user".to_string())
    }
}

impl Cache {
    fn cache_dir_from_env(get_env: impl Fn(&str) -> Option<String>) -> PathBuf {
        if let Some(xdg) = get_env("XDG_CACHE_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("rustfetch");
            }
        }
        if let Some(home) = get_env("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(".cache").join("rustfetch");
            }
        }
        if let Some(runtime) = get_env("XDG_RUNTIME_DIR") {
            let trimmed = runtime.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("rustfetch");
            }
        }
        let suffix = get_env("RUSTFETCH_CACHE_UID").unwrap_or_else(user_suffix);
        std::env::temp_dir().join(format!("rustfetch-cache-{suffix}"))
    }

    pub fn cache_dir() -> PathBuf {
        Self::cache_dir_from_env(non_empty_env)
    }

    /// Creates `dir` (and parents) with 0700 permissions on Unix.
    fn prepare_dir(dir: &Path) {
        if !dir.is_dir() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = fs::DirBuilder::new();
                builder.recursive(true).mode(0o700);
                let _ = builder.create(dir);
            }
            #[cfg(not(unix))]
            {
                let _ = fs::create_dir_all(dir);
            }
        }
        // Make sure an existing directory (created by an older version with the
        // default umask) is not group/world readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            if let Ok(meta) = fs::symlink_metadata(dir)
                && meta.file_type().is_dir()
                && meta.uid() == unsafe { libc::geteuid() }
            {
                let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
            }
        }
    }

    /// Removes the entire cache directory tree, returning the number of removed
    /// entries and any error messages. Best effort: one failure does not stop
    /// the remaining removals.
    pub fn clear_dir(dir: &Path) -> (usize, Vec<String>) {
        let mut removed = 0usize;
        let mut errors = Vec::new();
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    errors.push(format!(
                        "failed to read cache directory '{}': {err}",
                        dir.display()
                    ));
                }
                return (removed, errors);
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    errors.push(format!("failed to inspect '{}': {err}", path.display()));
                    continue;
                }
            };
            let result = if file_type.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            match result {
                Ok(()) => removed += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => errors.push(format!("failed to remove '{}': {err}", path.display())),
            }
        }
        (removed, errors)
    }

    pub fn clear_all() -> usize {
        let dir = Self::cache_dir();
        let (removed, errors) = Self::clear_dir(&dir);
        for error in errors {
            eprintln!("warning: {error}");
        }
        removed
    }

    pub fn new(module_name: &str) -> Self {
        let dir = Self::cache_dir();
        Self::prepare_dir(&dir);
        Self {
            dir: dir.join(module_name),
        }
    }

    pub fn for_path(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            Self::prepare_dir(parent);
        }
        Self { dir: path }
    }

    pub fn read(&self, ttl: Duration) -> Option<String> {
        let modified = fs::metadata(&self.dir).ok()?.modified().ok()?;
        if SystemTime::now().duration_since(modified).ok()? < ttl {
            fs::read_to_string(&self.dir).ok()
        } else {
            None
        }
    }

    pub fn read_stale(&self) -> Option<String> {
        fs::read_to_string(&self.dir).ok()
    }

    pub fn write(&self, value: &str) {
        // Write-then-rename prevents another RustFetch process from observing a
        // partially written cache entry.
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_path = self
            .dir
            .with_extension(format!("{}-{}.tmp", std::process::id(), nanos));
        if let Some(parent) = temp_path.parent() {
            Self::prepare_dir(parent);
        }
        #[cfg(unix)]
        let wrote = {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp_path)
                .and_then(|mut f| f.write_all(value.as_bytes()))
                .is_ok()
        };
        #[cfg(not(unix))]
        let wrote = fs::write(&temp_path, value).is_ok();

        if wrote {
            if fs::rename(&temp_path, &self.dir).is_err() {
                let _ = fs::remove_file(&temp_path);
            }
        } else {
            let _ = fs::remove_file(&temp_path);
        }
    }

    /// Removes this cache entry, used to roll back a throttle marker.
    pub fn remove(&self) {
        match fs::remove_file(&self.dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => eprintln!(
                "warning: failed to remove cache entry '{}': {err}",
                self.dir.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_root(label: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rustfetch-cache-test-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn cache_write_replaces_the_previous_value() {
        let root = temp_root("replace");
        let cache = Cache {
            dir: root.join("packages"),
        };

        cache.write("first");
        cache.write("second");

        assert_eq!(fs::read_to_string(&cache.dir).unwrap(), "second");
        let tmp_files: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter(|e| e.path().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            tmp_files.is_empty(),
            "Leftover temp files found: {:?}",
            tmp_files
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cache_dir_prefers_xdg_then_home() {
        let xdg = Cache::cache_dir_from_env(|var| match var {
            "XDG_CACHE_HOME" => Some("/xdg/cache".into()),
            "HOME" => Some("/home/user".into()),
            _ => None,
        });
        assert_eq!(xdg, PathBuf::from("/xdg/cache/rustfetch"));

        let home = Cache::cache_dir_from_env(|var| match var {
            "XDG_CACHE_HOME" => Some("   ".into()),
            "HOME" => Some("/home/user".into()),
            _ => None,
        });
        assert_eq!(home, PathBuf::from("/home/user/.cache/rustfetch"));
    }

    #[test]
    fn cache_dir_without_home_uses_runtime_or_user_specific_temp() {
        let runtime = Cache::cache_dir_from_env(|var| match var {
            "XDG_RUNTIME_DIR" => Some("/run/user/1000".into()),
            _ => None,
        });
        assert_eq!(runtime, PathBuf::from("/run/user/1000/rustfetch"));

        let temp = Cache::cache_dir_from_env(|var| match var {
            "RUSTFETCH_CACHE_UID" => Some("4242".into()),
            _ => None,
        });
        assert_eq!(temp, std::env::temp_dir().join("rustfetch-cache-4242"));
        // The fallback directory name must include the user suffix.
        assert!(temp.file_name().unwrap().to_string_lossy().contains("4242"));
    }

    #[test]
    fn clear_dir_removes_files_and_subdirectories() {
        let root = temp_root("clear");
        fs::write(root.join("update"), b"value").unwrap();
        fs::create_dir_all(root.join("nested/deeper")).unwrap();
        fs::write(root.join("nested/deeper/file"), b"value").unwrap();

        let (removed, errors) = Cache::clear_dir(&root);
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert_eq!(removed, 2);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clear_dir_missing_directory_is_not_an_error() {
        let root =
            std::env::temp_dir().join(format!("rustfetch-cache-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (removed, errors) = Cache::clear_dir(&root);
        assert_eq!(removed, 0);
        assert!(errors.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn prepare_dir_creates_private_directory() {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_root("perm");
        let dir = root.join("rustfetch");
        Cache::prepare_dir(&dir);
        let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "cache directory must be user-private");
        assert!(dir.is_dir());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn prepare_dir_tightens_existing_loose_directory() {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_root("tighten");
        let dir = root.join("rustfetch");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        let _cache = Cache::for_path(dir.join("entry"));
        let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o700,
            "existing cache directory must be tightened to 0o700"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remove_clears_cache_entry() {
        let root = temp_root("remove");
        let cache = Cache {
            dir: root.join("throttle"),
        };
        cache.write("1");
        assert!(cache.dir.is_file());
        cache.remove();
        assert!(!cache.dir.exists());
        let _ = fs::remove_dir_all(root);
    }
}
