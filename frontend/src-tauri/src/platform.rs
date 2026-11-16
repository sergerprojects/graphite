//! The few places where Mac, Windows and Linux differ for the things Graphite launches:
//! the environment child programs get, where a virtual environment keeps its programs,
//! opening a file or folder, and the log folder.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Where the `claude` command and other user-installed tools usually live, joined with the
/// platform's path separator and followed by the standard system folders.
pub fn tool_path() -> OsString {
    let home = dirs::home_dir().unwrap_or_default();
    let mut dirs_: Vec<PathBuf> = vec![home.join(".local/bin"), home.join(".claude/local")];
    #[cfg(windows)]
    {
        if let Some(a) = std::env::var_os("APPDATA") {
            dirs_.push(PathBuf::from(a).join("npm"));
        }
        if let Some(a) = std::env::var_os("LOCALAPPDATA") {
            dirs_.push(PathBuf::from(a).join("Programs").join("claude"));
        }
        if let Some(sys) = std::env::var_os("PATH") {
            dirs_.extend(std::env::split_paths(&sys));
        }
    }
    #[cfg(not(windows))]
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
        dirs_.push(PathBuf::from(d));
    }
    std::env::join_paths(dirs_).unwrap_or_default()
}

/// What a child program needs after `env_clear()`: home, a path, a temp folder, and on
/// Windows the system variables programs fail without.
pub fn base_env() -> Vec<(&'static str, OsString)> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut v: Vec<(&'static str, OsString)> = vec![
        ("HOME", home.clone().into_os_string()),
        ("PATH", tool_path()),
        ("TMPDIR", std::env::temp_dir().into_os_string()),
    ];
    #[cfg(windows)]
    {
        v.push(("USERPROFILE", home.into_os_string()));
        for k in ["SystemRoot", "SYSTEMDRIVE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP", "USERNAME", "PROGRAMFILES", "COMSPEC"] {
            if let Some(val) = std::env::var_os(k) {
                v.push((k, val));
            }
        }
    }
    v
}

/// The Python program inside a virtual environment folder.
pub fn venv_python(venv: &Path) -> PathBuf {
    if cfg!(windows) { venv.join("Scripts").join("python.exe") } else { venv.join("bin").join("python") }
}

/// The program that installs packages inside a virtual environment.
pub fn venv_pip(venv: &Path) -> PathBuf {
    if cfg!(windows) { venv.join("Scripts").join("pip.exe") } else { venv.join("bin").join("pip") }
}

/// The command and leading arguments that start the system's Python 3.
pub fn system_python() -> (&'static str, &'static [&'static str]) {
    if cfg!(windows) { ("py", &["-3"]) } else { ("/usr/bin/python3", &[]) }
}

/// Open a file, folder or https link with the system's default program.
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(target).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(target).spawn();
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let r = std::process::Command::new("xdg-open").arg(target).spawn();
    r.map(|_| ())
}

/// Folder for Graphite's own log files.
pub fn log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Some(dirs::home_dir()?.join("Library/Logs/Graphite"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(dirs::data_local_dir()?.join("Graphite").join("Logs"))
    }
}

/// Shown to people when Claude Code is missing.
pub fn this_computer() -> &'static str {
    if cfg!(target_os = "macos") { "this Mac" } else { "this computer" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venv_programs_sit_where_the_platform_puts_them() {
        let v = Path::new("/x/venv");
        assert!(venv_python(v).ends_with(if cfg!(windows) { "Scripts/python.exe" } else { "bin/python" }));
        assert!(venv_pip(v).ends_with(if cfg!(windows) { "Scripts/pip.exe" } else { "bin/pip" }));
    }

    #[test]
    fn tool_path_has_the_claude_folders_first() {
        let p = tool_path();
        let first = std::env::split_paths(&p).next().unwrap();
        assert!(first.ends_with(".local/bin"));
        assert!(std::env::split_paths(&p).count() >= 3);
    }

    #[test]
    fn base_env_has_home_path_and_temp() {
        let keys: Vec<&str> = base_env().iter().map(|(k, _)| *k).collect();
        for k in ["HOME", "PATH", "TMPDIR"] {
            assert!(keys.contains(&k), "{k}");
        }
    }
}
