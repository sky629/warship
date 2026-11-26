//! 서버를 소켓 없이 실행해서 입장, 진영별 스냅샷, 입력, 퇴장을 확인합니다.
//!
//! 접속 방식은 같은 프로세스 안에서 메시지를 옮기는 `loopback`이고, 시각은 테스트가 정합니다.

use std::time::{Duration, Instant};

use warship_protocol::{
    ClientEvent, ClientMsg, ClientTransport, InputMsg, LoopbackClient, LoopbackConnector,
    PROTOCOL_VERSION, RejectReason, ServerLimits, ServerMsg, Snapshot, Welcome, WireTeam, WireVec2,
    loopback,
};
use warship_rules::{Tuning, Vec2};
use warship_server::{GameServer, Notice};

struct Bench {
    server: GameServer,
    connector: LoopbackConnector,
    now: Instant,
    notices: Vec<Notice>,
}

impl Bench {
    fn new(tuning: Tuning) -> Bench {
        let (transport, connector) = loopback();
        let now = Instant::now();
        let server = GameServer::new(Box::new(transport), tuning, ServerLimits::default(), now);
        Bench {
            server: server.expect("the tuning is valid"),
            connector,
            now,
            notices: Vec::new(),
        }
    }

    /// 접속을 열고 `room`에 입장 요청을 보냅니다.
    fn join(&mut self, room: &str) -> Player {
        let mut link = self.connector.connect();
        assert!(matches!(link.poll(), Some(ClientEvent::Connected)));
        link.send(&ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: room.to_string(),
        });
        Player { link }
    }

    /// 서버가 Tick을 `ticks`번 실행하게 합니다. 1초는 20Tick입니다.
    fn run_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.now += Duration::from_millis(50);
            let notices = self.server.pump(self.now);
            self.notices.extend(notices);
        }
    }
}

struct Player {
    link: LoopbackClient,
}

impl Player {
    fn messages(&mut self) -> Vec<ServerMsg> {
        let mut messages = Vec::new();
        while let Some(event) = self.link.poll() {
            if let ClientEvent::Message(msg) = event {
                messages.push(msg);
            }
        }
        messages
    }

    fn welcome(&mut self) -> Welcome {
        let found = self.messages().into_iter().find_map(|msg| match msg {
            ServerMsg::Welcome(welcome) => Some(*welcome),
            _ => None,
        });
        found.expect("the server sent a welcome")
    }

    /// 받은 스냅샷 가운데 마지막 것입니다.
    fn last_snapshot(&mut self) -> Snapshot {
        let found = self.messages().into_iter().rev().find_map(|msg| match msg {
            ServerMsg::Snapshot(snapshot) => Some(*snapshot),
            _ => None,
        });
        found.expect("the server sent a snapshot")
    }

    fn sail_to(&mut self, seq: u32, x: f32, y: f32) {
        self.link.send(&ClientMsg::Input(InputMsg {
            seq,
            destination: Some(WireVec2::new(x, y)),
        }));
    }
}

/// 두 진영의 시작 자리가 서로 보이는 거리(5타일)에 있는 수치입니다. 타워는 없고, 배는 무기 없이 시작합니다.
fn close_spawns() -> Tuning {
    let mut tuning = Tuning::default();
    tuning.map.towers.clear();
    tuning.ship.start_weapons.clear();
    tuning.map.spawns = [Vec2::new(10.0, 10.0), Vec2::new(15.0, 10.0)];
    tuning
}

#[test]
fn a_join_is_answered_with_a_welcome_and_then_a_snapshot_every_tick() {
    let mut bench = Bench::new(Tuning::default());
    let mut kang = bench.join("Test");

    bench.run_ticks(3);

    let messages = kang.messages();
    let ServerMsg::Welcome(welcome) = &messages[0] else {
        panic!("the first message is the welcome");
    };
    assert_eq!(welcome.room, "test");
    assert_eq!(welcome.team, WireTeam::Red);
    assert_eq!(welcome.tick_hz, 20);
    let ticks: Vec<u64> = messages[1..]
        .iter()
        .map(|msg| match msg {
            ServerMsg::Snapshot(snapshot) => snapshot.tick,
            other => panic!("expected a snapshot, got {other:?}"),
        })
        .collect();
    assert_eq!(ticks, [1, 2, 3]);
    assert_eq!(
        bench.notices[..2],
        [
            Notice::RoomCreated {
                room: "test".parse_room()
            },
            Notice::Joined {
                room: "test".parse_room(),
                ship: warship_rules::ShipId(1),
                members: 1
            }
        ]
    );
}

trait ParseRoom {
    fn parse_room(&self) -> warship_protocol::RoomCode;
}

impl ParseRoom for str {
    fn parse_room(&self) -> warship_protocol::RoomCode {
        warship_protocol::RoomCode::parse(self).expect("a valid room code")
    }
}

