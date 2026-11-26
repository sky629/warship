//! 서버가 접속 하나와 메시지 하나에 허용하는 한계입니다.
//!
//! 게임 서버(`warship_server`)와 서버 쪽 접속 방식(`warship_ws_server`)이 함께 읽으므로, 둘이 공통으로 의존하는 이 크레이트에 둡니다.
//! 정상적인 클라이언트가 지켜야 하는 약속이기도 합니다. 기본값은 플레이어 열 명이 초당 20번 입력을 보내도 닿지 않을 만큼 넉넉하게 정했습니다.

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerLimits {
    /// WebSocket 연결 절차(handshake)를 끝내야 하는 시간입니다.
    pub handshake_timeout: Duration,
    /// 동시에 열어 둘 수 있는 접속(연결 절차 중인 접속 포함)의 수입니다. 넘치는 접속은 받자마자 닫습니다.
    pub max_links: usize,
    /// IP 주소 하나가 동시에 열어 둘 수 있는 접속의 수입니다.
    pub max_links_per_ip: usize,
    /// 네트워크 스레드가 게임 스레드에 넘기고 아직 처리되지 않은 이벤트의 최대 수입니다 (모든 접속 합계).
    pub inbound_capacity: usize,
    /// 접속 하나가 1초에 보낼 수 있는 메시지의 수입니다. 넘으면 접속을 닫습니다.
    pub max_messages_per_sec: u32,
    /// 클라이언트가 보내는 메시지 하나의 최대 크기(바이트)입니다.
    pub max_message_bytes: usize,
    /// 접속 하나에 쌓아 둘 수 있는 송신 메시지의 수입니다 (20 Hz에서 약 3초). 가득 차면 접속을 끊습니다.
    pub outbox_capacity: usize,
    /// 소켓 쓰기 하나가 끝나야 하는 시간입니다.
    pub write_timeout: Duration,
    /// `GameServer::pump` 한 번이 처리하는 접속 이벤트의 최대 수입니다.
    pub events_per_pump: usize,
}

impl Default for ServerLimits {
    fn default() -> ServerLimits {
        ServerLimits {
            handshake_timeout: Duration::from_secs(5),
            max_links: 64,
            max_links_per_ip: 16,
            inbound_capacity: 1024,
            max_messages_per_sec: 100,
            max_message_bytes: 4096,
            outbox_capacity: 64,
            write_timeout: Duration::from_secs(3),
            events_per_pump: 512,
        }
    }
}
