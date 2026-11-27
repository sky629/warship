//! 서버를 소켓 없이 실행해서 입장, 진영별 스냅샷, 입력, 퇴장을 확인합니다.
//!
//! 접속 방식은 같은 프로세스 안에서 메시지를 옮기는 `loopback`이고, 시각은 테스트가 정합니다.

use std::time::{Duration, Instant};

use warship_protocol::{
    ClientEvent, ClientMsg, ClientTransport, CommandMsg, InputMsg, LoopbackClient,
    LoopbackConnector, PROTOCOL_VERSION, RejectReason, ServerLimits, ServerMsg, Snapshot, Welcome,
    WireCommand, WireRefusal, WireTeam, WireVec2, loopback,
};
use warship_rules::{Command, ShipId, Sim, SkillId, Tuning, Vec2, Waypoint};
use warship_server::{GameServer, Notice, Orders, Pilot};

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

    /// 빈자리를 `Drifter`가 조종하는 배로 채우는 서버입니다.
    fn with_bots(tuning: Tuning) -> Bench {
        let mut bench = Bench::new(tuning);
        bench.server = bench.server.with_pilot(Box::new(Drifter));
        bench
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

    fn command(&mut self, seq: u32, command: WireCommand) {
        self.link
            .send(&ClientMsg::Command(CommandMsg { seq, command }));
    }

    fn sail_to(&mut self, seq: u32, x: f32, y: f32) {
        self.link.send(&ClientMsg::Input(InputMsg {
            seq,
            destination: Some(WireVec2::new(x, y)),
        }));
    }
}

/// 시험용 조종사입니다. 모든 배를 맵의 한가운데로 보내고, 0번 스킬을 배우게 합니다.
struct Drifter;

impl Pilot for Drifter {
    fn steer(&mut self, sim: &Sim, _ship: ShipId) -> Orders {
        let map = &sim.tuning().map;
        Orders {
            destination: Waypoint::new(map.width / 2.0, map.height / 2.0),
            commands: vec![Command::Learn(SkillId(0))],
        }
    }
}

