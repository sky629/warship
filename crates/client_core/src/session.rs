//! 접속 하나의 상태입니다. 접속 방식의 이벤트를 받아서 상태를 바꾸고, 서버에 보낼 메시지를 돌려줍니다.
//!
//! 이 타입은 접속 방식을 직접 호출하지 않습니다. 그래서 Bevy와 소켓 없이 테스트합니다.

use warship_protocol::{
    ClientEvent, ClientMsg, CommandMsg, InputMsg, PROTOCOL_VERSION, RejectReason, ServerMsg,
    Welcome, WireCommand, WireVec2,
};

use crate::interp::{Frame, History};

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// 접속이 열리기를 기다립니다.
    Connecting,
    /// 입장 요청을 보내고 답을 기다립니다.
    Joining,
    /// 방 안에 있습니다.
    Playing,
    /// 접속이 끝났습니다. 값은 화면에 표시하는 이유입니다.
    Ended(String),
}

pub struct Session {
    room: String,
    phase: Phase,
    welcome: Option<Welcome>,
    history: History,
    /// 마지막으로 보낸 입력의 순서 번호입니다.
    last_seq: u32,
    /// 마지막으로 보낸 명령의 순서 번호입니다.
    last_command_seq: u32,
}

impl Session {
    pub fn new(room: &str) -> Session {
        Session {
            room: room.to_string(),
            phase: Phase::Connecting,
            welcome: None,
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

    /// 접속 방식의 이벤트 하나를 처리하고, 서버에 보낼 메시지가 있으면 돌려줍니다. `now`는 초 단위의 현재 시각입니다.
    pub fn handle(&mut self, event: ClientEvent, now: f64) -> Option<ClientMsg> {
        if matches!(self.phase, Phase::Ended(_)) {
            return None;
        }
        match event {
            ClientEvent::Connected => {
                self.phase = Phase::Joining;
                Some(ClientMsg::Join {
                    version: PROTOCOL_VERSION,
                    room: self.room.clone(),
                })
            }
            ClientEvent::Message(ServerMsg::Welcome(welcome)) => {
                self.welcome = Some(*welcome);
                self.phase = Phase::Playing;
                None
            }
            ClientEvent::Message(ServerMsg::JoinRejected { reason }) => {
                self.phase = Phase::Ended(reject_text(reason));
                None
            }
            ClientEvent::Message(ServerMsg::Snapshot(snapshot)) => {
                self.history.push(now, *snapshot);
                None
            }
            ClientEvent::Disconnected { reason } => {
                self.phase = Phase::Ended(reason);
                None
            }
        }
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
        RejectReason::VersionMismatch { server } => {
            format!("The server speaks protocol {server}, this client {PROTOCOL_VERSION}")
        }
    }
}
