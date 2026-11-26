//! 실행 인자를 해석합니다.

pub const DEFAULT_SERVER: &str = "ws://127.0.0.1:7777";
pub const DEFAULT_ROOM: &str = "main";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    /// 서버 주소입니다. `ws://`나 `wss://`로 시작합니다.
    pub server: String,
    /// 들어갈 방의 코드입니다. 서버가 검사합니다.
    pub room: String,
}

impl Default for LaunchOptions {
    fn default() -> LaunchOptions {
        LaunchOptions {
            server: DEFAULT_SERVER.to_string(),
            room: DEFAULT_ROOM.to_string(),
        }
    }
}

impl LaunchOptions {
    /// `--server URL`과 `--room CODE`를 읽습니다. 주지 않은 값은 기본값입니다.
    pub fn parse(args: &[String]) -> Result<LaunchOptions, String> {
        let mut options = LaunchOptions::default();
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
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
                "--room" => options.room = value.clone(),
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
        let options = parse(&["--room", "duel", "--server", "wss://example.test"]).unwrap();
        assert_eq!(options.room, "duel");
        assert_eq!(options.server, "wss://example.test");
    }

    #[test]
    fn broken_arguments_are_refused() {
        assert!(parse(&["--room"]).is_err());
        assert!(parse(&["--server", "http://example.test"]).is_err());
        assert!(parse(&["--name", "KANG"]).is_err());
    }
}
