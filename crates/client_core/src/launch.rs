//! 실행 인자를 해석합니다. PC는 명령줄의 인자를, 웹은 페이지 주소의 쿼리를 읽습니다. 두 쪽이 같은 검사를 거칩니다.

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
            let key = flag.strip_prefix("--");
            let key = key.ok_or(format!("unknown option {flag}"))?;
            options.set(key, value)?;
        }
        Ok(options)
    }

    /// 페이지 주소의 쿼리(`?room=duel&name=Kang&server=wss://example.test`)를 읽습니다. 주지 않은 값은 기본값입니다.
    /// `room`을 주면 시작 화면 없이 바로 입장합니다. 값이 없는 `new-room`은 서버가 만든 새 방에 들어갑니다.
    pub fn from_query(query: &str) -> Result<LaunchOptions, String> {
        let mut options = LaunchOptions::default();
        let pairs = query.trim_start_matches('?').split('&');
        for pair in pairs.filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if key == "new-room" {
                options.room = RoomChoice::New;
            } else {
                options.set(key, &decode(value))?;
            }
        }
        Ok(options)
    }

    /// 이름이 `key`인 값 하나를 검사해서 넣습니다.
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            // 서버 주소는 기기의 기억에 한 줄로 저장하므로, 공백과 제어 문자가 있는 주소는 받지 않습니다.
            "server" if value.chars().any(|c| c.is_whitespace() || c.is_control()) => {
                return Err("the server address must not contain spaces".to_string());
            }
            "server" if value.starts_with("ws://") || value.starts_with("wss://") => {
                self.server = value.to_string();
            }
            "server" => {
                return Err(format!(
                    "the server {value} must start with ws:// or wss://"
                ));
            }
            "room" => self.room = RoomChoice::Code(value.to_string()),
            "name" => {
                let name = PlayerName::parse(value).map_err(|err| err.to_string())?;
                self.name = Some(name.as_str().to_string());
            }
            // 파일 이름에 들어가므로 글자와 숫자만 받습니다.
            "profile" if value.chars().all(|c| c.is_ascii_alphanumeric()) => {
                self.profile = Some(value.to_string());
            }
            "profile" => {
                return Err(format!(
                    "the profile {value} may use letters and digits only"
                ));
            }
            _ => return Err(format!("unknown option {key}")),
        }
        Ok(())
    }
}

/// 쿼리의 값에서 `+`를 공백으로, `%`와 16진수 두 글자를 그 바이트로 바꿉니다. 16진수가 아닌 `%`는 그대로 둡니다.
/// 바꾼 결과가 UTF-8이 아니면 잘못된 바이트를 대체 문자로 바꿉니다. 그 값은 이름이나 방 코드의 검사에서 걸립니다.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes.get(at + 1..at + 3).and_then(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        });
        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                at += 3;
            }
            (b'+', _) => {
                decoded.push(b' ');
                at += 1;
            }
            (byte, _) => {
                decoded.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
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
    fn a_page_query_gives_the_same_options_as_the_arguments() {
        assert_eq!(LaunchOptions::from_query(""), Ok(LaunchOptions::default()));
        assert_eq!(LaunchOptions::from_query("?"), Ok(LaunchOptions::default()));

        let query = "?room=duel&name=Kang+Lee&server=wss%3A%2F%2Fexample.test";
        let options = LaunchOptions::from_query(query).unwrap();
        assert_eq!(options.room, RoomChoice::Code("duel".to_string()));
        assert_eq!(options.name.as_deref(), Some("Kang Lee"));
        assert_eq!(options.server, "wss://example.test");

        let options = LaunchOptions::from_query("?new-room&name=Kang%20Lee").unwrap();
        assert_eq!(options.room, RoomChoice::New);
        assert_eq!(options.name.as_deref(), Some("Kang Lee"));
    }

    #[test]
    fn a_broken_page_query_is_refused() {
        assert!(LaunchOptions::from_query("?name=K%40NG").is_err());
        assert!(LaunchOptions::from_query("?server=http://example.test").is_err());
        assert!(LaunchOptions::from_query("?volume=3").is_err());
        // 줄바꿈이 든 서버 주소는 기억 파일에 다른 줄을 끼워 넣을 수 있으므로 거절합니다.
        assert!(LaunchOptions::from_query("?server=ws://a.test%0Aname=X").is_err());
        assert!(LaunchOptions::from_query("?server=ws://a.test+b").is_err());
        // 16진수가 아닌 `%`는 그대로 남아서 이름의 검사에 걸립니다.
        assert!(LaunchOptions::from_query("?name=100%").is_err());
    }

    #[test]
    fn broken_arguments_are_refused() {
        assert!(parse(&["--room"]).is_err());
        assert!(parse(&["--server", "http://example.test"]).is_err());
        assert!(parse(&["--name", "K@NG"]).is_err());
        assert!(parse(&["--profile", "../other"]).is_err());
        assert!(parse(&["--port", "7777"]).is_err());
        assert!(parse(&["room", "duel"]).is_err());
    }
}
