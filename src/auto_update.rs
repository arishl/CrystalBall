use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

const REPO_API: &str = "https://api.github.com/repos/arishl/CrystalBall/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub enum UpdateStatus {
    Installed(String),
    Skipped(String),
    Failed(String),
}

pub fn start_check() -> Receiver<UpdateStatus> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        if env::var_os("CRYSTALBALL_DISABLE_AUTO_UPDATE").is_some() {
            let _ = sender.send(UpdateStatus::Skipped("Auto updates disabled".to_owned()));
            return;
        }

        if !looks_installed() {
            let _ = sender.send(UpdateStatus::Skipped(
                "Auto updates only run from installed builds".to_owned(),
            ));
            return;
        }

        let result = check_and_install();
        let _ = sender.send(match result {
            Ok(Some(version)) => UpdateStatus::Installed(version),
            Ok(None) => UpdateStatus::Skipped("CrystalBall is up to date".to_owned()),
            Err(err) => UpdateStatus::Failed(err),
        });
    });

    receiver
}

fn check_and_install() -> Result<Option<String>, String> {
    let api_body = curl_to_string(REPO_API)?;
    let latest_tag = json_string_field(&api_body, "tag_name")
        .ok_or_else(|| "Could not read latest release tag".to_owned())?;
    let latest_version = latest_tag.trim_start_matches('v');

    if !is_newer_version(latest_version, CURRENT_VERSION) {
        return Ok(None);
    }

    let asset_name = platform_asset_name()?;
    let download_url = browser_download_url(&api_body, asset_name)
        .ok_or_else(|| format!("Latest release does not include {asset_name}"))?;
    let staging_dir = prepare_staging_dir(latest_version)?;
    let archive_path = staging_dir.join(asset_name);

    curl_to_file(&download_url, &archive_path)?;
    extract_archive(&archive_path, &staging_dir)?;
    install_from_staging(&staging_dir)?;

    Ok(Some(latest_tag))
}

fn platform_asset_name() -> Result<&'static str, String> {
    match env::consts::OS {
        "linux" => Ok("CrystalBall-linux-x86_64.tar.gz"),
        "macos" => Ok("CrystalBall-macos-app.tar.gz"),
        "windows" => Ok("CrystalBall-windows-x86_64.zip"),
        other => Err(format!("Auto updates are not configured for {other}")),
    }
}

fn curl_to_string(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "5",
            "--max-time",
            "30",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: CrystalBall-auto-updater",
            url,
        ])
        .output()
        .map_err(|err| format!("Could not start curl: {err}"))?;

    if !output.status.success() {
        return Err(format!("Update check failed with {}", output.status));
    }

    String::from_utf8(output.stdout).map_err(|err| format!("GitHub response was not UTF-8: {err}"))
}

fn curl_to_file(url: &str, output_path: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "-fL",
            "--connect-timeout",
            "5",
            "--max-time",
            "300",
            "-H",
            "User-Agent: CrystalBall-auto-updater",
            "-o",
        ])
        .arg(output_path)
        .arg(url)
        .status()
        .map_err(|err| format!("Could not download update: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Update download failed with {status}"))
    }
}

fn prepare_staging_dir(version: &str) -> Result<PathBuf, String> {
    let staging_dir = env::temp_dir().join(format!("crystalball-update-{version}"));
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)
            .map_err(|err| format!("Could not clear update staging directory: {err}"))?;
    }
    fs::create_dir_all(&staging_dir)
        .map_err(|err| format!("Could not create update staging directory: {err}"))?;
    Ok(staging_dir)
}

fn extract_archive(archive_path: &Path, staging_dir: &Path) -> Result<(), String> {
    let status = if archive_path.extension().and_then(|ext| ext.to_str()) == Some("zip") {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", "Expand-Archive -Force -Path"])
            .arg(archive_path)
            .arg("-DestinationPath")
            .arg(staging_dir)
            .status()
    } else {
        Command::new("tar")
            .arg("-xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(staging_dir)
            .status()
    }
    .map_err(|err| format!("Could not extract update: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Update extraction failed with {status}"))
    }
}

fn install_from_staging(staging_dir: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let installer = staging_dir.join("install.ps1");
        if !installer.exists() {
            return Err("Update package is missing install.ps1".to_owned());
        }

        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(installer)
            .arg(std::process::id().to_string())
            .status()
            .map_err(|err| format!("Could not start updater install script: {err}"))?;

        return if status.success() {
            Ok(())
        } else {
            Err(format!("Updater install script failed with {status}"))
        };
    }

    #[cfg(not(target_os = "windows"))]
    {
        let installer = staging_dir.join("install.sh");
        if !installer.exists() {
            return Err("Update package is missing install.sh".to_owned());
        }

        let mut command = Command::new("bash");
        command.arg(installer);

        if cfg!(target_os = "macos") {
            if let Some(install_dir) = current_macos_install_dir() {
                command.arg(install_dir);
            }
        }

        let status = command
            .current_dir(staging_dir)
            .status()
            .map_err(|err| format!("Could not start updater install script: {err}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Updater install script failed with {status}"))
        }
    }
}

fn current_macos_install_dir() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok()?;
    let app_root = current_exe.ancestors().find(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension == "app")
    })?;
    app_root.parent().map(Path::to_path_buf)
}

fn looks_installed() -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };

    if current_exe.components().any(|component| {
        let component = component.as_os_str().to_string_lossy();
        component == "target" || component == "debug" || component == "release"
    }) {
        return false;
    }

    if cfg!(target_os = "macos") {
        return current_exe.ancestors().any(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension == "app")
        });
    }

    true
}

fn json_string_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let start = body.find(&needle)? + needle.len();
    parse_json_string(&body[start..])
}

fn browser_download_url(body: &str, asset_name: &str) -> Option<String> {
    let mut remainder = body;
    while let Some(name_index) = remainder.find("\"name\":") {
        remainder = &remainder[name_index + "\"name\":".len()..];
        let name = parse_json_string(remainder)?;
        if name == asset_name {
            let url_index = remainder.find("\"browser_download_url\":")?;
            let url_start = url_index + "\"browser_download_url\":".len();
            return parse_json_string(&remainder[url_start..]);
        }
    }
    None
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.trim_start().chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for char in chars {
        if escaped {
            value.push(char);
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '"' {
            return Some(value);
        } else {
            value.push(char);
        }
    }
    None
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    version_parts(latest) > version_parts(current)
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split(|char: char| !char.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}
