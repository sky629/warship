//! 게임 서버 라이브러리입니다. 방을 관리하고, 방마다 Tick을 실행하고, 진영마다 스냅샷을 보냅니다.
//!
//! 이 크레이트는 소켓을 열지 않고, 스레드를 만들지 않고, 현재 시각을 직접 읽지 않습니다.
//! 호출하는 쪽이 접속 방식과 현재 시각을 넘기고 `GameServer::pump`를 반복해서 호출합니다.
//! 그래서 전용 서버 실행 파일과 테스트가 같은 코드를 실행합니다.
//!
//! 읽는 순서: `room`(방 하나), `clock`(Tick 일정), 이 파일.

mod clock;
mod room;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use warship_protocol::{
    ClientMsg, LinkId, PROTOCOL_VERSION, RejectReason, RoomCode, ServerEvent, ServerLimits,
    ServerMsg, ServerTransport,
};
use warship_rules::{ShipId, Tuning};

use clock::TickClock;
use room::Room;

/// 서버에서 일어난 일 가운데 운영자에게 알릴 것입니다. 라이브러리는 직접 로그를 쓰지 않고, 이 값을 돌려줍니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// 방이 만들어졌습니다. 그 방의 첫 `Joined` 바로 앞에 옵니다.
    RoomCreated {
        room: RoomCode,
    },
    /// 마지막 플레이어가 나가서 방이 없어졌습니다. 마지막 `Left` 바로 뒤에 옵니다.
    RoomRemoved {
        room: RoomCode,
    },
    Joined {
        room: RoomCode,
        ship: ShipId,
        members: usize,
    },
    /// 플레이어가 방을 떠났습니다. 스스로 나갔거나(`Leave`) 접속이 끊겼습니다.
    Left {
        room: RoomCode,
        ship: ShipId,
        members: usize,
    },
    Rejected {
        reason: RejectReason,
    },
    /// 접속 방식이 한도를 넘은 접속을 닫았습니다.
    LinkClosed {
        link: LinkId,
        reason: String,
    },
}

impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Notice::RoomCreated { room } => write!(f, "room {room} created"),
            Notice::RoomRemoved { room } => write!(f, "room {room} removed"),
            Notice::Joined {
                room,
                ship,
                members,
            } => write!(f, "ship {} joined room {room} ({members} members)", ship.0),
            Notice::Left {
                room,
                ship,
                members,
            } => write!(f, "ship {} left room {room} ({members} members)", ship.0),
            Notice::Rejected { reason } => write!(f, "join rejected: {reason:?}"),
            Notice::LinkClosed { link, reason } => write!(f, "link {} closed: {reason}", link.0),
        }
    }
}

/// 서버를 만들지 못한 이유입니다. 값은 `Tuning::validate`가 거절한 첫 항목을 설명하는 문장입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidTuning(pub String);

impl fmt::Display for InvalidTuning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid tuning: {}", self.0)
    }
}

impl Error for InvalidTuning {}

pub struct GameServer {
    transport: Box<dyn ServerTransport>,
    tuning: Tuning,
    rooms: BTreeMap<RoomCode, Room>,
    /// 입장한 접속이 들어가 있는 방입니다. 입장하지 않은 접속은 없습니다.
    seats: BTreeMap<LinkId, RoomCode>,
    clock: TickClock,
    events_per_pump: usize,
}

impl GameServer {
    /// 수치가 규칙의 전제를 지키는지 한 번 검사하고, 어긋나면 오류를 돌려줍니다.
    pub fn new(
        transport: Box<dyn ServerTransport>,
        tuning: Tuning,
        limits: ServerLimits,
        now: Instant,
    ) -> Result<GameServer, InvalidTuning> {
        tuning.validate().map_err(InvalidTuning)?;
        Ok(GameServer {
            transport,
            clock: TickClock::new(tuning.tick_hz, now),
            tuning,
            rooms: BTreeMap::new(),
            seats: BTreeMap::new(),
            events_per_pump: limits.events_per_pump,
        })
    }

