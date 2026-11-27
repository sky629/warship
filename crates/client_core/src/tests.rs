//! 접속 상태의 전환과, 스냅샷에서 화면에 그릴 모습을 만드는 계산을 확인합니다.
//!
//! 시험용 방은 1초에 20Tick이고, 무기 0번은 쿨다운 10Tick에 포탄이 한 Tick에 1타일을 납니다.

use warship_protocol::{
    ClientEvent, ClientMsg, CommandMsg, InputMsg, PROTOCOL_VERSION, RejectReason, ServerMsg,
    Snapshot, Welcome, WireCommand, WireCreep, WireCreepSpec, WireHull, WireMount, WireOutcome,
    WireOwn, WireRefusal, WireShip, WireSkill, WireSkillSpec, WireStructure, WireStructureKind,
    WireStructureSpec, WireTarget, WireTeam, WireVec2, WireWeapon,
};
use warship_rules::Vec2;

use crate::{Phase, Session, SkillView};

const ME: u32 = 1;
const ENEMY: u32 = 2;

fn welcome() -> Welcome {
    let gun = WireWeapon {
        name: "gun".to_string(),
        price: 100,
        range: 5.0,
        cooldown_ticks: 10,
        shot_speed: 20.0,
    };
    let armed = WireStructureSpec {
        max_hp: 1000,
        weapon: Some(gun.clone()),
    };
    Welcome {
        room: "test".to_string(),
        ship: ME,
        team: WireTeam::Red,
        tick_hz: 20,
        map_width: 40.0,
        map_height: 40.0,
        hulls: vec![
            WireHull {
                name: "small".to_string(),
                price: 0,
                max_hp: 1000,
                mounts: 2,
                skills: vec![0],
            },
            WireHull {
                name: "big".to_string(),
                price: 400,
                max_hp: 2000,
                mounts: 4,
                skills: vec![0],
            },
        ],
        weapons: vec![gun.clone()],
        skills: vec![WireSkillSpec {
            name: "skill".to_string(),
            cooldown_ticks: 100,
            max_rank: 2,
        }],
        tower: armed.clone(),
        harbor: WireStructureSpec {
            max_hp: 1000,
            weapon: None,
        },
        base: armed,
        creeps: vec![WireCreepSpec {
            name: "creep".to_string(),
            max_hp: 100,
            weapon: gun,
        }],
        lanes: Vec::new(),
        islands: Vec::new(),
    }
}

fn ship(id: u32, x: f32) -> WireShip {
    WireShip {
        id,
        team: if id == ME {
            WireTeam::Red
        } else {
            WireTeam::Blue
        },
        x,
        y: 10.0,
        hull: 0,
        hp: 500,
        level: 1,
        respawn_at: None,
        own: (id == ME).then(|| WireOwn {
            gold: 70,
            xp: 0,
            skill_points: 1,
            skills: vec![WireSkill {
                rank: 0,
                ready_at: 0,
            }],
            last_command_seq: 0,
            refusal: None,
        }),
        mounts: Vec::new(),
        last_input_seq: 0,
    }
}

fn snapshot(tick: u64, ships: Vec<WireShip>) -> ServerMsg {
    ServerMsg::Snapshot(Box::new(Snapshot {
        tick,
        ships,
        creeps: Vec::new(),
        structures: Vec::new(),
        outcome: None,
    }))
}

/// 입장을 마친 접속입니다.
fn playing() -> Session {
    let mut session = Session::new("test");
    session.handle(ClientEvent::Connected, 0.0);
    session.handle(
        ClientEvent::Message(ServerMsg::Welcome(Box::new(welcome()))),
        0.0,
    );
    session
}

fn receive(session: &mut Session, at: f64, msg: ServerMsg) {
    assert_eq!(session.handle(ClientEvent::Message(msg), at), None);
}

#[test]
fn a_session_joins_when_the_link_opens_and_plays_after_the_welcome() {
    let mut session = Session::new("test");
    assert_eq!(session.phase(), &Phase::Connecting);
    assert_eq!(session.sail_to(1.0, 1.0), None, "not in a room yet");

    let join = session.handle(ClientEvent::Connected, 0.0);
    assert_eq!(
        join,
        Some(ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: "test".to_string()
        })
    );
    assert_eq!(session.phase(), &Phase::Joining);

    receive(&mut session, 0.0, ServerMsg::Welcome(Box::new(welcome())));
    assert_eq!(session.phase(), &Phase::Playing);
    assert_eq!(session.frame(0.0), None, "no snapshot yet");
}

