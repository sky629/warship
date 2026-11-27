//! 접속 하나의 상태입니다. 접속 방식의 이벤트를 받아서 상태를 바꾸고, 서버에 보낼 메시지를 돌려줍니다.
//!
//! 이 타입은 접속 방식을 직접 호출하지 않습니다. 그래서 Bevy와 소켓 없이 테스트합니다.
//!
//! 방 안에 있다가 접속이 끊기면 `Phase::Reconnecting`이 됩니다. 호출하는 쪽은 `should_reconnect`가 참을 돌려줄 때 새 접속을 엽니다.
//! 새 접속은 입장 안내로 받은 자리의 표식을 입장 요청에 실어서, 서버가 유지하고 있는 같은 배로 돌아갑니다.

use warship_protocol::{
    ClientEvent, ClientMsg, CommandMsg, InputMsg, PROTOCOL_VERSION, RejectReason, SeatToken,
    ServerMsg, Welcome, WireCommand, WireMember, WireVec2,
};

use crate::interp::{Frame, History};

/// 방 안에 있는 동안 이 시간(초)마다 `Ping`을 보냅니다. 서버가 조용한 접속을 닫는 시간(30초)보다 짧습니다.
const PING_SECS: f64 = 5.0;
/// 접속이 끊긴 뒤 이 시간(초)마다 새 접속을 엽니다.
const RETRY_SECS: f64 = 2.0;
/// 접속이 끊긴 뒤 이 시간(초)이 지나면 돌아가기를 그만둡니다. 서버가 자리를 유지하는 시간과 같습니다.
const RETURN_SECS: f64 = 60.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// 접속이 열리기를 기다립니다.
    Connecting,
    /// 입장 요청을 보내고 답을 기다립니다.
    Joining,
    /// 방 안에 있습니다.
    Playing,
    /// 방 안에 있다가 접속이 끊겼습니다. 새 접속으로 같은 자리에 돌아가려고 합니다.
    Reconnecting,
    /// 접속이 끝났습니다. 값은 화면에 표시하는 이유입니다.
    Ended(String),
}

pub struct Session {
    /// 들어갈 방의 코드입니다. 없으면 서버가 새 방을 만들고, 입장한 뒤에는 그 방의 코드가 됩니다.
    room: Option<String>,
    name: String,
    /// 입장 안내로 받은 자리의 표식입니다. 다시 입장할 때 보냅니다.
    token: Option<SeatToken>,
    /// 이 실행에서 입장 안내를 받은 적이 있는지입니다. 받은 뒤에 끊긴 접속만 다시 접속합니다.
    joined: bool,
    phase: Phase,
    welcome: Option<Welcome>,
    roster: Vec<WireMember>,
    /// 접속이 끊긴 시각입니다. `Phase::Reconnecting`일 때만 뜻이 있습니다.
    lost_at: f64,
    /// 이 시각부터 새 접속을 엽니다. 새 접속을 연 뒤에는 그 접속이 끊길 때까지 무한대입니다.
    retry_at: f64,
    /// 마지막으로 `Ping`을 보낸 시각입니다.
    last_ping: f64,
    history: History,
    /// 마지막으로 보낸 입력의 순서 번호입니다.
    last_seq: u32,
    /// 마지막으로 보낸 명령의 순서 번호입니다.
    last_command_seq: u32,
}

