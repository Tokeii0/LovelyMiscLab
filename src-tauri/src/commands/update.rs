//! In-place self-update for the portable single-file build.
//!
//! The app ships as one standalone executable per OS (no installer), so the
//! official Tauri updater (which expects signed installer bundles) doesn't
//! apply. Instead we query the GitHub Releases API for the latest *published*
//! release, compare versions, and — on the user's confirm — download the new
//! binary and swap it in place: rename the current executable aside and move the
//! download into its path, then relaunch. Both Windows and Unix allow renaming a
//! running executable, so the swap works on Windows and Linux.
//!
//! macOS is the exception: it ships a `.app` bundle inside a `.dmg`, which can't
//! be replaced as a single file, so there [`PLATFORM.self_update`] is `false` and
//! the UI falls back to opening the Release page for a manual download.

use serde::Serialize;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

const REPO: &str = "Tokeii0/LovelyMiscLab";
const USER_AGENT: &str = "LovelyMiscLab-Updater";

/// Per-platform release-asset selection + self-update capability, resolved at
/// compile time. The published Release carries one artifact per OS whose file
/// name contains `key` (e.g. `…-windows-x64.exe`, `…-linux-x64`,
/// `…-macos-universal.dmg`).
struct Platform {
    /// Substring identifying this platform's asset (matched case-insensitively).
    key: &'static str,
    /// Leading magic bytes a valid downloaded executable must start with.
    magic: &'static [u8],
    /// Whether the running executable can be replaced in place (see module doc).
    self_update: bool,
}

#[cfg(target_os = "windows")]
const PLATFORM: Platform = Platform { key: "windows", magic: b"MZ", self_update: true };
#[cfg(target_os = "linux")]
const PLATFORM: Platform = Platform { key: "linux", magic: b"\x7fELF", self_update: true };
// macOS ships a universal (FAT) `.dmg`, not a swappable single binary, so
// `self_update` is false and `download()`/the magic check never run here — the
// magic is left empty rather than pinning one arch (Intel `0xCF…`, ARM, or the
// FAT `0xCAFEBABE` header a universal build actually produces).
#[cfg(target_os = "macos")]
const PLATFORM: Platform = Platform { key: "macos", magic: b"", self_update: false };

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub release_url: String,
    pub download_url: String,
    pub asset_name: String,
}

fn err(msg: impl Into<String>) -> AppError {
    AppError::new("update", msg)
}

/// True when `a` is a strictly newer dotted version than `b` (e.g. 0.2.3 > 0.2.2).
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| {
        s.trim()
            .trim_start_matches('v')
            .split(['.', '-', '+'])
            .filter_map(|x| x.parse::<u64>().ok())
            .collect::<Vec<_>>()
    };
    let (pa, pb) = (parse(a), parse(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

fn fetch_latest(current: &str) -> AppResult<UpdateInfo> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| err(format!("检查更新失败：{e}")))?;
    let json: serde_json::Value = resp.into_json().map_err(|e| err(e.to_string()))?;

    let latest = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if latest.is_empty() {
        return Err(err("未获取到最新版本号（可能还没有已发布的 Release）。"));
    }
    let notes = json["body"].as_str().unwrap_or("").to_string();
    let release_url = json["html_url"].as_str().unwrap_or("").to_string();

    // Pick the asset that matches the current platform (by file-name substring).
    let asset = json["assets"].as_array().and_then(|arr| {
        arr.iter().find(|a| {
            a["name"]
                .as_str()
                .map(|n| n.to_lowercase().contains(PLATFORM.key))
                .unwrap_or(false)
        })
    });
    let asset_name = asset
        .and_then(|a| a["name"].as_str())
        .unwrap_or("")
        .to_string();
    // Only offer an in-app download when we can actually self-install; otherwise
    // the UI keeps the install button disabled and points at the Release page.
    let download_url = if PLATFORM.self_update {
        asset
            .and_then(|a| a["browser_download_url"].as_str())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    Ok(UpdateInfo {
        available: version_gt(&latest, current),
        current: current.to_string(),
        latest,
        notes,
        release_url,
        download_url,
        asset_name,
    })
}

#[tauri::command]
pub async fn check_update() -> AppResult<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    tauri::async_runtime::spawn_blocking(move || fetch_latest(&current))
        .await
        .map_err(|e| err(e.to_string()))?
}

fn download(url: &str, dest: &Path) -> AppResult<()> {
    let resp = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| err(format!("下载失败：{e}")))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| err(e.to_string()))?;
    // Guard against redirects to an error page / truncated download.
    if buf.len() < 512 * 1024 || !buf.starts_with(PLATFORM.magic) {
        return Err(err(format!(
            "下载的文件无效（{} 字节，非本平台可执行文件）。",
            buf.len()
        )));
    }
    std::fs::write(dest, &buf).map_err(|e| err(e.to_string()))?;
    Ok(())
}

fn sibling(exe: &Path, suffix: &str) -> AppResult<PathBuf> {
    let name = exe.file_name().ok_or_else(|| err("无法定位程序文件名。"))?;
    Ok(exe.with_file_name(format!("{}{suffix}", name.to_string_lossy())))
}

fn swap_in_place(download_url: &str) -> AppResult<()> {
    let exe = std::env::current_exe().map_err(|e| err(e.to_string()))?;
    let new_path = sibling(&exe, ".new")?;
    let old_path = sibling(&exe, ".old")?;

    download(download_url, &new_path)?;

    // Downloads land as 0644; restore the executable bit on Unix before the swap.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&new_path)
            .map_err(|e| err(e.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&new_path, perms).map_err(|e| err(e.to_string()))?;
    }

    let _ = std::fs::remove_file(&old_path); // clear any stale leftover
    std::fs::rename(&exe, &old_path)
        .map_err(|e| err(format!("重命名当前程序失败（目录可能只读或无写入权限）：{e}")))?;
    if let Err(e) = std::fs::rename(&new_path, &exe) {
        let _ = std::fs::rename(&old_path, &exe); // roll back
        return Err(err(format!("替换程序失败：{e}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, download_url: String) -> AppResult<()> {
    if !PLATFORM.self_update {
        return Err(err("此平台不支持应用内自动更新，请到发布页下载新版本手动安装。"));
    }
    if download_url.is_empty() {
        return Err(err("没有可用的下载地址。"));
    }
    tauri::async_runtime::spawn_blocking(move || swap_in_place(&download_url))
        .await
        .map_err(|e| err(e.to_string()))??;
    // Relaunch into the freshly-swapped binary. `restart` diverges.
    app.restart();
    #[allow(unreachable_code)]
    Ok(())
}

/// Remove leftover `.old` / `.new` files from a previous self-update. Best-effort.
pub fn cleanup_leftovers() {
    if let Ok(exe) = std::env::current_exe() {
        for suffix in [".old", ".new"] {
            if let Ok(p) = sibling(&exe, suffix) {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::version_gt;

    #[test]
    fn compares_versions_numerically() {
        assert!(version_gt("0.2.3", "0.2.2"));
        assert!(version_gt("0.3.0", "0.2.9"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(version_gt("v0.2.3", "0.2.2")); // tolerates a leading 'v'
        assert!(version_gt("0.2.10", "0.2.9")); // numeric, not lexicographic
        assert!(!version_gt("0.2.2", "0.2.2"));
        assert!(!version_gt("0.2.1", "0.2.2"));
        assert!(!version_gt("0.2.9", "0.2.10"));
    }
}