/// 두 진영의 시작 자리가 서로 보이는 거리(5타일)에 있는 수치입니다. 건물과 NPC 배는 없고, 배는 무기 없이 시작합니다.
fn close_spawns() -> Tuning {
    let mut tuning = Tuning::default();
    tuning.map.structures.clear();
    tuning.waves.clear();
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

#[test]
fn a_link_that_does_not_join_in_time_is_closed() {
    let mut bench = Bench::new(Tuning::default());
    let mut silent = bench.connector.connect();
    let mut rejected = bench.join("no spaces");
    let mut playing = bench.join("test");

    // 기한은 5초(100Tick)입니다.
    bench.run_ticks(99);
    assert!(
        !bench
            .notices
            .iter()
            .any(|notice| matches!(notice, Notice::LinkClosed { .. }))
    );
    bench.run_ticks(2);

    let closed = |link: &mut LoopbackClient| {
        let mut last = None;
        while let Some(event) = link.poll() {
            last = Some(event);
        }
        matches!(last, Some(ClientEvent::Disconnected { .. }))
    };
    assert!(closed(&mut silent), "never sent a join");
    assert!(closed(&mut rejected.link), "its join was rejected");
    assert!(!closed(&mut playing.link), "joined in time");
    let closures = bench
        .notices
        .iter()
        .filter(|notice| matches!(notice, Notice::LinkClosed { .. }));
    assert_eq!(closures.count(), 2);
}

#[test]
fn a_snapshot_carries_own_creeps_and_own_gold_but_not_the_far_enemys() {
    let mut bench = Bench::new(Tuning::default());
    let mut red = bench.join("test");
    let _blue = bench.join("test");

    // 기본 수치에서 NPC 배는 40번째 Tick부터 나오고, 60번째 Tick까지 3초의 수입(6골드)이 들어옵니다.
    bench.run_ticks(60);
    let snapshot = red.last_snapshot();

    assert!(!snapshot.creeps.is_empty());
    assert!(
        snapshot
            .creeps
            .iter()
            .all(|creep| creep.team == WireTeam::Red)
    );
    assert_eq!(snapshot.structures.len(), 10, "structures of both teams");
    let own = snapshot.ships.iter().map(|ship| ship.own.as_ref());
    let golds: Vec<Option<u32>> = own.map(|own| own.map(|own| own.gold)).collect();
    assert_eq!(golds, [Some(6)]);
    assert_eq!(snapshot.outcome, None);
}

#[test]
fn a_command_is_handled_once_and_its_result_comes_back_in_the_snapshot() {
    let mut bench = Bench::new(Tuning::default());
    let mut kang = bench.join("test");
    bench.run_ticks(1);

    kang.command(1, WireCommand::Learn { skill: 0 });
    kang.command(1, WireCommand::Learn { skill: 1 });
    bench.run_ticks(1);
    let own = kang.last_snapshot().ships[0].own.clone().expect("my ship");
    let ranks: Vec<u8> = own.skills.iter().map(|skill| skill.rank).collect();
    assert_eq!(ranks, [1, 0, 0], "the repeated number is ignored");
    assert_eq!((own.skill_points, own.last_command_seq), (0, 1));
    assert_eq!(own.refusal, None);

    // 시작 자리는 본진의 상점 안이지만 골드가 모자랍니다.
    kang.command(2, WireCommand::Buy { weapon: 0 });
    bench.run_ticks(1);
    let own = kang.last_snapshot().ships[0].own.clone().expect("my ship");
    assert_eq!(own.last_command_seq, 2);
    assert_eq!(own.refusal, Some(WireRefusal::NotEnoughGold));
}

#[test]
fn a_ship_buys_a_weapon_at_its_base_once_it_has_the_gold() {
    let mut bench = Bench::new(Tuning::default());
    let mut kang = bench.join("test");
    // 수입은 1초에 2골드이고, 0번 무기의 가격은 100입니다.
    bench.run_ticks(1000);

    kang.command(1, WireCommand::Buy { weapon: 0 });
    bench.run_ticks(1);

    let ship = kang.last_snapshot().ships.remove(0);
    let weapons: Vec<u16> = ship.mounts.iter().map(|mount| mount.weapon).collect();
    assert_eq!(weapons, [0, 0], "the start weapon and the bought one");
    let own = ship.own.expect("my ship");
    assert_eq!((own.gold, own.refusal), (0, None));
}

#[test]
fn a_room_with_a_pilot_fills_every_seat_and_the_pilot_steers_the_spare_ships() {
    let mut bench = Bench::with_bots(Tuning::default());
    let mut kang = bench.join("test");
    bench.run_ticks(1);
    let me = kang.welcome().ship;

    bench.run_ticks(40);

    let ships = kang.last_snapshot().ships;
    assert_eq!(ships.len(), 5, "my team of five, the far enemy is not seen");
    for ship in ships {
        let rank = ship.own.expect("my team").skills[0].rank;
        if ship.id == me {
            assert_eq!((ship.x, rank), (7.0, 0), "nobody steers my ship");
        } else {
            assert!(ship.x > 7.0, "ship {} sails to the middle", ship.id);
            assert_eq!(rank, 1);
        }
    }
}

#[test]
fn a_player_takes_a_spare_seat_and_gives_it_back_on_leaving() {
    let mut bench = Bench::with_bots(Tuning::default());
    let mut players: Vec<Player> = (0..10).map(|_| bench.join("test")).collect();
    bench.run_ticks(1);
    let teams: Vec<WireTeam> = players.iter_mut().map(|p| p.welcome().team).collect();
    let reds = teams.iter().filter(|team| **team == WireTeam::Red).count();
    assert_eq!(reds, 5, "the players are split evenly: {teams:?}");

    let mut late = bench.join("test");
    bench.run_ticks(1);
    let reason = RejectReason::RoomFull { capacity: 10 };
    assert_eq!(late.messages(), [ServerMsg::JoinRejected { reason }]);

    let mut gone = players.remove(0);
    gone.link.send(&ClientMsg::Leave);
    bench.run_ticks(2);
    let team_size = |player: &mut Player| player.last_snapshot().ships.len();
    assert!(players.iter_mut().all(|player| team_size(player) >= 5));
}