#[test]
fn a_rejection_and_a_lost_link_end_the_session_with_the_reason() {
    let mut session = Session::new("test");
    session.handle(ClientEvent::Connected, 0.0);
    let reason = RejectReason::RoomFull { capacity: 10 };
    receive(&mut session, 0.0, ServerMsg::JoinRejected { reason });
    assert_eq!(
        session.phase(),
        &Phase::Ended("The room is full (10 ships)".to_string())
    );

    let mut session = playing();
    let lost = ClientEvent::Disconnected {
        reason: "the server is gone".to_string(),
    };
    session.handle(lost, 1.0);
    assert_eq!(
        session.phase(),
        &Phase::Ended("the server is gone".to_string())
    );
    assert_eq!(session.sail_to(1.0, 1.0), None);
}

#[test]
fn every_move_order_carries_the_next_sequence_number() {
    let mut session = playing();

    let orders = [session.sail_to(3.0, 4.0), session.sail_to(5.0, 6.0)];

    let input = |seq, x, y| {
        Some(ClientMsg::Input(InputMsg {
            seq,
            destination: Some(WireVec2::new(x, y)),
        }))
    };
    assert_eq!(orders, [input(1, 3.0, 4.0), input(2, 5.0, 6.0)]);
}

#[test]
fn the_frame_shows_the_room_a_tenth_of_a_second_ago_between_two_snapshots() {
    let mut session = playing();
    receive(&mut session, 1.00, snapshot(20, vec![ship(ME, 10.0)]));
    receive(&mut session, 1.05, snapshot(21, vec![ship(ME, 10.2)]));

    // 1.125초에 보여 주는 시각은 1.025초이고, 두 스냅샷의 한가운데입니다.
    let frame = session.frame(1.125).unwrap();

    let me = &frame.ships[0];
    assert!((me.pos.x - 10.1).abs() < 1e-4, "got {}", me.pos.x);
    assert!(me.mine);
    assert_eq!(me.hp, 0.5);
}

#[test]
fn the_frame_holds_the_last_snapshot_when_no_newer_one_has_come() {
    let mut session = playing();
    receive(&mut session, 1.00, snapshot(20, vec![ship(ME, 10.0)]));
    receive(&mut session, 1.05, snapshot(21, vec![ship(ME, 10.2)]));

    let frame = session.frame(5.0).unwrap();

    assert_eq!(frame.ships[0].pos, Vec2::new(10.2, 10.0));
}

#[test]
fn an_enemy_that_just_came_into_view_is_drawn_where_it_is() {
    let mut session = playing();
    receive(&mut session, 1.00, snapshot(20, vec![ship(ME, 10.0)]));
    receive(
        &mut session,
        1.05,
        snapshot(21, vec![ship(ME, 10.0), ship(ENEMY, 18.0)]),
    );

    let frame = session.frame(1.125).unwrap();

    assert_eq!(frame.ships[1].pos, Vec2::new(18.0, 10.0));
    assert!(!frame.ships[1].mine);
}

#[test]
fn a_shell_is_drawn_between_the_gun_and_its_target_while_it_flies() {
    // 20번째 Tick에 4타일 떨어진 적을 쏘았습니다. 포탄은 4Tick 동안 날아갑니다.
    let armed = |x| {
        let mount = WireMount {
            slot: 0,
            weapon: 0,
            target: Some(WireTarget::Ship(ENEMY)),
            ready_at: 30,
        };
        WireShip {
            mounts: vec![mount],
            ..ship(ME, x)
        }
    };
    let room = |tick| snapshot(tick, vec![armed(10.0), ship(ENEMY, 14.0)]);
    let mut session = playing();
    receive(&mut session, 1.00, room(21));
    receive(&mut session, 1.05, room(22));
    receive(&mut session, 1.20, room(25));

    // 22번째 Tick의 모습입니다. 포탄은 2Tick을 날아서 한가운데에 있습니다.
    let flying = session.frame(1.15).unwrap();
    assert_eq!(flying.shots.len(), 1);
    assert!((flying.shots[0].pos.x - 12.0).abs() < 1e-3);
    assert_eq!(flying.shots[0].team, WireTeam::Red);
    assert_eq!(flying.ships[0].ranges, [5.0]);

    // 25번째 Tick에는 포탄이 이미 닿았습니다.
    let landed = session.frame(1.30).unwrap();
    assert_eq!(landed.shots, []);
}

