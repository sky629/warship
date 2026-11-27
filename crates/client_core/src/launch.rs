//! 실행 인자를 해석합니다.

use warship_protocol::PlayerName;

pub const DEFAULT_SERVER: &str = "ws://127.0.0.1:7777";
pub const DEFAULT_NAME: &str = "Captain";

/// 실행 인자가 정한 방입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomChoice {
    /// 시작 화면에서 이름과 방 코드를 받습니다.
    Ask,
    /// 시작 화면 없이 서버가 만든 새 방에 들어갑니다.
    New,
    /// 시작 화면 없이 이 코드의 방에 들어갑니다. 코드는 서버가 검사합니다.
    Code(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    /// 서버 주소입니다. `ws://`나 `wss://`로 시작합니다.
    pub server: String,
    pub room: RoomChoice,
    /// 플레이어의 이름입니다. 없으면 기기가 기억하는 이름을 쓰고, 그것도 없으면 `DEFAULT_NAME`입니다.
    pub name: Option<String>,
    /// 기기의 기억을 따로 쓰는 이름입니다. 같은 컴퓨터에서 클라이언트 두 개를 실행할 때 서로의 자리를 덮어쓰지 않게 합니다.
    pub profile: Option<String>,
}

impl Default for LaunchOptions {
    fn default() -> LaunchOptions {
        LaunchOptions {
            server: DEFAULT_SERVER.to_string(),
            room: RoomChoice::Ask,
            name: None,
            profile: None,
        }
    }
}

impl LaunchOptions {
    /// `--server URL`, `--room CODE`, `--new-room`, `--name NAME`, `--profile NAME`을 읽습니다. 주지 않은 값은 기본값입니다.
    /// `--room`이나 `--new-room`을 주면 시작 화면 없이 바로 입장합니다.
    pub fn parse(args: &[String]) -> Result<LaunchOptions, String> {
        let mut options = LaunchOptions::default();
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            if flag == "--new-room" {
                options.room = RoomChoice::New;
                continue;
            }
            let value = rest.next().ok_or(format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--server" if value.starts_with("ws://") || value.starts_with("wss://") => {
                    options.server = value.clone();
                }
                "--server" => {
                    return Err(format!(
                        "the server {value} must start with ws:// or wss://"
                    ));
                }
                "--room" => options.room = RoomChoice::Code(value.clone()),
                "--name" => {
                    let name = PlayerName::parse(value).map_err(|err| err.to_string())?;
                    options.name = Some(name.as_str().to_string());
                }
                // 파일 이름에 들어가므로 글자와 숫자만 받습니다.
                "--profile" if value.chars().all(|c| c.is_ascii_alphanumeric()) => {
                    options.profile = Some(value.clone());
                }
                "--profile" => {
                    return Err(format!(
                        "the profile {value} may use letters and digits only"
                    ));
                }
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(list: &[&str]) -> Result<LaunchOptions, String> {
        let args: Vec<String> = list.iter().map(|arg| arg.to_string()).collect();
        LaunchOptions::parse(&args)
    }

    #[test]
    fn options_come_from_the_arguments_or_the_defaults() {
        assert_eq!(parse(&[]), Ok(LaunchOptions::default()));
        let options = parse(&[
            "--room",
            "duel",
            "--server",
            "wss://example.test",
            "--name",
            " Kang ",
            "--profile",
            "second",
        ]);
        let options = options.unwrap();
        assert_eq!(options.room, RoomChoice::Code("duel".to_string()));
        assert_eq!(options.server, "wss://example.test");
        assert_eq!(options.name.as_deref(), Some("Kang"));
        assert_eq!(options.profile.as_deref(), Some("second"));
    }

    #[test]
    fn the_later_room_option_decides() {
        assert_eq!(parse(&["--new-room"]).unwrap().room, RoomChoice::New);
        assert_eq!(
            parse(&["--new-room", "--room", "duel"]).unwrap().room,
            RoomChoice::Code("duel".to_string())
        );
    }

    #[test]
    fn broken_arguments_are_refused() {
        assert!(parse(&["--room"]).is_err());
        assert!(parse(&["--server", "http://example.test"]).is_err());
        assert!(parse(&["--name", "K@NG"]).is_err());
        assert!(parse(&["--profile", "../other"]).is_err());
        assert!(parse(&["--port", "7777"]).is_err());
    }
}
