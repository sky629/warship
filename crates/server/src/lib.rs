//! 게임 서버 라이브러리입니다. 방을 관리하고, 방마다 Tick을 실행하고, 진영마다 스냅샷을 보냅니다.
//!
//! 이 크레이트는 소켓을 열지 않고, 스레드를 만들지 않고, 현재 시각을 직접 읽지 않습니다.
//! 호출하는 쪽이 접속 방식과 현재 시각을 넘기고 `GameServer::pump`를 반복해서 호출합니다.
//! 그래서 전용 서버 실행 파일과 테스트가 같은 코드를 실행합니다.
//!
//! 접속 하나는 입장을 기다리는 상태, 방에 있는 상태, 서버가 이유를 알린 상태, 닫는 중인 상태 가운데 하나입니다.
//! 열려 있는 접속은 모두 기한까지 메시지를 보내야 하고, 보내지 않으면 서버가 닫습니다.
//! 방에 있던 접속이 닫히면 그 자리는 한동안 남고, 같은 표식으로 입장한 접속이 이어받습니다(`room`).
//!
//! 읽는 순서: `room`(방 하나와 자리), `pilot`(빈자리의 배를 조종하는 쪽), `mint`와 `random`(방 코드와 표식), `clock`(Tick 일정), 이 파일.

mod clock;
mod mint;
mod pilot;
mod random;
mod room;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use warship_protocol::{
    ClientMsg, LinkId, PROTOCOL_VERSION, PeerKey, PlayerName, RejectReason, RoomCode, SeatToken,
    ServerEvent, ServerLimits, ServerMsg, ServerTransport,
};
use warship_rules::{ShipId, Tuning};

use clock::TickClock;
use mint::Mint;
use room::Room;

pub use pilot::{Orders, Pilot};
pub use random::{PredictableRandom, RandomSource};

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
    /// 표식이 맞는 접속이 있던 자리로 돌아왔습니다.
    Returned {
        room: RoomCode,
        ship: ShipId,
    },
    /// 접속이 닫혀서 자리를 유지하기 시작했습니다.
    Held {
        room: RoomCode,
        ship: ShipId,
    },
    /// 자리가 끝났습니다. 스스로 나갔거나(`Leave`), 약속을 어겨서 닫혔거나, 유지하는 시간이 다 되었습니다.
    Left {
        room: RoomCode,
        ship: ShipId,
        members: usize,
    },
    Rejected {
        reason: RejectReason,
    },
    /// 접속 방식이나 서버가 접속을 닫았습니다.
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
            Notice::Returned { room, ship } => {
                write!(f, "ship {} returned to room {room}", ship.0)
            }
            Notice::Held { room, ship } => {
                write!(f, "ship {} of room {room} lost its link, seat held", ship.0)
            }
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

/// 접속 하나의 상태입니다. 열려 있는 접속은 모두 `deadline`까지 메시지를 보내야 하고, 보내지 않으면 서버가 닫습니다.
enum LinkState {
    /// 접속했지만 아직 `Join`을 보내지 않았습니다.
    AwaitingJoin { deadline: Instant },
    /// 방에 들어왔습니다. 메시지를 받을 때마다 `deadline`이 뒤로 밀립니다.
    InRoom { room: RoomCode, deadline: Instant },
    /// 서버가 이유를 보낸 접속입니다(입장 거절, 같은 표식의 새 접속에 자리를 내줌). 서버는 이유를 보낸 뒤 접속을 닫지 않고,
    /// 클라이언트가 이유를 읽고 닫기를 `deadline`까지 기다립니다. 서버가 이유를 보내자마자 닫으면,
    /// 브라우저는 닫힘을 그 메시지보다 먼저 알릴 수 있어서 이유가 화면에 나오지 않습니다. 닫히기 전에 도착하는 메시지는 무시합니다.
    Told { deadline: Instant },
    /// 서버가 닫기로 한 접속입니다. 닫히기 전에 도착하는 메시지는 무시하고 `Disconnected`를 기다립니다.
    Closing,
}