#[test]
fn two_players_in_a_room_are_on_opposite_teams_and_do_not_see_a_far_enemy() {
    let mut bench = Bench::new(Tuning::default());
    let mut red = bench.join("test");
    let mut blue = bench.join("test");
    bench.run_ticks(1);

    let (red_ship, blue_ship) = (red.welcome(), blue.welcome());
    assert_eq!(
        (red_ship.team, blue_ship.team),
        (WireTeam::Red, WireTeam::Blue)
    );

    // 기본 맵의 시작 자리는 52타일 떨어져 있고 시야는 9타일입니다.
    bench.run_ticks(1);
    let ids = |snapshot: Snapshot| {
        snapshot
            .ships
            .iter()
            .map(|ship| ship.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(red.last_snapshot()), [red_ship.ship]);
    assert_eq!(ids(blue.last_snapshot()), [blue_ship.ship]);
}

#[test]
fn an_enemy_inside_the_vision_is_in_the_snapshot() {
    let mut bench = Bench::new(close_spawns());
    let mut red = bench.join("test");
    let _blue = bench.join("test");

    bench.run_ticks(2);

    assert_eq!(red.last_snapshot().ships.len(), 2);
}

#[test]
fn an_input_moves_the_ship_and_an_old_or_broken_input_is_dropped() {
    let mut bench = Bench::new(close_spawns());
    let mut red = bench.join("test");
    bench.run_ticks(1);

    red.sail_to(2, 20.0, 10.0);
    bench.run_ticks(20);
    let ship = red.last_snapshot().ships[0].clone();
    assert!(
        (ship.x - 14.0).abs() < 1e-3,
        "4 tiles in a second, got {}",
        ship.x
    );
    assert_eq!(ship.last_input_seq, 2);

    // 순서 번호가 크지 않은 입력과 좌표가 유한하지 않은 입력은 목적지를 바꾸지 못합니다.
    red.sail_to(1, 0.0, 10.0);
    red.sail_to(3, f32::NAN, 10.0);
    bench.run_ticks(20);
    let ship = red.last_snapshot().ships[0].clone();
    assert!(
        (ship.x - 18.0).abs() < 1e-3,
        "still sailing east, got {}",
        ship.x
    );
    assert_eq!(ship.last_input_seq, 2);
}

#[test]
fn joins_that_break_the_rules_are_rejected_with_the_reason() {
    let tuning = Tuning {
        team_size: 1,
        ..Tuning::default()
    };
    let mut bench = Bench::new(tuning);

    let mut old = bench.connector.connect();
    old.poll();
    old.send(&ClientMsg::Join {
        version: PROTOCOL_VERSION + 1,
        room: "test".to_string(),
    });
    let mut bad_code = bench.join("no spaces");
    let _first = bench.join("test");
    let _second = bench.join("test");
    let mut third = bench.join("test");
    bench.run_ticks(1);

    let reason = |messages: Vec<ServerMsg>| match messages.as_slice() {
        [ServerMsg::JoinRejected { reason }] => *reason,
        other => panic!("expected one rejection, got {other:?}"),
    };
    let mut old = Player { link: old };
    assert_eq!(
        reason(old.messages()),
        RejectReason::VersionMismatch {
            server: PROTOCOL_VERSION
        }
    );
    assert_eq!(reason(bad_code.messages()), RejectReason::BadRoomCode);
    assert_eq!(
        reason(third.messages()),
        RejectReason::RoomFull { capacity: 2 }
    );
}

#[test]
fn a_player_who_leaves_or_disconnects_is_removed_and_an_empty_room_is_removed() {
    let mut bench = Bench::new(close_spawns());
    let mut red = bench.join("test");
    let mut blue = bench.join("test");
    bench.run_ticks(2);
    assert_eq!(red.last_snapshot().ships.len(), 2);

    blue.link.send(&ClientMsg::Leave);
    bench.run_ticks(2);
    assert_eq!(red.last_snapshot().ships.len(), 1);

    drop(red);
    bench.run_ticks(1);
    let room = "test".parse_room();
    assert_eq!(
        bench.notices[bench.notices.len() - 2..],
        [
            Notice::Left {
                room: room.clone(),
                ship: warship_rules::ShipId(1),
                members: 0
            },
            Notice::RoomRemoved { room }
        ]
    );
}

#[test]
fn rooms_do_not_see_each_other() {
    let mut bench = Bench::new(close_spawns());
    let mut here = bench.join("here");
    let mut there = bench.join("there");

    bench.run_ticks(2);

    assert_eq!(here.welcome().team, WireTeam::Red);
    assert_eq!(there.welcome().team, WireTeam::Red);
    bench.run_ticks(1);
    assert_eq!(here.last_snapshot().ships.len(), 1);
}
