//! 전용 서버 실행 파일입니다. 서버 라이브러리와 WebSocket 접속 방식을 조립합니다.
//!
//! 실행: `cargo run -p warship_dedicated` 또는 `cargo run -p warship_dedicated -- --addr 0.0.0.0:7777`
//! 서버는 일어난 일(방의 생성과 제거, 입장과 퇴장, 거절)을 한 줄씩 출력합니다. `Ctrl-C`로 종료합니다.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use warship_protocol::ServerLimits;
use warship_rules::Tuning;
use warship_server::GameServer;
use warship_ws_server::WsServer;

const DEFAULT_ADDR: &str = "127.0.0.1:7777";
const USAGE: &str = "usage: warship-dedicated [--addr HOST:PORT]";

/// 다음 Tick까지 남은 시간이 길어도 이 간격마다 접속 이벤트를 처리합니다.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// 실행 인자에서 접속을 받을 주소를 읽습니다.
fn parse_args(args: &[String]) -> Result<String, String> {
    match args {
        [] => Ok(DEFAULT_ADDR.to_string()),
        [flag, addr] if flag == "--addr" => Ok(addr.clone()),
        _ => Err(format!("unknown arguments: {}", args.join(" "))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let addr = match parse_args(&args) {
        Ok(addr) => addr,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let limits = ServerLimits::default();
    let ws = match WsServer::bind(&addr, limits) {
        Ok(ws) => ws,
        Err(err) => {
            eprintln!("cannot listen on {addr}: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("listening on ws://{}", ws.local_addr());

    let mut server = match GameServer::new(Box::new(ws), Tuning::default(), limits, Instant::now())
    {
        Ok(server) => server,
        Err(err) => {
            eprintln!("cannot start the server: {err}");
            return ExitCode::FAILURE;
        }
    };
    // 프로세스를 종료하면(Ctrl-C) 반복도 끝납니다.
    loop {
        let now = Instant::now();
        for notice in server.pump(now) {
            eprintln!("{notice}");
        }
        std::thread::sleep(server.time_until_next_tick(now).min(POLL_INTERVAL));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn the_address_comes_from_the_arguments_or_the_default() {
        assert_eq!(parse_args(&[]), Ok(DEFAULT_ADDR.to_string()));
        assert_eq!(
            parse_args(&args(&["--addr", "0.0.0.0:9000"])),
            Ok("0.0.0.0:9000".to_string())
        );
        assert!(parse_args(&args(&["--port", "9000"])).is_err());
        assert!(parse_args(&args(&["--addr"])).is_err());
    }
}