/// 서버가 접속을 닫을 때 그 접속의 자리를 어떻게 하는지입니다. 접속이 자리를 쓰고 있지 않으면 어느 쪽이든 같습니다.
#[derive(Clone, Copy)]
enum SeatFate {
    /// 메시지가 오지 않는 접속입니다. 끊긴 접속처럼 자리를 유지하고, 사람이 돌아올 수 있습니다.
    Hold,
    /// 약속을 어긴 접속입니다. 자리를 곧바로 끝냅니다.
    GiveUp,
}

pub struct GameServer {
    transport: Box<dyn ServerTransport>,
    tuning: Tuning,
    rooms: BTreeMap<RoomCode, Room>,
    links: BTreeMap<LinkId, LinkState>,
    /// 열려 있는 접속의 상대입니다. 접속 방식이 `Connected`로 알려 준 값이고, `Disconnected`까지 있습니다.
    peers: BTreeMap<LinkId, PeerKey>,
    /// 방 코드를 비우고 입장한 사람의 방 코드와, 새 자리의 표식을 만듭니다.
    mint: Mint,
    clock: TickClock,
    limits: ServerLimits,
    /// 접속이 닫힌 자리를 유지하는 Tick의 수입니다.
    hold_ticks: u32,
    /// 빈자리의 배를 조종하는 쪽입니다. 없으면 방에는 사람의 배만 있습니다.
    pilot: Option<Box<dyn Pilot>>,
}

impl GameServer {
    /// 수치가 규칙의 전제를 지키는지 한 번 검사하고, 어긋나면 오류를 돌려줍니다.
    /// `random`은 방 코드와 자리의 표식을 만드는 원천입니다. 실제 서버에는 예측할 수 없는 원천을 넘겨야 합니다.
    pub fn new(
        transport: Box<dyn ServerTransport>,
        tuning: Tuning,
        limits: ServerLimits,
        random: Box<dyn RandomSource>,
        now: Instant,
    ) -> Result<GameServer, InvalidTuning> {
        tuning.validate().map_err(InvalidTuning)?;
        // 유지하는 시간이 모자라지 않게 올림합니다.
        let hold_ticks = (limits.seat_hold.as_secs_f64() * f64::from(tuning.tick_hz)).ceil() as u32;
        Ok(GameServer {
            transport,
            clock: TickClock::new(tuning.tick_hz, now),
            tuning,
            rooms: BTreeMap::new(),
            links: BTreeMap::new(),
            peers: BTreeMap::new(),
            mint: Mint::new(random),
            limits,
            hold_ticks,
            pilot: None,
        })
    }

    /// 방의 빈자리를 `pilot`이 조종하는 배로 채우게 합니다. 방을 만들기 전에 불러야 합니다.
    pub fn with_pilot(mut self, pilot: Box<dyn Pilot>) -> GameServer {
        self.pilot = Some(pilot);
        self
    }

    /// 서버를 한 번 진행합니다. 기다리지 않습니다.
    /// 접속 방식에 도착한 이벤트를 `events_per_pump`개까지 처리하고, 기한까지 메시지를 보내지 않은 접속을 닫고,
    /// `now`까지 실행해야 하는 Tick을 실행한 뒤, 알릴 일을 돌려줍니다.
    /// 처리하지 못한 이벤트는 다음 호출이 처리합니다.
    pub fn pump(&mut self, now: Instant) -> Vec<Notice> {
        let mut notices = Vec::new();
        for _ in 0..self.limits.events_per_pump {
            let Some(event) = self.transport.poll() else {
                break;
            };
            self.handle_event(event, now, &mut notices);
        }
        self.close_silent_links(now, &mut notices);
        for _ in 0..self.clock.due(now) {
            self.tick(&mut notices);
        }
        notices
    }

    /// 다음 Tick까지 남은 시간입니다. 실행 파일은 이 시간만큼 쉰 뒤 다시 `pump`를 호출합니다.
    pub fn time_until_next_tick(&self, now: Instant) -> Duration {
        self.clock.time_until_next(now)
    }

