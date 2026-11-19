use crate::text::sanitize_terminal;
use rustfetch_core::cache::Cache;
use rustfetch_core::modules::update::{
    CURRENT_VERSION, check_latest_release, codename_for_version, is_newer_version,
};
use rustfetch_core::proc::{NETWORK_TIMEOUT, run_with_timeout};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Maximum accepted installer script size (2 MiB).
pub(super) const MAX_SCRIPT_BYTES: u64 = 2 * 1024 * 1024;

/// Official one-line installer URL on `main`.
pub(super) const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh";

/// Manual one-line install command shown whenever a self-update fails.
const ONE_LINE_INSTALL_CMD: &str =
    "curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/rustfetch/main/install.sh | bash";

pub(super) fn update_status_line(
    check_updates: bool,
    latest: Option<&(String, String)>,
    current: &str,
) -> String {
    if !check_updates {
        return "skipped (--no-update-check)".to_string();
    }
    match latest {
        Some((latest_version, latest_code)) => {
            let latest_code = sanitize_terminal(latest_code);
            if is_newer_version(current, latest_version) {
                format!(
                    "Update available: v{latest_version} \"{latest_code}\" (run rustfetch --self-update)"
                )
            } else {
                let current_code = codename_for_version(current);
                format!("Up to date (v{current} \"{current_code}\")")
            }
        }
        None => "Offline / GitHub unreachable".to_string(),
    }
}

struct TempDirGuard {
    path: PathBuf,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(super) fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn create_update_temp_dir() -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(format!(
        "rustfetch-update-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder
            .create(&directory)
            .map_err(|error| format!("Failed to create temporary update directory: {error}"))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("Failed to create temporary update directory: {error}"))?;
    }
    Ok(directory)
}

fn curl_binary() -> &'static str {
    if Path::new("/usr/bin/curl").is_file() {
        "/usr/bin/curl"
    } else {
        "curl"
    }
}

pub(super) fn curl_download_args(url: &str, destination: &Path, max_time: Duration) -> Vec<String> {
    let max_time = max_time.max(Duration::from_secs(1));
    let connect_timeout = max_time.min(Duration::from_secs(10));
    vec![
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--location".to_string(),
        "--proto".to_string(),
        "=https".to_string(),
        "--tlsv1.2".to_string(),
        "--max-redirs".to_string(),
        "5".to_string(),
        "--connect-timeout".to_string(),
        connect_timeout.as_secs().to_string(),
        "--max-time".to_string(),
        max_time.as_secs().to_string(),
        "--max-filesize".to_string(),
        MAX_SCRIPT_BYTES.to_string(),
        "--output".to_string(),
        destination.to_string_lossy().into_owned(),
        "--url".to_string(),
        url.to_string(),
    ]
}

fn download(url: &str, destination: &Path, timeout: Duration) -> Result<(), String> {
    let args_owned = curl_download_args(url, destination, timeout);
    let args: Vec<&str> = args_owned.iter().map(String::as_str).collect();
    let process_timeout = timeout + Duration::from_secs(2);
    let output = run_with_timeout(curl_binary(), &args, process_timeout)
        .ok_or_else(|| format!("Could not run {}. Is curl installed?", curl_binary()))?;
    if output.timed_out {
        return Err(format!("Timed out downloading {url}"));
    }
    if !output.status.success() {
        return Err(format!(
            "curl failed for {url}: {}",
            sanitize_terminal(&output.stderr_string()).trim()
        ));
    }
    Ok(())
}

pub(super) fn validate_installer_script(content: &str) -> Result<(), String> {
    if content.contains('\0') {
        return Err("Downloaded installer script contains NUL bytes".to_string());
    }
    let trimmed = content.trim_start();
    if trimmed.is_empty() {
        return Err("Downloaded installer script is empty".to_string());
    }
    if !trimmed.starts_with("#!/") {
        return Err(
            "Downloaded file does not look like a shell script (missing shebang)".to_string(),
        );
    }
    if !content
        .to_ascii_lowercase()
        .contains("mehmetcanwt/rustfetch")
    {
        return Err("Downloaded installer script failed repository origin check".to_string());
    }
    Ok(())
}

fn update_target_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".local/bin")
        })
}

pub(super) fn build_installer_command(
    script_path: &Path,
    target_version: &str,
    install_dir: Option<&Path>,
) -> std::process::Command {
    let mut cmd = std::process::Command::new("bash");
    cmd.arg("--")
        .arg(script_path)
        .env("NONINTERACTIVE", "1")
        .env("RUSTFETCH_FORCE_CONFIG", "0")
        .env("RUSTFETCH_VERSION", target_version);
    if let Some(dir) = install_dir {
        cmd.env("RUSTFETCH_INSTALL_DIR", dir);
    }
    cmd
}

pub(crate) fn self_update() -> Result<(), String> {
    let current = CURRENT_VERSION;
    let current_codename = codename_for_version(current);
    println!(
        "==> Checking GitHub for RustFetch updates (current: v{current} \"{current_codename}\")..."
    );

    let (latest, latest_codename) = check_latest_release()
        .ok_or_else(|| "Could not reach GitHub to check for updates".to_string())?;

    if !is_newer_version(current, &latest) {
        println!("✓ RustFetch is already up to date (v{current} \"{current_codename}\").");
        return Ok(());
    }

    println!(
        "==> Updating RustFetch: v{current} \"{current_codename}\" -> v{latest} \"{latest_codename}\""
    );
    println!("==> Downloading official one-line installer from {INSTALL_SCRIPT_URL}...");

    let temp_dir = create_update_temp_dir()?;
    let _guard = TempDirGuard {
        path: temp_dir.clone(),
    };
    let script_path = temp_dir.join("install.sh");
    download(
        INSTALL_SCRIPT_URL,
        &script_path,
        NETWORK_TIMEOUT.saturating_mul(6),
    )?;

    let script_content = std::fs::read_to_string(&script_path)
        .map_err(|error| format!("Failed to inspect downloaded installer script: {error}"))?;
    validate_installer_script(&script_content)?;

    let target_dir = update_target_dir();
    println!(
        "==> Running installer (target: {}, existing config.toml will be preserved)...",
        target_dir.display()
    );

    let mut cmd = build_installer_command(&script_path, &latest, Some(&target_dir));
    let status = cmd
        .status()
        .map_err(|error| format!("Failed to execute bash installer: {error}"))?;

    if !status.success() {
        return Err(format!("Installer exited with status {status}"));
    }

    let _ = Cache::clear_all();
    println!(
        "✓ Successfully updated to RustFetch v{latest} \"{latest_codename}\" (existing config.toml preserved)."
    );
    Ok(())
}

pub fn run_self_update() {
    if let Err(error) = self_update() {
        eprintln!("✗ Update failed: {error}");
        eprintln!("Manual update: {ONE_LINE_INSTALL_CMD}");
        std::process::exit(1);
    }
}
