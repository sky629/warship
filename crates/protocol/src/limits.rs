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
    /// IP 주소 하나가 유지시킬 수 있는 끊긴 자리의 수입니다. 넘치는 자리는 유지하지 않고 곧바로 내놓습니다.
    pub max_held_seats_per_ip: usize,
    /// 서버 전체에서 유지할 수 있는 끊긴 자리의 수입니다.
    pub max_held_seats: usize,
    /// 한꺼번에 둘 수 있는 방의 수입니다. 넘치는 방을 만들려는 입장은 거절합니다.
    pub max_rooms: usize,
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
    /// 접속한 뒤 입장해야 하는 시간입니다. 입장하지 않고 접속 자리만 차지하는 상대를 닫습니다.
    /// 입장이 거절된 접속도 이 시간이 지나면 닫습니다.
    pub join_deadline: Duration,
    /// 입장한 접속이 아무 메시지도 보내지 않고 지날 수 있는 시간입니다. 넘으면 접속을 닫고 자리는 유지합니다.
    /// 클라이언트는 이 시간보다 자주 `Ping`을 보내므로 살아 있는 클라이언트는 닿지 않습니다.
    pub idle_timeout: Duration,
    /// 접속이 닫힌 자리를 유지하는 시간입니다. 이 시간 안에 같은 표식으로 입장하면 같은 배를 다시 조종합니다.
    pub seat_hold: Duration,
}

impl Default for ServerLimits {
    fn default() -> ServerLimits {
        ServerLimits {
            handshake_timeout: Duration::from_secs(5),
            max_links: 64,
            max_links_per_ip: 16,
            max_held_seats_per_ip: 8,
            max_held_seats: 128,
            max_rooms: 256,
            inbound_capacity: 1024,
            max_messages_per_sec: 100,
            max_message_bytes: 4096,
            outbox_capacity: 64,
            write_timeout: Duration::from_secs(3),
            events_per_pump: 512,
            join_deadline: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(30),
            seat_hold: Duration::from_secs(60),
        }
    }
}