    /// 모든 방을 Tick 하나만큼 진행하고 스냅샷을 보냅니다. 유지하는 시간이 다 된 자리를 알리고, 자리가 남지 않은 방을 없앱니다.
    fn tick(&mut self, notices: &mut Vec<Notice>) {
        let mut changed = Vec::new();
        for (code, room) in &mut self.rooms {
            let tick = room.tick(self.pilot.as_deref_mut());
            for (link, snapshot) in tick.snapshots {
                self.transport.send(link, &ServerMsg::Snapshot(snapshot));
            }
            let mut members = room.members() + tick.ended.len();
            for ship in tick.ended {
                members -= 1;
                notices.push(Notice::Left {
                    room: code.clone(),
                    ship,
                    members,
                });
                changed.push(code.clone());
            }
        }
        for code in changed {
            self.send_roster(&code);
        }
        self.remove_empty_rooms(notices);
    }

    /// 기한까지 메시지를 보내지 않은 접속을 닫습니다.
    /// 입장하지 않은 접속, 입장한 뒤 조용한 접속, 이유를 들은 뒤에도 닫지 않은 접속이 여기에 해당합니다.
    fn close_silent_links(&mut self, now: Instant, notices: &mut Vec<Notice>) {
        let (join_deadline, idle_timeout) = (self.limits.join_deadline, self.limits.idle_timeout);
        let silent = |(&link, state): (&LinkId, &LinkState)| match state {
            LinkState::AwaitingJoin { deadline } if now >= *deadline => {
                let reason = format!("no Join within {join_deadline:?} of connecting");
                Some((link, reason, SeatFate::GiveUp))
            }
            // 조용한 접속의 자리는 유지합니다. 창이 멈춘 사람이 돌아올 수 있습니다.
            LinkState::InRoom { deadline, .. } if now >= *deadline => {
                let reason = format!("no message for {idle_timeout:?}");
                Some((link, reason, SeatFate::Hold))
            }
            LinkState::Told { deadline } if now >= *deadline => {
                let reason =
                    format!("still open {join_deadline:?} after it was told why it cannot stay");
                Some((link, reason, SeatFate::GiveUp))
            }
            _ => None,
        };
        let silent: Vec<(LinkId, String, SeatFate)> =
            self.links.iter().filter_map(silent).collect();
        for (link, reason, fate) in silent {
            self.close_link(link, reason, fate, notices);
        }
    }

    fn handle_event(&mut self, event: ServerEvent, now: Instant, notices: &mut Vec<Notice>) {
        match event {
            ServerEvent::Connected(link, peer) => {
                self.peers.insert(link, peer);
                let deadline = now + self.limits.join_deadline;
                self.links
                    .insert(link, LinkState::AwaitingJoin { deadline });
            }
            ServerEvent::Message(link, msg) => self.handle_message(link, msg, now, notices),
            // 접속 방식이 한도를 넘은 접속을 닫았습니다. 이 접속의 `Disconnected`가 바로 이어서 오고, 자리는 그때 끊긴 접속처럼 유지합니다.
            ServerEvent::LimitReached(link, reason) => {
                notices.push(Notice::LinkClosed { link, reason });
            }
            ServerEvent::Disconnected(link) => {
                // 모르는 접속의 퇴장은 무시하므로 같은 퇴장을 두 번 받아도 안전합니다.
                if let Some(LinkState::InRoom { room, .. }) = self.links.remove(&link) {
                    self.hold_seat(&room, link, notices);
                }
                self.peers.remove(&link);
            }
        }
    }

    fn handle_message(
        &mut self,
        link: LinkId,
        msg: ClientMsg,
        now: Instant,
        notices: &mut Vec<Notice>,
    ) {
        let idle_timeout = self.limits.idle_timeout;
        let broken = match (self.links.get_mut(&link), msg) {
            (
                Some(LinkState::AwaitingJoin { .. }),
                ClientMsg::Join {
                    version,
                    room,
                    name,
                    token,
                },
            ) => {
                self.join(link, version, room.as_deref(), &name, token, now, notices);
                return;
            }
            (Some(LinkState::AwaitingJoin { .. }), _) => "sent a message before Join",
            (Some(LinkState::InRoom { .. }), ClientMsg::Join { .. }) => {
                "sent Join again after joining"
            }
            (Some(LinkState::InRoom { .. }), ClientMsg::Leave) => {
                self.leave(link, notices);
                return;
            }
            // 메시지는 접속이 살아 있다는 표시이므로, 버려지는 입력도 기한을 미룹니다.
            (Some(LinkState::InRoom { room, deadline }), msg) => {
                *deadline = now + idle_timeout;
                if let Some(room) = self.rooms.get_mut(room) {
                    match msg {
                        ClientMsg::Input(input) => room.take_input(link, input),
                        ClientMsg::Command(command) => room.take_command(link, command),
                        _ => {}
                    }
                }
                return;
            }
            (Some(LinkState::Told { .. } | LinkState::Closing) | None, _) => return,
        };
        // 입장하기 전의 다른 메시지와 두 번째 `Join`은 약속 위반입니다.
        self.close_link(link, broken.to_string(), SeatFate::GiveUp, notices);
    }

