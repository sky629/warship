//! 실제 소켓 위에서 조립한 서버를 확인합니다.
//!
//! 서버의 규칙은 `warship_server`의 테스트가 가짜 시계와 프로세스 내부 전달로 확인합니다.
//! 이 파일은 같은 서버가 WebSocket 위에서도 같은 결과를 내는지, 즉 접속이 방에 들어가서
//! 이동 입력이 스냅샷에서 배의 움직임으로 보이는지를 확인합니다.

use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use tokio_tungstenite::tungstenite::{Message, WebSocket, connect};
use warship_protocol::{
    ClientMsg, InputMsg, PROTOCOL_VERSION, ServerLimits, ServerMsg, WireTeam, WireVec2,
    decode_server, encode_client,
};
use warship_rules::Tuning;
use warship_server::GameServer;
use warship_ws_server::WsServer;

/// 실행 파일(`main.rs`)과 같은 방식으로 스레드 하나에서 서버를 돌립니다. 버려지면 스레드를 멈춥니다.
struct RunningServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RunningServer {
    fn start() -> RunningServer {
        let limits = ServerLimits::default();
        let ws = WsServer::bind("127.0.0.1:0", limits).expect("bind a free port");
        let addr = ws.local_addr();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let mut server =
                GameServer::new(Box::new(ws), Tuning::default(), limits, Instant::now())
                    .expect("the default tuning is valid");
            while !stopped.load(Ordering::Relaxed) {
                let now = Instant::now();
                server.pump(now);
                let rest = server.time_until_next_tick(now);
                thread::sleep(rest.min(Duration::from_millis(5)));
            }
        });
        RunningServer {
            addr,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

fn open(addr: SocketAddr) -> Socket {
    let (socket, _) = connect(format!("ws://{addr}")).expect("the server accepts the link");
    if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
        let timeout = Some(Duration::from_secs(5));
        stream.set_read_timeout(timeout).expect("set a timeout");
    }
    socket
}

fn send(socket: &mut Socket, msg: &ClientMsg) {
    let bytes = encode_client(msg);
    socket.send(Message::Binary(bytes.into())).expect("send");
}

/// 다음 메시지를 읽습니다. 5초 안에 오지 않으면 테스트가 실패합니다.
fn read(socket: &mut Socket) -> ServerMsg {
    loop {
        match socket.read().expect("the server sends within the timeout") {
            Message::Binary(bytes) => return decode_server(&bytes).expect("a server message"),
            _ => continue,
        }
    }
}

#[test]
fn a_ship_joins_over_a_websocket_and_its_input_moves_it() {
    let server = RunningServer::start();
    let mut socket = open(server.addr);
    send(
        &mut socket,
        &ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: "socket".to_string(),
        },
    );

    let ServerMsg::Welcome(welcome) = read(&mut socket) else {
        panic!("the first message is the welcome");
    };
    assert_eq!(welcome.team, WireTeam::Red);
    send(
        &mut socket,
        &ClientMsg::Input(InputMsg {
            seq: 1,
            destination: Some(WireVec2::new(20.0, 24.0)),
        }),
    );

    // 기본 맵에서 빨간 진영의 시작 자리는 x = 6입니다. 배가 1타일 넘게 움직인 스냅샷이 올 때까지 읽습니다.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "the ship did not move in time");
        if let ServerMsg::Snapshot(snapshot) = read(&mut socket) {
            let ship = &snapshot.ships[0];
            assert_eq!(ship.id, welcome.ship);
            if ship.x > 7.0 {
                assert_eq!(ship.last_input_seq, 1);
                break;
            }
        }
    }
}