impl Session {
    /// `room`이 없으면 서버가 만든 새 방에 들어갑니다. `token`은 기기가 기억하는 그 방의 자리의 표식입니다.
    pub fn new(room: Option<&str>, name: &str, token: Option<SeatToken>) -> Session {
        Session {
            room: room.map(str::to_string),
            name: name.to_string(),
            token,
            joined: false,
            phase: Phase::Connecting,
            welcome: None,
            roster: Vec::new(),
            lost_at: 0.0,
            retry_at: f64::INFINITY,
            last_ping: 0.0,
            history: History::default(),
            last_seq: 0,
            last_command_seq: 0,
        }
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// 입장한 뒤에만 있습니다.
    pub fn welcome(&self) -> Option<&Welcome> {
        self.welcome.as_ref()
    }

    /// 스스로 방을 나가는 메시지입니다. 방 안에 있을 때만 만들어지고, 접속 상태는 끝납니다.
    pub fn leave(&mut self) -> Option<ClientMsg> {
        let playing = self.phase == Phase::Playing;
        self.phase = Phase::Ended("You left the room".to_string());
        playing.then_some(ClientMsg::Leave)
    }

    /// 방에 있는 사람의 명단입니다. 입장하기 전에는 비어 있습니다.
    pub fn roster(&self) -> &[WireMember] {
        &self.roster
    }

    /// 접속 방식의 이벤트 하나를 처리하고, 서버에 보낼 메시지가 있으면 돌려줍니다. `now`는 초 단위의 현재 시각입니다.
    pub fn handle(&mut self, event: ClientEvent, now: f64) -> Option<ClientMsg> {
        if matches!(self.phase, Phase::Ended(_)) {
            return None;
        }
        match event {
            ClientEvent::Connected => {
                if self.phase == Phase::Connecting {
                    self.phase = Phase::Joining;
                }
                Some(ClientMsg::Join {
                    version: PROTOCOL_VERSION,
                    room: self.room.clone(),
                    name: self.name.clone(),
                    token: self.token,
                })
            }
            ClientEvent::Message(ServerMsg::Welcome(welcome)) => {
                // 다시 입장하면 같은 방으로 갑니다. 서버는 새 접속의 순서 번호를 1부터 받습니다.
                self.room = Some(welcome.room.clone());
                self.token = Some(welcome.token);
                self.joined = true;
                self.welcome = Some(*welcome);
                self.history = History::default();
                self.last_seq = 0;
                self.last_command_seq = 0;
                self.last_ping = now;
                self.phase = Phase::Playing;
                None
            }
            ClientEvent::Message(ServerMsg::JoinRejected { reason }) => {
                self.phase = Phase::Ended(reject_text(reason));
                None
            }
            ClientEvent::Message(ServerMsg::Roster(roster)) => {
                self.roster = roster;
                None
            }
            ClientEvent::Message(ServerMsg::SeatTakenOver) => {
                self.joined = false;
                self.phase = Phase::Ended("Another window took over this ship".to_string());
                None
            }
            ClientEvent::Message(ServerMsg::Snapshot(snapshot)) => {
                self.history.push(now, *snapshot);
                None
            }
            // 방에 들어간 적이 있으면 돌아갈 자리가 있습니다. 없으면 입장하기 전에 끊긴 것이므로 끝냅니다.
            ClientEvent::Disconnected { reason } => {
                if !self.joined {
                    self.phase = Phase::Ended(reason);
                    return None;
                }
                if self.phase != Phase::Reconnecting {
                    self.lost_at = now;
                    self.phase = Phase::Reconnecting;
                }
                self.retry_at = now + RETRY_SECS;
                None
            }
        }
    }

    /// 지금 새 접속을 열어야 하는지입니다. 참을 돌려주면 호출하는 쪽이 접속을 열고, 그 접속의 이벤트를 `handle`에 넘깁니다.
    /// 접속이 끊긴 뒤 `RETURN_SECS`가 지나면 돌아가기를 그만두고 끝냅니다.
    pub fn should_reconnect(&mut self, now: f64) -> bool {
        if self.phase != Phase::Reconnecting {
            return false;
        }
        if now - self.lost_at > RETURN_SECS {
            self.phase = Phase::Ended("Lost the connection to the server".to_string());
            return false;
        }
        let due = now >= self.retry_at;
        if due {
            self.retry_at = f64::INFINITY;
        }
        due
    }

    /// 접속이 살아 있다는 표시입니다. 방 안에 있는 동안 `PING_SECS`마다 한 번 만들어집니다.
    pub fn ping(&mut self, now: f64) -> Option<ClientMsg> {
        if self.phase != Phase::Playing || now - self.last_ping < PING_SECS {
            return None;
        }
        self.last_ping = now;
        Some(ClientMsg::Ping)
    }

    /// 배를 이 지점으로 보내는 입력입니다. 방 안에 있을 때만 만들어집니다.
    pub fn sail_to(&mut self, x: f32, y: f32) -> Option<ClientMsg> {
        if self.phase != Phase::Playing {
            return None;
        }
        self.last_seq += 1;
        Some(ClientMsg::Input(InputMsg {
            seq: self.last_seq,
            destination: Some(WireVec2::new(x, y)),
        }))
    }

    /// 구매, 판매, 선체 교체, 스킬의 명령입니다. 방 안에 있을 때만 만들어집니다.
    pub fn command(&mut self, command: WireCommand) -> Option<ClientMsg> {
        if self.phase != Phase::Playing {
            return None;
        }
        self.last_command_seq += 1;
        Some(ClientMsg::Command(CommandMsg {
            seq: self.last_command_seq,
            command,
        }))
    }

    /// 지금 화면에 그릴 방의 모습입니다. 입장하지 않았거나 스냅샷을 아직 받지 못했으면 없습니다.
    pub fn frame(&self, now: f64) -> Option<Frame> {
        self.history.frame(self.welcome.as_ref()?, now)
    }
}

fn reject_text(reason: RejectReason) -> String {
    match reason {
        RejectReason::RoomFull { capacity } => format!("The room is full ({capacity} ships)"),
        RejectReason::BadRoomCode => "The room code may use a-z, 0-9, - and _ only".to_string(),
        RejectReason::BadName => {
            "The name may use up to 12 letters, digits, spaces, - and _".to_string()
        }
        RejectReason::ServerFull => "The server cannot open another room".to_string(),
        RejectReason::VersionMismatch { server } => {
            format!("The server speaks protocol {server}, this client {PROTOCOL_VERSION}")
        }
    }
}