#[test]
fn the_frame_shows_creeps_structures_gold_the_respawn_time_and_the_outcome() {
    let sunk = WireShip {
        hp: 0,
        respawn_at: Some(140),
        ..ship(ME, 10.0)
    };
    let creep = |x| WireCreep {
        id: 7,
        team: WireTeam::Blue,
        kind: 0,
        x,
        y: 20.0,
        hp: 25,
        target: None,
        ready_at: 0,
    };
    let harbor = WireStructure {
        id: 3,
        kind: WireStructureKind::Harbor,
        team: WireTeam::Red,
        x: 5.0,
        y: 5.0,
        hp: 400,
        target: None,
        ready_at: 0,
    };
    let room = |tick, creep_x| {
        ServerMsg::Snapshot(Box::new(Snapshot {
            tick,
            ships: vec![sunk.clone()],
            creeps: vec![creep(creep_x)],
            structures: vec![harbor.clone()],
            outcome: Some(WireOutcome::Victory(WireTeam::Blue)),
        }))
    };
    let mut session = playing();
    receive(&mut session, 1.00, room(100, 30.0));
    receive(&mut session, 1.05, room(101, 29.9));

    // 1.15초에 보여 주는 것은 101번째 Tick입니다.
    let frame = session.frame(1.15).unwrap();

    assert!(frame.ships[0].sunk);
    let me = frame.me.as_ref().expect("my ship is in the snapshot");
    assert_eq!((me.gold, me.respawn_secs), (70, Some(39.0 / 20.0)));
    assert_eq!(frame.outcome, Some(WireOutcome::Victory(WireTeam::Blue)));
    assert_eq!((frame.creeps[0].pos.x, frame.creeps[0].hp), (29.9, 0.25));
    let site = &frame.structures[0];
    assert_eq!((site.hp, site.range), (0.4, None), "a harbor has no gun");
}

#[test]
fn every_command_carries_the_next_sequence_number() {
    let mut session = Session::new("test");
    assert_eq!(session.command(WireCommand::Sell { slot: 0 }), None);
    let mut session = playing();

    let buy = WireCommand::Buy { weapon: 0 };
    let cast = WireCommand::Cast { skill: 0 };
    let sent = [session.command(buy), session.command(cast)];

    let msg = |seq, command| Some(ClientMsg::Command(CommandMsg { seq, command }));
    assert_eq!(sent, [msg(1, buy), msg(2, cast)]);
}

#[test]
fn the_frame_shows_my_hull_mounts_skills_and_the_last_refusal() {
    // 큰 선체(HP 2000, 칸 4개)의 둘째 칸에 무기가 있고, 스킬은 120번째 Tick부터 다시 쓸 수 있습니다.
    let mount = WireMount {
        slot: 1,
        weapon: 0,
        target: None,
        ready_at: 0,
    };
    let mine = WireShip {
        hull: 1,
        level: 3,
        mounts: vec![mount],
        own: Some(WireOwn {
            gold: 250,
            xp: 320,
            skill_points: 2,
            skills: vec![WireSkill {
                rank: 1,
                ready_at: 120,
            }],
            last_command_seq: 4,
            refusal: Some(WireRefusal::NoShopInRange),
        }),
        ..ship(ME, 10.0)
    };
    let mut session = playing();
    receive(&mut session, 1.00, snapshot(100, vec![mine.clone()]));
    receive(&mut session, 1.05, snapshot(101, vec![mine]));

    // 1.15초에 보여 주는 것은 101번째 Tick입니다.
    let frame = session.frame(1.15).unwrap();

    assert_eq!(frame.ships[0].hp, 0.25, "500 of the big hull's 2000");
    let me = frame.me.expect("my ship is in the snapshot");
    assert_eq!((me.hull, me.level, me.xp, me.skill_points), (1, 3, 320, 2));
    assert_eq!(me.gold, 250);
    assert_eq!(me.mounts, [None, Some(0), None, None]);
    let skill = SkillView {
        rank: 1,
        ready_secs: 19.0 / 20.0,
    };
    assert_eq!(me.skills, [skill]);
    assert_eq!(me.refusal, Some(WireRefusal::NoShopInRange));
}
