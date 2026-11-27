//! 실행 인자를 해석합니다.

use warship_protocol::PlayerName;

pub const DEFAULT_SERVER: &str = "ws://127.0.0.1:7777";
pub const DEFAULT_ROOM: &str = "main";
pub const DEFAULT_NAME: &str = "Captain";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    /// 서버 주소입니다. `ws://`나 `wss://`로 시작합니다.
    pub server: String,
    /// 들어갈 방의 코드입니다. 서버가 검사합니다. 없으면 서버가 새 방을 만들고 그 코드를 알려 줍니다.
    pub room: Option<String>,
    /// 플레이어의 이름입니다.
    pub name: String,
}

impl Default for LaunchOptions {
    fn default() -> LaunchOptions {
        LaunchOptions {
            server: DEFAULT_SERVER.to_string(),
            room: Some(DEFAULT_ROOM.to_string()),
            name: DEFAULT_NAME.to_string(),
        }
    }
}

impl LaunchOptions {
    /// `--server URL`, `--room CODE`, `--name NAME`, `--new-room`을 읽습니다. 주지 않은 값은 기본값입니다.
    /// `--new-room`은 서버가 만든 새 방에 들어갑니다.
    pub fn parse(args: &[String]) -> Result<LaunchOptions, String> {
        let mut options = LaunchOptions::default();
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            if flag == "--new-room" {
                options.room = None;
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
                "--room" => options.room = Some(value.clone()),
                "--name" => {
                    let name = PlayerName::parse(value).map_err(|err| err.to_string())?;
                    options.name = name.as_str().to_string();
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
        ]);
        let options = options.unwrap();
        assert_eq!(options.room.as_deref(), Some("duel"));
        assert_eq!(options.server, "wss://example.test");
        assert_eq!(options.name, "Kang");
    }

    #[test]
    fn a_new_room_has_no_code() {
        assert_eq!(parse(&["--new-room"]).unwrap().room, None);
        assert_eq!(
            parse(&["--new-room", "--room", "duel"]).unwrap().room,
            Some("duel".to_string()),
            "the later option decides"
        );
    }

    #[test]
    fn broken_arguments_are_refused() {
        assert!(parse(&["--room"]).is_err());
        assert!(parse(&["--server", "http://example.test"]).is_err());
        assert!(parse(&["--name", "K@NG"]).is_err());
        assert!(parse(&["--port", "7777"]).is_err());
    }
}
