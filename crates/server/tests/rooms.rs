//! 서버를 소켓 없이 실행해서 입장, 진영별 스냅샷, 입력, 퇴장, 끊긴 자리로 돌아오는 것을 확인합니다.
//!
//! 접속 방식은 같은 프로세스 안에서 메시지를 옮기는 `loopback`이고, 시각은 테스트가 정합니다.

use std::time::{Duration, Instant};

use warship_protocol::{
    ClientEvent, ClientMsg, ClientTransport, CommandMsg, InputMsg, LoopbackClient,
    LoopbackConnector, PROTOCOL_VERSION, RejectReason, SeatToken, ServerLimits, ServerMsg,
    Snapshot, Welcome, WireCommand, WireItem, WireMember, WireRefusal, WireTeam, WireVec2,
    loopback,
};
use warship_rules::{Command, ShipId, Sim, SkillId, Tuning, Vec2, Waypoint};
use warship_server::{GameServer, Notice, Orders, Pilot, PredictableRandom};

struct Bench {
    server: GameServer,
    connector: LoopbackConnector,
    now: Instant,
    notices: Vec<Notice>,
}

impl Bench {
    fn new(tuning: Tuning) -> Bench {
        Bench::with_limits(tuning, ServerLimits::default())
    }

    fn with_limits(tuning: Tuning, limits: ServerLimits) -> Bench {
        let (transport, connector) = loopback();
        let now = Instant::now();
        let random = Box::new(PredictableRandom::new(1));
        let server = GameServer::new(Box::new(transport), tuning, limits, random, now);
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

    /// 접속을 열고 `room`에 "KANG"이라는 이름으로 입장 요청을 보냅니다.
    fn join(&mut self, room: &str) -> Player {
        self.join_with(Some(room), "KANG", None)
    }

    /// 접속을 열고 입장 요청을 보냅니다. `room`이 없으면 서버가 방 코드를 만듭니다.
    fn join_with(&mut self, room: Option<&str>, name: &str, token: Option<SeatToken>) -> Player {
        let mut link = self.connector.connect();
        assert!(matches!(link.poll(), Some(ClientEvent::Connected)));
        link.send(&ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: room.map(str::to_string),
            name: name.to_string(),
            token,
        });
        Player::of(link)
    }

    /// 지금까지 받은 알림 가운데 접속을 닫은 알림의 수입니다.
    fn closures(&self) -> usize {
        let closed = |notice: &&Notice| matches!(notice, Notice::LinkClosed { .. });
        self.notices.iter().filter(closed).count()
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
    /// 지금까지 받은 명단과 스냅샷 가운데 마지막 것입니다.
    roster: Option<Vec<WireMember>>,
    snapshot: Option<Snapshot>,
}

impl Player {
    fn of(link: LoopbackClient) -> Player {
        Player {
            link,
            roster: None,
            snapshot: None,
        }
    }

    /// 지난번에 읽은 뒤 새로 받은 메시지입니다.
    fn messages(&mut self) -> Vec<ServerMsg> {
        let mut messages = Vec::new();
        while let Some(event) = self.link.poll() {
            if let ClientEvent::Message(msg) = event {
                match &msg {
                    ServerMsg::Roster(roster) => self.roster = Some(roster.clone()),
                    ServerMsg::Snapshot(snapshot) => self.snapshot = Some((**snapshot).clone()),
                    _ => {}
                }
                messages.push(msg);
            }
        }
        messages
    }

    /// 서버가 닫지 않게 접속이 살아 있다는 표시를 보냅니다.
    fn ping(&mut self) {
        self.link.send(&ClientMsg::Ping);
    }

    fn welcome(&mut self) -> Welcome {
        let found = self.messages().into_iter().find_map(|msg| match msg {
            ServerMsg::Welcome(welcome) => Some(*welcome),
            _ => None,
        });
        found.expect("the server sent a welcome")
    }

    /// 받은 명단 가운데 마지막 것입니다.
    fn last_roster(&mut self) -> Vec<WireMember> {
        self.messages();
        self.roster.clone().expect("the server sent a roster")
    }

    /// 서버가 이 접속을 닫았는지입니다.
    fn is_closed(&mut self) -> bool {
        let mut last = None;
        while let Some(event) = self.link.poll() {
            last = Some(event);
        }
        matches!(last, Some(ClientEvent::Disconnected { .. }))
    }

