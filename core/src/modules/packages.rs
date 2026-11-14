use crate::proc::{PACKAGE_TOOL_TIMEOUT, run_with_timeout};
use crate::{Info, Module, cache::Cache};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct PackagesModule;

impl Module for PackagesModule {
    fn name(&self) -> &'static str {
        "packages"
    }

    /// Package counting may fork a package manager (rpm) and walk large
    /// directories, so it must run on the parallel (slow) lane.
    fn is_slow(&self) -> bool {
        true
    }

    fn detect(&self) -> Option<Info> {
        let cache = Cache::new("packages");
        if let Some(cached_val) = cache.read(Duration::from_secs(2 * 3600)) {
            return Some(Info::new("Packages", cached_val));
        }

        let mut packages: Vec<String> = Vec::new();
        let home = std::env::var("HOME")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let user_flatpak = home
            .as_deref()
            .map(|home| PathBuf::from(home).join(".local/share/flatpak/app"));
        push_count(
            &mut packages,
            count_flatpak_apps(Path::new("/var/lib/flatpak/app"), user_flatpak.as_deref()),
            "flatpak",
        );

        let mut rpm_counted = false;
        if let Some(count) = count_rpm_sqlite() {
            push_count(&mut packages, count, "rpm");
            rpm_counted = true;
        }
        if !rpm_counted && let Some(count) = count_rpm_fallback() {
            push_count(&mut packages, count, "rpm");
        }

        let dpkg_count = std::fs::read_to_string("/var/lib/dpkg/status")
            .map(|status| count_dpkg_packages(&status))
            .unwrap_or_else(|_| count_dpkg_info_files(Path::new("/var/lib/dpkg/info")));
        push_count(&mut packages, dpkg_count, "dpkg");

        push_count(
            &mut packages,
            count_pacman_packages(Path::new("/var/lib/pacman/local")),
            "pacman",
        );

        if let Ok(apk_str) = std::fs::read_to_string("/lib/apk/db/installed") {
            push_count(&mut packages, count_apk_packages(&apk_str), "apk");
        }

        if let Some(xbps_path) = find_xbps_database(Path::new("/var/db/xbps"))
            && let Ok(xbps_str) = std::fs::read_to_string(xbps_path)
        {
            push_count(&mut packages, count_xbps_packages(&xbps_str), "xbps");
        }

        push_count(
            &mut packages,
            count_emerge_packages(Path::new("/var/db/pkg")),
            "emerge",
        );

        let mut nix_bin_dirs = vec![PathBuf::from("/run/current-system/sw/bin")];
        if let Some(home) = &home {
            nix_bin_dirs.push(PathBuf::from(home).join(".nix-profile/bin"));
        }
        push_count(&mut packages, count_nix_packages_in(&nix_bin_dirs), "nix");

        push_count(
            &mut packages,
            count_snaps(Path::new("/var/lib/snapd/snaps")),
            "snap",
        );

        let mut brew_paths = vec![PathBuf::from("/home/linuxbrew/.linuxbrew/Cellar")];
        if let Some(home) = &home {
            brew_paths.push(PathBuf::from(home).join(".linuxbrew/Cellar"));
        }
        push_count(&mut packages, count_brew_cellars(&brew_paths), "brew");

        if packages.is_empty() {
            return None;
        }

        let val = packages.join(", ");
        cache.write(&val);

        Some(Info::new("Packages", val))
    }
}

pub fn count_apk_packages(content: &str) -> usize {
    content
        .lines()
        .filter(|line| line.starts_with("P:"))
        .count()
}

pub fn count_xbps_packages(content: &str) -> usize {
    content.matches("<key>pkgver</key>").count()
}

pub fn count_nonempty_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn push_count(out: &mut Vec<String>, count: usize, manager: &str) {
    if count > 0 {
        out.push(format!("{count} ({manager})"));
    }
}

fn count_direct_children(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0)
}

pub fn count_flatpak_apps(system_root: &Path, user_root: Option<&Path>) -> usize {
    count_direct_children(system_root) + user_root.map(count_direct_children).unwrap_or(0)
}

pub fn count_pacman_packages(local_dir: &Path) -> usize {
    fs::read_dir(local_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
                .count()
        })
        .unwrap_or(0)
}

/// Counts installed packages from the dpkg status database ("Package:" stanzas).
pub fn count_dpkg_packages(status: &str) -> usize {
    status
        .split("\n\n")
        .filter(|stanza| {
            let mut has_package = false;
            let mut installed = true;
            for line in stanza.lines() {
                if let Some(pkg) = line.strip_prefix("Package:") {
                    has_package = !pkg.trim().is_empty();
                } else if let Some(st) = line.strip_prefix("Status:") {
                    installed = st.trim().ends_with(" installed");
                }
            }
            has_package && installed
        })
        .count()
}