    /// 입장 요청을 검사하고, 받아들이면 입장 안내와 명단을 보냅니다. 거절하면 이유를 보내고 클라이언트가 닫기를 기다립니다.
    #[allow(clippy::too_many_arguments)]
    fn join(
        &mut self,
        link: LinkId,
        version: u32,
        raw_room: Option<&str>,
        raw_name: &str,
        token: Option<SeatToken>,
        now: Instant,
        notices: &mut Vec<Notice>,
    ) {
        let mut created = false;
        // 전송 형식의 글자가 `PlayerName`과 `RoomCode`가 되는 경계입니다.
        // 방 코드가 없으면 서버가 새 코드를 만들고, 방 코드가 있는데 그 방이 없으면 그 코드로 방을 만듭니다.
        let entered = (|| {
            if version != PROTOCOL_VERSION {
                return Err(RejectReason::VersionMismatch {
                    server: PROTOCOL_VERSION,
                });
            }
            let name = PlayerName::parse(raw_name).map_err(|_| RejectReason::BadName)?;
            let code = match raw_room {
                Some(raw) => RoomCode::parse(raw).map_err(|_| RejectReason::BadRoomCode)?,
                None => self.mint.room_code(&self.rooms),
            };
            if !self.rooms.contains_key(&code) && self.rooms.len() >= self.limits.max_rooms {
                return Err(RejectReason::ServerFull);
            }
            let room = self.rooms.entry(code.clone()).or_insert_with(|| {
                created = true;
                let fills_seats = self.pilot.is_some();
                Room::new(
                    code.clone(),
                    self.tuning.clone(),
                    fills_seats,
                    self.hold_ticks,
                )
            });
            let fresh = self.mint.token();
            let entered = room.enter(link, name, token, fresh)?;
            Ok((code, entered, room.members()))
        })();
        match entered {
            Ok((code, entered, members)) => {
                let deadline = now + self.limits.idle_timeout;
                let room = code.clone();
                self.links
                    .insert(link, LinkState::InRoom { room, deadline });
                let welcome = ServerMsg::Welcome(Box::new(entered.welcome));
                self.transport.send(link, &welcome);
                if let Some(old) = entered.replaced {
                    // 이전 접속은 닫지 않습니다. 이유를 읽은 클라이언트가 닫기를 기다립니다(`LinkState::Told`).
                    let deadline = now + self.limits.join_deadline;
                    self.links.insert(old, LinkState::Told { deadline });
                    self.transport.send(old, &ServerMsg::SeatTakenOver);
                }
                self.send_roster(&code);
                if created {
                    notices.push(Notice::RoomCreated { room: code.clone() });
                }
                let (room, ship) = (code, entered.ship);
                notices.push(if entered.returned {
                    Notice::Returned { room, ship }
                } else {
                    Notice::Joined {
                        room,
                        ship,
                        members,
                    }
                });
            }
            Err(reason) => {
                let deadline = now + self.limits.join_deadline;
                self.links.insert(link, LinkState::Told { deadline });
                self.transport
                    .send(link, &ServerMsg::JoinRejected { reason });
                notices.push(Notice::Rejected { reason });
                self.remove_empty_rooms(notices);
            }
        }
    }

    /// 방의 모든 접속에 지금의 명단을 보냅니다.
    fn send_roster(&mut self, code: &RoomCode) {
        let Some(room) = self.rooms.get(code) else {
            return;
        };
        let roster = ServerMsg::Roster(room.roster());
        for link in room.links() {
            self.transport.send(link, &roster);
        }
    }