    /// 받은 스냅샷 가운데 마지막 것입니다.
    fn last_snapshot(&mut self) -> Snapshot {
        self.messages();
        self.snapshot.clone().expect("the server sent a snapshot")
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
    let member = |name: &str| WireMember {
        ship: 1,
        name: name.to_string(),
        connected: true,
    };
    assert_eq!(messages[1], ServerMsg::Roster(vec![member("KANG")]));
    let ticks: Vec<u64> = messages[2..]
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
        room: Some("test".to_string()),
        name: "KANG".to_string(),
        token: None,
    });
    let mut bad_code = bench.join("no spaces");
    let mut bad_name = bench.join_with(Some("test"), "K@NG", None);
    let _first = bench.join("test");
    let _second = bench.join("test");
    let mut third = bench.join("test");
    bench.run_ticks(1);

    let reason = |messages: Vec<ServerMsg>| match messages.as_slice() {
        [ServerMsg::JoinRejected { reason }] => *reason,
        other => panic!("expected one rejection, got {other:?}"),
    };
    let mut old = Player::of(old);
    assert_eq!(
        reason(old.messages()),
        RejectReason::VersionMismatch {
            server: PROTOCOL_VERSION
        }
    );
    assert_eq!(reason(bad_code.messages()), RejectReason::BadRoomCode);
    assert_eq!(reason(bad_name.messages()), RejectReason::BadName);
    assert_eq!(
        reason(third.messages()),
        RejectReason::RoomFull { capacity: 2 }
    );
}

