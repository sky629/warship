//! 기기의 저장소입니다. `Memory`의 글자 하나를 읽고 씁니다. 읽지 못하면 빈 기억으로 시작하고, 쓰지 못하면 경고만 남깁니다.
//!
//! 웹: `localStorage`의 `warship.memory`입니다. 새로 고침해도 남지만 사생활 보호 창에서는 막힐 수 있습니다.
//! 같은 브라우저 프로필의 탭은 값을 함께 쓰므로, 두 사람을 테스트하려면 서로 다른 브라우저 프로필을 씁니다.
//! PC: 설정 폴더의 `warship/memory.txt`입니다. 설정 폴더는 Windows가 `%APPDATA%`, macOS가 `~/Library/Application Support`,
//! 그 밖은 `$XDG_CONFIG_HOME` 또는 `~/.config`이고, 환경 변수만 읽습니다.
//! 같은 컴퓨터에서 클라이언트 두 개를 실행하면 같은 파일을 쓰면서 자리의 표식을 서로 덮어쓰므로, `--profile 이름`이 있으면
//! 그 이름의 `memory-이름.txt`를 따로 씁니다. 파일은 임시 파일에 쓴 뒤 바꿔 넣는데, 임시 파일의 이름에 프로세스 번호가 있어서 두 클라이언트가 같은 임시 파일을 쓰지 않습니다.
//! 웹은 `profile`을 쓰지 않습니다.

use bevy::prelude::warn;
use warship_client_core::Memory;

/// 기기에 저장된 기억입니다. 저장된 글자가 없거나 읽을 수 없으면 빈 기억입니다.
pub fn load(profile: Option<&str>) -> Memory {
    read_text(profile)
        .map(|text| Memory::parse(&text))
        .unwrap_or_default()
}

/// `Memory::to_text`의 글자를 저장합니다.
pub fn save(profile: Option<&str>, text: &str) {
    if let Err(reason) = write_text(profile, text) {
        warn!("cannot save what this device remembers: {reason}");
    }
}