/// Historical fallback: one .list file per installed package.
pub fn count_dpkg_info_files(info_dir: &Path) -> usize {
    fs::read_dir(info_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".list"))
                .count()
        })
        .unwrap_or(0)
}

pub fn count_emerge_packages(pkg_dir: &Path) -> usize {
    fs::read_dir(pkg_dir)
        .map(|categories| {
            categories
                .flatten()
                .filter_map(|category| fs::read_dir(category.path()).ok())
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
                        .count()
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Extracts the unique /nix/store/<hash-name> identity from a resolved path.
pub fn nix_store_package(target: &Path) -> Option<String> {
    let components: Vec<&str> = target.to_str()?.split('/').collect();
    if components.len() >= 5 && components[1] == "nix" && components[2] == "store" {
        let package = components[3].trim();
        if !package.is_empty() {
            return Some(package.to_string());
        }
    }
    None
}

/// Counts unique Nix packages by resolving every profile binary symlink to its
/// store path and de-duplicating; a package reachable through both the system
/// and the user profile is counted once.
pub fn count_nix_packages_in(bin_dirs: &[PathBuf]) -> usize {
    let mut packages: HashSet<String> = HashSet::new();
    for dir in bin_dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let package = fs::read_link(entry.path())
                .ok()
                .and_then(|target| nix_store_package(&target))
                .or_else(|| {
                    fs::canonicalize(entry.path())
                        .ok()
                        .and_then(|target| nix_store_package(&target))
                });
            if let Some(package) = package {
                packages.insert(package);
            }
        }
    }
    packages.len()
}

/// Counts installed snaps by application name, ignoring retained revisions
/// (firefox_120.snap and firefox_123.snap are one snap).
pub fn count_snaps(snap_dir: &Path) -> usize {
    let mut apps: HashSet<String> = HashSet::new();
    let Ok(entries) = fs::read_dir(snap_dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(stem) = name.strip_suffix(".snap") else {
            continue;
        };
        let app = stem
            .rsplit_once('_')
            .map(|(app, _revision)| app)
            .unwrap_or(stem);
        if !app.is_empty() {
            apps.insert(app.to_string());
        }
    }
    apps.len()
}

/// Finds the versioned xbps package database instead of hard-coding one name.
pub fn find_xbps_database(db_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(db_dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("pkgdb-") && name.ends_with(".plist"))
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

/// Counts packages from rpm -qa output, excluding gpg-pubkey pseudo-packages.
pub fn count_rpm_listing(listing: &str) -> usize {
    listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("gpg-pubkey"))
        .count()
}

pub fn count_brew_cellars(paths: &[std::path::PathBuf]) -> usize {
    let mut seen = Vec::new();
    let mut total = 0;
    for path in paths {
        let Ok(canon) = fs::canonicalize(path) else {
            continue;
        };
        if seen.contains(&canon) {
            continue;
        }
        seen.push(canon.clone());
        if let Ok(entries) = fs::read_dir(&canon) {
            total += entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .count();
        }
    }
    total
}

fn count_rpm_sqlite() -> Option<usize> {
    const CANDIDATES: &[&str] = &[
        "/usr/share/rpm/rpmdb.sqlite",
        "/var/lib/rpm/rpmdb.sqlite",
        "/usr/lib/sysimage/rpm/rpmdb.sqlite",
    ];

    let db_path = CANDIDATES
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.is_file())?;

    let c_path = std::ffi::CString::new(db_path.to_str()?).ok()?;

    type Sqlite3OpenV2 = unsafe extern "C" fn(
        filename: *const libc::c_char,
        pp_db: *mut *mut libc::c_void,
        flags: libc::c_int,
        z_vfs: *const libc::c_char,
    ) -> libc::c_int;
    type Sqlite3PrepareV2 = unsafe extern "C" fn(
        db: *mut libc::c_void,
        z_sql: *const libc::c_char,
        n_byte: libc::c_int,
        pp_stmt: *mut *mut libc::c_void,
        pz_tail: *mut *const libc::c_char,
    ) -> libc::c_int;
    type Sqlite3Step = unsafe extern "C" fn(p_stmt: *mut libc::c_void) -> libc::c_int;
    type Sqlite3ColumnInt64 =
        unsafe extern "C" fn(p_stmt: *mut libc::c_void, i_col: libc::c_int) -> i64;
    type Sqlite3Finalize = unsafe extern "C" fn(p_stmt: *mut libc::c_void) -> libc::c_int;
    type Sqlite3Close = unsafe extern "C" fn(p_db: *mut libc::c_void) -> libc::c_int;

    const SQLITE_OK: libc::c_int = 0;
    const SQLITE_ROW: libc::c_int = 100;
    const SQLITE_OPEN_READONLY: libc::c_int = 0x00000001;

    let lib = unsafe {
        let lib0 = libc::dlopen(c"libsqlite3.so.0".as_ptr(), libc::RTLD_LAZY);
        if !lib0.is_null() {
            lib0
        } else {
            libc::dlopen(c"libsqlite3.so".as_ptr(), libc::RTLD_LAZY)
        }
    };
    if lib.is_null() {
        return None;
    }

    unsafe {
        let sym_open = libc::dlsym(lib, c"sqlite3_open_v2".as_ptr());
        let sym_prep = libc::dlsym(lib, c"sqlite3_prepare_v2".as_ptr());
        let sym_step = libc::dlsym(lib, c"sqlite3_step".as_ptr());
        let sym_col = libc::dlsym(lib, c"sqlite3_column_int64".as_ptr());
        let sym_fin = libc::dlsym(lib, c"sqlite3_finalize".as_ptr());
        let sym_close = libc::dlsym(lib, c"sqlite3_close".as_ptr());

        if sym_open.is_null()
            || sym_prep.is_null()
            || sym_step.is_null()
            || sym_col.is_null()
            || sym_fin.is_null()
            || sym_close.is_null()
        {
            libc::dlclose(lib);
            return None;
        }

        let fn_open: Sqlite3OpenV2 = std::mem::transmute(sym_open);
        let fn_prep: Sqlite3PrepareV2 = std::mem::transmute(sym_prep);
        let fn_step: Sqlite3Step = std::mem::transmute(sym_step);
        let fn_col: Sqlite3ColumnInt64 = std::mem::transmute(sym_col);
        let fn_fin: Sqlite3Finalize = std::mem::transmute(sym_fin);
        let fn_close: Sqlite3Close = std::mem::transmute(sym_close);

        let mut db: *mut libc::c_void = std::ptr::null_mut();

        if fn_open(
            c_path.as_ptr(),
            &mut db,
            SQLITE_OPEN_READONLY,
            std::ptr::null(),
        ) != SQLITE_OK
            || db.is_null()
        {
            if !db.is_null() {
                fn_close(db);
            }
            libc::dlclose(lib);
            return None;
        }

        let mut count: Option<usize> = None;
        for sql in [
            c"SELECT count(*) FROM Sigmd5".as_ptr(),
            c"SELECT count(*) FROM Packages".as_ptr(),
        ] {
            let mut stmt: *mut libc::c_void = std::ptr::null_mut();
            if fn_prep(db, sql, -1, &mut stmt, std::ptr::null_mut()) == SQLITE_OK && !stmt.is_null()
            {
                if fn_step(stmt) == SQLITE_ROW {
                    let n = fn_col(stmt, 0);
                    if n >= 0 {
                        count = Some(n as usize);
                    }
                }
                fn_fin(stmt);
            }
            if count.is_some() {
                break;
            }
        }

        fn_close(db);
        libc::dlclose(lib);
        count
    }
}