#[test]
fn a_player_who_leaves_is_removed_and_a_room_without_seats_is_removed() {
    let mut bench = Bench::new(close_spawns());
    let mut red = bench.join("test");
    let mut blue = bench.join("test");
    bench.run_ticks(2);
    assert_eq!(red.last_snapshot().ships.len(), 2);

    blue.link.send(&ClientMsg::Leave);
    bench.run_ticks(2);
    assert_eq!(red.last_snapshot().ships.len(), 1);

    red.link.send(&ClientMsg::Leave);
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
    let silent = bench.connector.connect();
    let mut rejected = bench.join("no spaces");
    let mut playing = bench.join("test");

    // 기한은 5초(100Tick)입니다.
    bench.run_ticks(99);
    assert_eq!(bench.closures(), 0);
    bench.run_ticks(2);

    let mut silent = Player::of(silent);
    assert!(silent.is_closed(), "never sent a join");
    assert!(rejected.is_closed(), "its join was rejected");
    assert!(!playing.is_closed(), "joined in time");
    assert_eq!(bench.closures(), 2);
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
    assert_eq!(snapshot.structures.len(), 14, "structures of both teams");
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
    assert_eq!(
        ranks,
        [1, 0, 0, 0, 0, 0, 0, 0],
        "the repeated number is ignored"
    );
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
    for _ in 0..50 {
        kang.ping();
        bench.run_ticks(20);
    }

    kang.command(1, WireCommand::Buy { weapon: 0 });
    bench.run_ticks(1);

    let ship = kang.last_snapshot().ships.remove(0);
    let items: Vec<WireItem> = ship.mounts.iter().map(|mount| mount.item).collect();
    let gun = WireItem::Weapon(0);
    assert_eq!(items, [gun, gun], "the start weapon and the bought one");
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
            assert_eq!((ship.x, rank), (9.0, 0), "nobody steers my ship");
        } else {
            assert!(ship.x > 9.0, "ship {} sails to the middle", ship.id);
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

/// 방 "test"에 입장하고 입장 안내를 받은 플레이어입니다.
fn joined(bench: &mut Bench) -> (Player, Welcome) {
    let mut player = bench.join("test");
    bench.run_ticks(1);
    let welcome = player.welcome();
    (player, welcome)
}

#[test]
fn a_player_who_lost_the_link_comes_back_to_the_same_ship_with_the_seat_token() {
    let mut bench = Bench::new(close_spawns());
    let (mut red, _) = joined(&mut bench);
    let (blue, old) = joined(&mut bench);
    let room = "test".parse_room();
    let ship = ShipId(old.ship);

    drop(blue);
    bench.run_ticks(20);
    assert!(bench.notices.contains(&Notice::Held {
        room: room.clone(),
        ship
    }));
    let away = red.last_roster();
    assert_eq!((away[1].ship, away[1].connected), (old.ship, false));
    assert_eq!(red.last_snapshot().ships.len(), 2, "the ship stays");

    let mut back = bench.join_with(Some("test"), "MIRO", Some(old.token));
    bench.run_ticks(1);
    let welcome = back.welcome();
    assert_eq!((welcome.ship, welcome.token), (old.ship, old.token));
    assert!(bench.notices.contains(&Notice::Returned { room, ship }));
    let here = red.last_roster();
    assert_eq!(here[1].name, "MIRO", "the new name is used");
    assert!(here[1].connected);

    // 돌아온 접속은 순서 번호를 1부터 다시 셉니다.
    back.sail_to(1, 20.0, 10.0);
    bench.run_ticks(20);
    let ships = back.last_snapshot().ships;
    let mine = ships.iter().find(|ship| ship.id == old.ship);
    assert!(mine.is_some_and(|ship| ship.x > 16.0 && ship.last_input_seq == 1));
}

#[test]
fn a_second_link_with_the_same_token_takes_the_seat_and_the_first_is_told() {
    let mut bench = Bench::new(close_spawns());
    let (mut first, old) = joined(&mut bench);
    first.messages();

    let mut second = bench.join_with(Some("test"), "KANG", Some(old.token));
    bench.run_ticks(1);

    assert_eq!(second.welcome().ship, old.ship);
    assert_eq!(first.messages(), [ServerMsg::SeatTakenOver]);
    // 이유를 들은 접속이 5초 안에 스스로 닫지 않으면 서버가 닫습니다.
    bench.run_ticks(101);
    assert!(first.is_closed());
    assert!(!second.is_closed());
}

#[test]
fn a_token_of_another_seat_or_another_room_gives_a_new_ship() {
    let mut bench = Bench::new(close_spawns());
    let (_kang, old) = joined(&mut bench);

    let mut other_room = bench.join_with(Some("other"), "MIRO", Some(old.token));
    let mut wrong = bench.join_with(Some("test"), "MIRO", Some(SeatToken([9; 16])));
    bench.run_ticks(1);

    assert_ne!(other_room.welcome().token, old.token);
    let welcome = wrong.welcome();
    assert_ne!((welcome.ship, welcome.token), (old.ship, old.token));
}

#[test]
fn a_held_seat_ends_after_a_minute_and_its_ship_leaves_the_room() {
    let mut bench = Bench::new(close_spawns());
    let (mut red, _) = joined(&mut bench);
    let (blue, old) = joined(&mut bench);
    drop(blue);

    // 자리는 60초(1200Tick) 동안 남습니다. 입장한 접속은 30초 동안 조용하면 닫히므로 `Ping`을 보냅니다.
    for _ in 0..59 {
        red.ping();
        bench.run_ticks(20);
    }
    assert_eq!(red.last_snapshot().ships.len(), 2);
    red.ping();
    bench.run_ticks(25);

    assert_eq!(red.last_snapshot().ships.len(), 1);
    assert_eq!(red.last_roster().len(), 1);
    assert!(bench.notices.contains(&Notice::Left {
        room: "test".parse_room(),
        ship: ShipId(old.ship),
        members: 1
    }));

    // 표식의 자리가 끝났으므로 같은 표식은 새 배를 받습니다.
    let mut late = bench.join_with(Some("test"), "KANG", Some(old.token));
    bench.run_ticks(1);
    assert_ne!(late.welcome().token, old.token);
}

#[test]
fn a_room_whose_last_seat_ends_is_removed() {
    let mut bench = Bench::new(close_spawns());
    let (kang, _) = joined(&mut bench);

    drop(kang);
    bench.run_ticks(1199);
    assert!(!bench.notices.contains(&Notice::RoomRemoved {
        room: "test".parse_room()
    }));
    bench.run_ticks(3);

    assert_eq!(
        bench.notices.last(),
        Some(&Notice::RoomRemoved {
            room: "test".parse_room()
        })
    );
}

#[test]
fn the_pilot_steers_the_ship_of_a_lost_link_and_keeps_it_when_the_seat_ends() {
    let mut bench = Bench::with_bots(Tuning::default());
    // 입장한 순서대로 붉은 진영과 푸른 진영에 번갈아 들어갑니다. `mate`는 `gone`과 같은 진영이라서 그 배를 봅니다.
    let (_red, _) = joined(&mut bench);
    let (gone, old) = joined(&mut bench);
    let (_other, _) = joined(&mut bench);
    let (mut mate, mate_welcome) = joined(&mut bench);
    assert_eq!(mate_welcome.team, old.team);
    drop(gone);

    for _ in 0..3 {
        mate.ping();
        bench.run_ticks(20);
    }
    let x_of = |snapshot: Snapshot| {
        let ship = snapshot.ships.into_iter().find(|ship| ship.id == old.ship);
        ship.map(|ship| ship.x)
    };
    let start = if old.team == WireTeam::Red { 9.0 } else { 87.0 };
    let sailed = x_of(mate.last_snapshot());
    assert!(sailed.is_some_and(|x| x != start), "the pilot sails it");

    // 사람이 돌아오면 조종사는 그 배를 놓습니다.
    let mut back = bench.join_with(Some("test"), "KANG", Some(old.token));
    bench.run_ticks(1);
    assert_eq!(back.welcome().ship, old.ship);
    back.sail_to(1, start, 32.0);
    bench.run_ticks(200);
    assert_eq!(x_of(back.last_snapshot()), Some(start));

    // 다시 끊긴 뒤 자리가 끝나도 배는 빈자리의 배로 남습니다.
    drop(back);
    for _ in 0..61 {
        mate.ping();
        bench.run_ticks(20);
    }
    assert!(x_of(mate.last_snapshot()).is_some());
    let roster = mate.last_roster();
    assert!(roster.iter().all(|member| member.ship != old.ship));
}

#[test]
fn a_joined_link_that_sends_nothing_for_thirty_seconds_is_closed_and_its_seat_is_held() {
    let mut bench = Bench::new(close_spawns());
    let (mut quiet, old) = joined(&mut bench);
    let (mut alive, _) = joined(&mut bench);

    for _ in 0..31 {
        alive.ping();
        bench.run_ticks(20);
    }

    assert!(quiet.is_closed());
    assert!(!alive.is_closed());
    assert!(bench.notices.contains(&Notice::Held {
        room: "test".parse_room(),
        ship: ShipId(old.ship)
    }));
}

#[test]
fn a_join_without_a_room_code_gets_a_new_room_each_time() {
    let mut bench = Bench::new(close_spawns());
    let mut first = bench.join_with(None, "KANG", None);
    let mut second = bench.join_with(None, "MIRO", None);
    bench.run_ticks(1);

    let (first, second) = (first.welcome(), second.welcome());
    assert_eq!(first.room.len(), 5);
    assert_ne!(first.room, second.room);
    assert_eq!((first.team, second.team), (WireTeam::Red, WireTeam::Red));
}

#[test]
fn the_server_refuses_a_room_above_its_limit_but_not_a_seat_in_an_open_room() {
    let limits = ServerLimits {
        max_rooms: 1,
        ..ServerLimits::default()
    };
    let mut bench = Bench::with_limits(close_spawns(), limits);
    let _first = bench.join("here");
    let mut second = bench.join("there");
    let mut third = bench.join("here");
    bench.run_ticks(1);

    let reason = RejectReason::ServerFull;
    assert_eq!(second.messages(), [ServerMsg::JoinRejected { reason }]);
    assert_eq!(third.welcome().team, WireTeam::Blue);
}

#[test]
fn a_link_that_breaks_the_order_of_messages_is_closed_and_gives_up_its_seat() {
    let mut bench = Bench::new(close_spawns());
    let mut early = bench.connector.connect();
    early.poll();
    early.send(&ClientMsg::Ping);
    let (mut twice, old) = joined(&mut bench);
    twice.link.send(&ClientMsg::Join {
        version: PROTOCOL_VERSION,
        room: Some("test".to_string()),
        name: "KANG".to_string(),
        token: None,
    });
    bench.run_ticks(2);

    let mut early = Player::of(early);
    assert!(early.is_closed(), "a message before the join");
    assert!(twice.is_closed(), "a second join");
    assert!(bench.notices.contains(&Notice::Left {
        room: "test".parse_room(),
        ship: ShipId(old.ship),
        members: 0
    }));
}

#[test]
fn a_seat_above_the_limit_of_held_seats_ends_at_once() {
    let limits = ServerLimits {
        max_held_seats_per_ip: 1,
        ..ServerLimits::default()
    };
    let mut bench = Bench::with_limits(close_spawns(), limits);
    let (_stays, _) = joined(&mut bench);
    let (first, _) = joined(&mut bench);
    let mut third = bench.join("other");
    bench.run_ticks(1);
    let second = third.welcome();

    drop(first);
    bench.run_ticks(1);
    drop(third);
    bench.run_ticks(1);

    // 테스트의 모든 접속은 같은 주소에서 옵니다. 첫 자리는 유지하고, 둘째 자리는 곧바로 끝냅니다.
    let held = |notice: &&Notice| matches!(notice, Notice::Held { .. });
    assert_eq!(bench.notices.iter().filter(held).count(), 1);
    assert!(bench.notices.contains(&Notice::Left {
        room: "other".parse_room(),
        ship: ShipId(second.ship),
        members: 0
    }));
}