#[cfg(target_arch = "wasm32")]
const KEY: &str = "warship.memory";

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn read_text(_profile: Option<&str>) -> Option<String> {
    local_storage()?.get_item(KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn write_text(_profile: Option<&str>, text: &str) -> Result<(), String> {
    local_storage()
        .ok_or_else(|| "the browser gives no local storage".to_string())?
        .set_item(KEY, text)
        .map_err(|err| format!("{err:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::path::{Path, PathBuf};

    /// 운영체제에 따른 설정 폴더입니다. `var`는 환경 변수를 읽는 함수이고, 테스트가 값을 넣어 줍니다.
    pub fn config_dir(os: &str, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
        match os {
            "windows" => var("APPDATA").map(PathBuf::from),
            "macos" => var("HOME").map(|home| Path::new(&home).join("Library/Application Support")),
            _ => var("XDG_CONFIG_HOME")
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from)
                .or_else(|| var("HOME").map(|home| Path::new(&home).join(".config"))),
        }
    }

    /// 기억을 담는 파일의 이름입니다. `profile`이 있으면 그 이름의 파일을 따로 씁니다.
    pub fn file_name(profile: Option<&str>) -> String {
        match profile {
            Some(profile) => format!("memory-{profile}.txt"),
            None => "memory.txt".to_string(),
        }
    }

    pub fn memory_path(profile: Option<&str>) -> Option<PathBuf> {
        let dir = config_dir(std::env::consts::OS, |name| std::env::var(name).ok())?;
        Some(dir.join("warship").join(file_name(profile)))
    }

    /// `path`에 쓸 내용을 먼저 담는 임시 파일입니다. 이름에 프로세스 번호가 있어서, 같은 파일을 쓰는 클라이언트 둘이 서로의 임시 파일을 덮어쓰지 않습니다.
    pub fn temp_path(path: &Path, process: u32) -> PathBuf {
        path.with_extension(format!("{process}.tmp"))
    }

    /// 같은 폴더의 임시 파일에 쓴 뒤 바꿔 넣으므로, 쓰다가 끊겨도 이전 파일이 남습니다.
    pub fn write_to(path: &Path, text: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = temp_path(path, std::process::id());
        write_private(&temp, text)?;
        std::fs::rename(&temp, path)
    }

    /// 파일에 자리의 표식이 들어 있으므로, 이 컴퓨터의 다른 사용자가 읽지 못하게 소유자만 읽고 쓰는 파일로 만듭니다.
    #[cfg(unix)]
    fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
            .write_all(text.as_bytes())
    }

    #[cfg(not(unix))]
    fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
        std::fs::write(path, text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_text(profile: Option<&str>) -> Option<String> {
    std::fs::read_to_string(desktop::memory_path(profile)?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_text(profile: Option<&str>, text: &str) -> Result<(), String> {
    let path =
        desktop::memory_path(profile).ok_or_else(|| "no settings folder is known".to_string())?;
    desktop::write_to(&path, text).map_err(|err| format!("{}: {err}", path.display()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::desktop::*;
    use std::path::Path;

    fn vars(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn each_system_keeps_its_settings_in_its_own_folder() {
        assert_eq!(
            config_dir(
                "windows",
                vars(&[("APPDATA", "C:\\Users\\k\\AppData\\Roaming")])
            ),
            Some(Path::new("C:\\Users\\k\\AppData\\Roaming").to_path_buf())
        );
        assert_eq!(
            config_dir("macos", vars(&[("HOME", "/Users/k")])),
            Some(Path::new("/Users/k/Library/Application Support").to_path_buf())
        );
        assert_eq!(
            config_dir("linux", vars(&[("HOME", "/home/k")])),
            Some(Path::new("/home/k/.config").to_path_buf())
        );
        assert_eq!(
            config_dir(
                "linux",
                vars(&[("HOME", "/home/k"), ("XDG_CONFIG_HOME", "/data/cfg")])
            ),
            Some(Path::new("/data/cfg").to_path_buf())
        );
        assert_eq!(
            config_dir(
                "linux",
                vars(&[("HOME", "/home/k"), ("XDG_CONFIG_HOME", "")])
            ),
            Some(Path::new("/home/k/.config").to_path_buf()),
            "an empty variable counts as unset"
        );
    }

    #[test]
    fn a_system_without_the_variable_has_no_folder() {
        assert_eq!(config_dir("macos", vars(&[])), None);
        assert_eq!(config_dir("windows", vars(&[("HOME", "/x")])), None);
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("warship-private-{}", std::process::id()));
        let path = dir.join("memory.txt");

        write_to(&path, "name=KANG\n").expect("the file is written");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_is_written_is_read_back_and_a_second_write_replaces_it() {
        let dir = std::env::temp_dir().join(format!("warship-storage-{}", std::process::id()));
        let path = dir.join("warship").join("memory.txt");

        write_to(&path, "name=KANG\n").expect("the folder is made on the first write");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "name=KANG\n");
        write_to(&path, "name=MIRO\n").expect("the file is replaced");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "name=MIRO\n");
        let left: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            left,
            [std::ffi::OsString::from("memory.txt")],
            "no temporary file is left"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn two_processes_writing_the_same_file_use_temporary_files_of_their_own() {
        let path = Path::new("/data/warship/memory.txt");

        let first = temp_path(path, 41);
        let second = temp_path(path, 42);

        assert_ne!(first, second);
        assert_ne!(first, path);
        assert_eq!(
            first.parent(),
            path.parent(),
            "renaming stays in one folder"
        );
    }

    #[test]
    fn a_profile_has_a_file_of_its_own() {
        assert_eq!(file_name(None), "memory.txt");
        assert_eq!(file_name(Some("two")), "memory-two.txt");
        assert_ne!(file_name(Some("one")), file_name(Some("two")));
    }
}