    /// 서버를 한 번 진행합니다. 기다리지 않습니다.
    /// 접속 방식에 도착한 이벤트를 `events_per_pump`개까지 처리하고, `now`까지 실행해야 하는 Tick을 실행한 뒤, 알릴 일을 돌려줍니다.
    /// 처리하지 못한 이벤트는 다음 호출이 처리합니다.
    pub fn pump(&mut self, now: Instant) -> Vec<Notice> {
        let mut notices = Vec::new();
        for _ in 0..self.events_per_pump {
            let Some(event) = self.transport.poll() else {
                break;
            };
            self.handle_event(event, &mut notices);
        }
        for _ in 0..self.clock.due(now) {
            for room in self.rooms.values_mut() {
                for (link, snapshot) in room.tick() {
                    self.transport.send(link, &ServerMsg::Snapshot(snapshot));
                }
            }
        }
        notices
    }

    /// 다음 Tick까지 남은 시간입니다. 실행 파일은 이 시간만큼 쉰 뒤 다시 `pump`를 호출합니다.
    pub fn time_until_next_tick(&self, now: Instant) -> Duration {
        self.clock.time_until_next(now)
    }

    fn handle_event(&mut self, event: ServerEvent, notices: &mut Vec<Notice>) {
        match event {
            ServerEvent::Connected(..) => {}
            ServerEvent::Message(link, ClientMsg::Join { version, room }) => {
                self.join(link, version, &room, notices);
            }
            ServerEvent::Message(link, ClientMsg::Input(input)) => {
                let room = self
                    .seats
                    .get(&link)
                    .and_then(|code| self.rooms.get_mut(code));
                if let Some(room) = room {
                    room.take_input(link, input);
                }
            }
            ServerEvent::Message(link, ClientMsg::Leave) | ServerEvent::Disconnected(link) => {
                self.leave(link, notices);
            }
            ServerEvent::LimitReached(link, reason) => {
                notices.push(Notice::LinkClosed { link, reason });
            }
        }
    }

    /// 입장 요청을 검사하고, 받아들이면 입장 안내를 보냅니다. 이미 입장한 접속의 입장 요청은 무시합니다.
    fn join(&mut self, link: LinkId, version: u32, room: &str, notices: &mut Vec<Notice>) {
        if self.seats.contains_key(&link) {
            return;
        }
        let mut created = false;
        let entered = (|| {
            if version != PROTOCOL_VERSION {
                return Err(RejectReason::VersionMismatch {
                    server: PROTOCOL_VERSION,
                });
            }
            let code = RoomCode::parse(room).map_err(|_| RejectReason::BadRoomCode)?;
            let room = self.rooms.entry(code.clone()).or_insert_with(|| {
                created = true;
                Room::new(self.tuning.clone())
            });
            let (ship, welcome) = room.enter(link, &code)?;
            Ok((code, ship, welcome, room.members()))
        })();
        match entered {
            Ok((code, ship, welcome, members)) => {
                if created {
                    notices.push(Notice::RoomCreated { room: code.clone() });
                }
                self.seats.insert(link, code.clone());
                self.transport
                    .send(link, &ServerMsg::Welcome(Box::new(welcome)));
                notices.push(Notice::Joined {
                    room: code,
                    ship,
                    members,
                });
            }
            Err(reason) => {
                self.transport
                    .send(link, &ServerMsg::JoinRejected { reason });
                notices.push(Notice::Rejected { reason });
            }
        }
    }

    /// 접속의 배를 방에서 빼고, 빈 방은 없앱니다. 입장하지 않은 접속이면 아무 일도 하지 않습니다.
    fn leave(&mut self, link: LinkId, notices: &mut Vec<Notice>) {
        let Some(code) = self.seats.remove(&link) else {
            return;
        };
        let Some(room) = self.rooms.get_mut(&code) else {
            return;
        };
        if let Some(ship) = room.exit(link) {
            notices.push(Notice::Left {
                room: code.clone(),
                ship,
                members: room.members(),
            });
        }
        if room.members() == 0 {
            self.rooms.remove(&code);
            notices.push(Notice::RoomRemoved { room: code });
        }
    }
}