    /// 플레이어가 스스로 나갑니다. 자리를 곧바로 끝내고 접속을 닫습니다.
    /// 나간 접속에는 더 보낼 것이 없으므로 닫아도 잃는 메시지가 없습니다.
    fn leave(&mut self, link: LinkId, notices: &mut Vec<Notice>) {
        if let Some(LinkState::InRoom { room, .. }) = self.links.insert(link, LinkState::Closing) {
            self.give_up_seat(&room, link, notices);
        }
        self.transport.close(link);
    }

    /// 서버가 `reason` 때문에 접속을 닫습니다. 방에 있던 접속의 자리는 `fate`가 정합니다.
    /// 닫힌 접속의 `Disconnected`는 다음 이벤트 처리에서 옵니다.
    fn close_link(
        &mut self,
        link: LinkId,
        reason: String,
        fate: SeatFate,
        notices: &mut Vec<Notice>,
    ) {
        notices.push(Notice::LinkClosed { link, reason });
        if let Some(LinkState::InRoom { room, .. }) = self.links.insert(link, LinkState::Closing) {
            match fate {
                SeatFate::Hold => self.hold_seat(&room, link, notices),
                SeatFate::GiveUp => self.give_up_seat(&room, link, notices),
            }
        }
        self.transport.close(link);
    }

    /// `link`가 닫혔으니 자리를 유지합니다. 자리는 방에 남고, 시간이 다 되면 끝납니다.
    /// 이미 다른 접속이 이어받은 자리이면 아무 일도 하지 않습니다.
    /// 같은 상대가 유지시키는 자리가 한도(`max_held_seats_per_ip`)를 넘거나, 서버 전체의 끊긴 자리가 한도(`max_held_seats`)를 넘으면
    /// 이 자리는 유지하지 않고 곧바로 끝냅니다. 접속이 없는 자리는 접속 수의 한도에 들지 않으므로, 이 두 한도가 접속자 없는 방이 쌓이는 것을 막습니다.
    fn hold_seat(&mut self, code: &RoomCode, link: LinkId, notices: &mut Vec<Notice>) {
        // 열린 접속은 모두 `Connected`로 상대가 알려졌으므로, 상대를 모르는 접속은 없습니다.
        let Some(&peer) = self.peers.get(&link) else {
            return self.give_up_seat(code, link, notices);
        };
        let held = self.rooms.get_mut(code);
        let Some(ship) = held.and_then(|room| room.link_closed(link, peer)) else {
            return;
        };
        let held_from: usize = self.rooms.values().map(|room| room.held_from(peer)).sum();
        let held_total: usize = self.rooms.values().map(Room::held_count).sum();
        if held_from > self.limits.max_held_seats_per_ip || held_total > self.limits.max_held_seats
        {
            return self.end_seat(code, ship, notices);
        }
        notices.push(Notice::Held {
            room: code.clone(),
            ship,
        });
        self.send_roster(code);
    }

    /// `link`의 자리를 곧바로 끝냅니다.
    fn give_up_seat(&mut self, code: &RoomCode, link: LinkId, notices: &mut Vec<Notice>) {
        let ship = self.rooms.get(code).and_then(|room| room.ship_of(link));
        if let Some(ship) = ship {
            self.end_seat(code, ship, notices);
        }
    }

    /// `ship`의 자리를 끝냅니다. 남은 사람에게 새 명단을 보내고, 마지막 자리였으면 방도 없앱니다.
    fn end_seat(&mut self, code: &RoomCode, ship: ShipId, notices: &mut Vec<Notice>) {
        if let Some(room) = self.rooms.get_mut(code) {
            room.leave(ship);
            notices.push(Notice::Left {
                room: code.clone(),
                ship,
                members: room.members(),
            });
        }
        self.send_roster(code);
        self.remove_empty_rooms(notices);
    }

    /// 자리가 하나도 남지 않은 방을 없앱니다.
    fn remove_empty_rooms(&mut self, notices: &mut Vec<Notice>) {
        let empty = |(code, room): (&RoomCode, &Room)| (room.members() == 0).then(|| code.clone());
        let empty: Vec<RoomCode> = self.rooms.iter().filter_map(empty).collect();
        for room in empty {
            self.rooms.remove(&room);
            notices.push(Notice::RoomRemoved { room });
        }
    }
}