pub fn count_rpm_from_cache_or_exec<F>(
    rpmdb_path: Option<&std::path::Path>,
    cache: &Cache,
    exec_rpm: F,
) -> Option<usize>
where
    F: FnOnce() -> Option<usize>,
{
    let mtime_stamp = rpmdb_path.and_then(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
            .map(|d| format!("{}:{}", d.as_secs(), d.subsec_nanos()))
    });

    if let (Some(stamp), Some(cached)) = (mtime_stamp.as_deref(), cache.read_stale())
        && let Some((cached_stamp, count_str)) = cached.trim().split_once(' ')
        && cached_stamp == stamp
        && let Ok(count) = count_str.parse::<usize>()
    {
        return Some(count);
    }

    let count = exec_rpm()?;
    if let Some(stamp) = &mtime_stamp {
        cache.write(&format!("{stamp} {count}"));
    }
    Some(count)
}

pub fn count_rpm_fallback() -> Option<usize> {
    const CANDIDATES: &[&str] = &[
        "/var/lib/rpm/rpmdb.sqlite",
        "/usr/share/rpm/rpmdb.sqlite",
        "/usr/lib/sysimage/rpm/rpmdb.sqlite",
        "/var/lib/rpm/Packages",
        "/var/lib/rpm",
    ];

    let rpmdb_path = CANDIDATES
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());

    let cache = Cache::new("rpm_cache");

    count_rpm_from_cache_or_exec(rpmdb_path, &cache, || {
        if std::path::Path::new("/var/lib/rpm").exists()
            || std::path::Path::new("/usr/bin/rpm").exists()
        {
            // Bounded: a locked rpmdb must never hang the fetch.
            let output = run_with_timeout("rpm", &["-nodigest", "-qa"], PACKAGE_TOOL_TIMEOUT)?;
            let count = count_rpm_listing(&output.stdout_string());
            if output.success() && count > 0 {
                return Some(count);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_brew_cellars_deduplicates_symlinks() {
        let tmp = std::env::temp_dir().join(format!("rustfetch_brew_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let cellar = tmp.join("Cellar");
        fs::create_dir_all(cellar.join("ripgrep")).unwrap();
        fs::create_dir_all(cellar.join("fd")).unwrap();
        fs::write(cellar.join("README"), b"ignore").unwrap();

        let alias = tmp.join("CellarAlias");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&cellar, &alias).unwrap();

        let count = count_brew_cellars(&[cellar.clone(), alias, cellar]);
        let _ = fs::remove_dir_all(&tmp);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_apk_packages() {
        let sample = "C:Q1xyz\nP:musl\nV:1.2.4\nP:busybox\nV:1.36.1\n";
        assert_eq!(count_apk_packages(sample), 2);
        assert_eq!(count_apk_packages(""), 0);
    }

    #[test]
    fn test_count_xbps_packages() {
        let sample = r#"
        <dict>
            <key>pkgver</key>
            <string>bash-5.2.026_1</string>
            <key>pkgver</key>
            <string>coreutils-9.4_1</string>
        </dict>
        "#;
        assert_eq!(count_xbps_packages(sample), 2);
        assert_eq!(count_xbps_packages("<dict></dict>"), 0);
    }

    #[test]
    fn test_packages_module_name() {
        assert_eq!(PackagesModule.name(), "packages");
    }

    #[test]
    fn test_count_nonempty_lines() {
        assert_eq!(count_nonempty_lines("package-a\n\n package-b \n"), 2);
    }

    #[test]
    fn test_count_rpm_sqlite() {
        if std::path::Path::new("/usr/share/rpm/rpmdb.sqlite").exists()
            || std::path::Path::new("/var/lib/rpm/rpmdb.sqlite").exists()
        {
            let count = count_rpm_sqlite();
            assert!(count.is_some());
        }
    }

    #[test]
    fn test_rpm_fallback_cache_with_mtime() {
        let tmp =
            std::env::temp_dir().join(format!("rustfetch_rpm_cache_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let fake_rpmdb = tmp.join("rpmdb.sqlite");
        fs::write(&fake_rpmdb, b"fake-db").unwrap();

        let cache_file = tmp.join("rpm_cache");
        let cache = Cache::for_path(cache_file);

        let mut exec_count = 0;
        let count1 = count_rpm_from_cache_or_exec(Some(&fake_rpmdb), &cache, || {
            exec_count += 1;
            Some(42)
        });
        assert_eq!(count1, Some(42));
        assert_eq!(exec_count, 1);

        // Second call with same mtime should read from cache without executing closure
        let count2 = count_rpm_from_cache_or_exec(Some(&fake_rpmdb), &cache, || {
            exec_count += 1;
            Some(999)
        });
        assert_eq!(count2, Some(42));
        assert_eq!(exec_count, 1);

        // Third call after updating mtime should invalidate cache and re-execute closure
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(&fake_rpmdb, b"updated-fake-db").unwrap();
        let count3 = count_rpm_from_cache_or_exec(Some(&fake_rpmdb), &cache, || {
            exec_count += 1;
            Some(100)
        });
        assert_eq!(count3, Some(100));
        assert_eq!(exec_count, 2);

        let _ = fs::remove_dir_all(&tmp);
    }

    fn temp_pkg_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rustfetch_pkg_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_packages_module_is_slow() {
        assert!(PackagesModule.is_slow());
    }

    #[test]
    fn test_count_dpkg_packages_from_status() {
        let status = "Package: bash\nStatus: install ok installed\n\nPackage: coreutils\nStatus: install ok installed\n\nPackage: \n";
        assert_eq!(count_dpkg_packages(status), 2);
        assert_eq!(count_dpkg_packages(""), 0);
    }

    #[test]
    fn test_count_dpkg_info_files() {
        let dir = temp_pkg_dir("dpkg");
        fs::write(dir.join("bash.list"), b"").unwrap();
        fs::write(dir.join("coreutils.list"), b"").unwrap();
        fs::write(dir.join("bash.md5sums"), b"").unwrap();
        assert_eq!(count_dpkg_info_files(&dir), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_count_snaps_deduplicates_revisions() {
        let dir = temp_pkg_dir("snap");
        fs::write(dir.join("firefox_120.snap"), b"").unwrap();
        fs::write(dir.join("firefox_123.snap"), b"").unwrap();
        fs::write(dir.join("core_1.snap"), b"").unwrap();
        fs::write(dir.join("README"), b"").unwrap();
        assert_eq!(count_snaps(&dir), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_count_nix_packages_deduplicates_store_paths() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let system = temp_pkg_dir("nix-system");
            let user = temp_pkg_dir("nix-user");
            symlink("/nix/store/aaa-coreutils-9.4/bin/ls", system.join("ls")).unwrap();
            symlink("/nix/store/aaa-coreutils-9.4/bin/cat", system.join("cat")).unwrap();
            symlink("/nix/store/bbb-ripgrep-14/bin/rg", system.join("rg")).unwrap();
            // Same packages visible through the user profile must not be counted twice.
            symlink("/nix/store/aaa-coreutils-9.4/bin/ls", user.join("ls")).unwrap();
            symlink("/nix/store/ccc-fd-10/bin/fd", user.join("fd")).unwrap();

            assert_eq!(count_nix_packages_in(&[system.clone(), user.clone()]), 3);
            assert_eq!(count_nix_packages_in(&[]), 0);

            let _ = fs::remove_dir_all(&system);
            let _ = fs::remove_dir_all(&user);
        }
    }

    #[test]
    fn test_nix_store_package_parsing() {
        assert_eq!(
            nix_store_package(std::path::Path::new("/nix/store/abc-coreutils-9.4/bin/ls"))
                .as_deref(),
            Some("abc-coreutils-9.4")
        );
        assert_eq!(nix_store_package(std::path::Path::new("/usr/bin/ls")), None);
        assert_eq!(nix_store_package(std::path::Path::new("/nix/store/")), None);
    }

    #[test]
    fn test_count_flatpak_apps() {
        let system = temp_pkg_dir("flatpak-system");
        let user = temp_pkg_dir("flatpak-user");
        fs::create_dir_all(system.join("org.gnome.Calculator")).unwrap();
        fs::create_dir_all(system.join("org.mozilla.firefox")).unwrap();
        fs::create_dir_all(user.join("org.gnome.Calculator")).unwrap();
        assert_eq!(count_flatpak_apps(&system, Some(&user)), 3);
        assert_eq!(count_flatpak_apps(&system, None), 2);
        let _ = fs::remove_dir_all(&system);
        let _ = fs::remove_dir_all(&user);
    }

    #[test]
    fn test_count_pacman_packages_ignores_db_version_file() {
        let dir = temp_pkg_dir("pacman");
        fs::create_dir_all(dir.join("bash")).unwrap();
        fs::create_dir_all(dir.join("coreutils")).unwrap();
        fs::write(dir.join("ALPM_DB_VERSION"), b"").unwrap();
        assert_eq!(count_pacman_packages(&dir), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_count_emerge_packages_counts_directories_only() {
        let dir = temp_pkg_dir("emerge");
        fs::create_dir_all(dir.join("sys-apps/baselayout-2.14")).unwrap();
        fs::create_dir_all(dir.join("app-editors/vim-9.1")).unwrap();
        fs::write(dir.join("sys-apps/stray-file"), b"").unwrap();
        assert_eq!(count_emerge_packages(&dir), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_xbps_database_uses_glob_not_hardcoded_version() {
        let dir = temp_pkg_dir("xbps");
        fs::write(dir.join("pkgdb-0.38.plist"), b"<dict></dict>").unwrap();
        fs::write(dir.join("pkgdb-0.39.plist"), b"<dict></dict>").unwrap();
        fs::write(dir.join("pkgdb-0.38.plist.bak"), b"").unwrap();
        let found = find_xbps_database(&dir).expect("database must be found");
        assert_eq!(found.file_name().unwrap(), "pkgdb-0.39.plist");
        let _ = fs::remove_dir_all(&dir);
        assert!(find_xbps_database(std::path::Path::new("/definitely/missing")).is_none());
    }

    #[test]
    fn test_count_rpm_listing_excludes_gpg_pubkeys() {
        let listing = "bash-5.2.026-1.x86_64\ncoreutils-9.4-1.x86_64\ngpg-pubkey-1234abcd-99999999\n\ngpg-pubkey-deadbeef-11111111\n";
        assert_eq!(count_rpm_listing(listing), 2);
    }
}
