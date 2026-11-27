//! 전용 서버 실행 파일입니다. 서버 라이브러리와 WebSocket 접속 방식을 조립합니다.
//!
//! 실행: `cargo run -p warship_dedicated` 또는 `cargo run -p warship_dedicated -- --addr 0.0.0.0:7777`
//! 방의 빈자리는 봇이 채웁니다. `--no-bots`를 주면 사람의 배만 있는 방이 됩니다.
//! 접속이 끊긴 사람의 자리는 60초 동안 남고, 그 안에 같은 클라이언트가 다시 접속하면 같은 배를 조종합니다.
//! 서버는 일어난 일(방의 생성과 제거, 입장과 퇴장, 끊긴 자리, 거절)을 한 줄씩 출력합니다. `Ctrl-C`로 종료합니다.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use warship_protocol::ServerLimits;
use warship_rules::{ShipId, Sim, Tuning, Waypoint};
use warship_server::{GameServer, Orders, Pilot, RandomSource};
use warship_ws_server::WsServer;

const DEFAULT_ADDR: &str = "127.0.0.1:7777";
const USAGE: &str = "usage: warship-dedicated [--addr HOST:PORT] [--no-bots]";

/// 다음 Tick까지 남은 시간이 길어도 이 간격마다 접속 이벤트를 처리합니다.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, PartialEq)]
struct Options {
    /// 접속을 받을 주소입니다.
    addr: String,
    /// 방의 빈자리를 봇으로 채우는지입니다.
    bots: bool,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        addr: DEFAULT_ADDR.to_string(),
        bots: true,
    };
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--addr" => {
                let addr = rest.next().ok_or("--addr needs HOST:PORT")?;
                options.addr = addr.clone();
            }
            "--no-bots" => options.bots = false,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

/// 빈자리의 배를 `warship_bot`의 계획대로 조종합니다.
struct BotPilot;

impl Pilot for BotPilot {
    fn steer(&mut self, sim: &Sim, ship: ShipId) -> Orders {
        let plan = warship_bot::plan(sim, ship);
        Orders {
            destination: plan
                .destination
                .and_then(|point| Waypoint::new(point.x, point.y)),
            commands: plan.commands,
        }
    }
}

/// 운영체제의 암호용 무작위입니다. 자리의 표식은 다른 사람이 예측할 수 없어야 합니다.
struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, bytes: &mut [u8]) {
        // 서버를 시작할 때 운영체제가 무작위 바이트를 주는지 확인했습니다.
        getrandom::fill(bytes).expect("the operating system gives random bytes");
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Options { addr, bots } = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if let Err(err) = getrandom::fill(&mut [0u8; 16]) {
        eprintln!("cannot start the server: the operating system gives no random bytes: {err}");
        return ExitCode::FAILURE;
    }
    let limits = ServerLimits::default();
    let ws = match WsServer::bind(&addr, limits) {
        Ok(ws) => ws,
        Err(err) => {
            eprintln!("cannot listen on {addr}: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("listening on ws://{}", ws.local_addr());

    let server = GameServer::new(
        Box::new(ws),
        Tuning::default(),
        limits,
        Box::new(OsRandom),
        Instant::now(),
    );
    let mut server = match server {
        Ok(server) if bots => server.with_pilot(Box::new(BotPilot)),
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
    fn the_options_come_from_the_arguments_or_the_defaults() {
        let options = |addr: &str, bots| Options {
            addr: addr.to_string(),
            bots,
        };
        assert_eq!(parse_args(&[]), Ok(options(DEFAULT_ADDR, true)));
        assert_eq!(
            parse_args(&args(&["--no-bots", "--addr", "0.0.0.0:9000"])),
            Ok(options("0.0.0.0:9000", false))
        );
        assert!(parse_args(&args(&["--port", "9000"])).is_err());
        assert!(parse_args(&args(&["--addr"])).is_err());
    }
}
