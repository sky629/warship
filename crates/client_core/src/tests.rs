//! 접속 상태의 전환과, 스냅샷에서 화면에 그릴 모습을 만드는 계산을 확인합니다.
//!
//! 시험용 방은 1초에 20Tick이고, 무기 0번은 쿨다운 10Tick에 포탄이 한 Tick에 1타일을 납니다.

use warship_protocol::{
    ClientEvent, ClientMsg, CommandMsg, InputMsg, PROTOCOL_VERSION, RejectReason, ServerMsg,
    Snapshot, Welcome, WireCommand, WireCreep, WireCreepSpec, WireGear, WireHull, WireItem,
    WireMount, WireOutcome, WireOwn, WireRefusal, WireShip, WireSkill, WireSkillSpec,
    WireStructure, WireStructureKind, WireStructureSpec, WireTarget, WireTeam, WireVec2,
    WireWeapon,
};
use warship_protocol::{
    SeatToken, WireHire, WireMember, WireRecipe, WireScore, WireShop, WireShot, WireSupply,
    WireTrade,
};
use warship_rules::Vec2;

use crate::{
    BlastView, Icon, IconKind, MountView, Phase, Session, SkillView, first_tab, mount_views,
    score_table, shop_tabs, shops_near,
};
use crate::{HudView, ScoreRow, ScoreTeam, SkillSlot, TopBar, hud_view, top_bar};

const ME: u32 = 1;
const ENEMY: u32 = 2;

const TOKEN: SeatToken = SeatToken([7; 16]);

fn welcome() -> Welcome {
    let gun = WireWeapon {
        name: "gun".to_string(),
        price: 100,
        buyable: true,
        damage: 10,
        range: 5.0,
        cooldown_ticks: 10,
        shot_speed: 20.0,
        shot: WireShot::Shell,
    };
    let armed = WireStructureSpec {
        max_hp: 1000,
        weapon: Some(gun.clone()),
    };
    Welcome {
        room: "test".to_string(),
        token: TOKEN,
        ship: ME,
        team: WireTeam::Red,
        tick_hz: 20,
        level_xp: vec![100, 300],
        shop_range: 5.0,
        map_width: 40.0,
        map_height: 40.0,
        hulls: vec![
            WireHull {
                name: "small".to_string(),
                price: 0,
                max_hp: 1000,
                mounts: 2,
                skills: vec![0],
                traits: String::new(),
            },
            WireHull {
                name: "big".to_string(),
                price: 400,
                max_hp: 2000,
                mounts: 4,
                skills: vec![0],
                traits: "trader, -15% damage".to_string(),
            },
        ],
        weapons: vec![gun.clone()],
        gear: Vec::new(),
        recipes: Vec::new(),
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
        supplies: Vec::new(),
        trades: Vec::new(),
        hire: WireHire {
            price: 220,
            creep: 0,
            count: 2,
        },
        shops: Vec::new(),
        ports: Vec::new(),
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
        max_hp: 1000,
        level: 1,
        respawn_at: None,
        slowed: false,
        cloaked: false,
        burning: false,
        own: (id == ME).then(|| WireOwn {
            gold: 70,
            xp: 0,
            skill_points: 1,
            skills: vec![WireSkill {
                rank: 0,
                ready_at: 0,
            }],
            supply_ready_at: 0,
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
        scores: Vec::new(),
        outcome: None,
    }))
}

#[test]
fn the_scoreboard_lists_each_team_with_the_best_ship_first() {
    let score = |ship, team, kills, deaths| WireScore {
        ship,
        team,
        level: 3,
        kills,
        deaths,
        creep_kills: 14,
    };
    let scores = [
        score(1, WireTeam::Red, 0, 2),
        score(2, WireTeam::Blue, 1, 0),
        score(3, WireTeam::Red, 4, 1),
    ];
    let roster = [WireMember {
        ship: 3,
        name: "KANG".to_string(),
        connected: true,
    }];

    let table = score_table(&scores, &roster, 3);

    let row = |name: &str, mine, kills, deaths| ScoreRow {
        name: name.to_string(),
        mine,
        level: 3,
        kills,
        deaths,
        creep_kills: 14,
    };
    let expected = [
        ScoreTeam {
            team: WireTeam::Red,
            kills: 4,
            rows: vec![row("KANG", true, 4, 1), row("Bot 1", false, 0, 2)],
        },
        ScoreTeam {
            team: WireTeam::Blue,
            kills: 1,
            rows: vec![row("Bot 2", false, 1, 0)],
        },
    ];
    assert_eq!(table, expected);
}

/// 입장을 마친 접속입니다.
fn playing() -> Session {
    let mut session = Session::new(Some("test"), "KANG", None);
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
    let mut session = Session::new(Some("test"), "KANG", None);
    assert_eq!(session.phase(), &Phase::Connecting);
    assert_eq!(session.sail_to(1.0, 1.0), None, "not in a room yet");

    let join = session.handle(ClientEvent::Connected, 0.0);
    assert_eq!(
        join,
        Some(ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: Some("test".to_string()),
            name: "KANG".to_string(),
            token: None,
        })
    );
    assert_eq!(session.phase(), &Phase::Joining);

    receive(&mut session, 0.0, ServerMsg::Welcome(Box::new(welcome())));
    assert_eq!(session.phase(), &Phase::Playing);
    assert_eq!(session.frame(0.0), None, "no snapshot yet");
}

#[test]
fn a_rejection_and_a_link_lost_before_the_welcome_end_the_session_with_the_reason() {
    let mut session = Session::new(Some("test"), "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
    let reason = RejectReason::RoomFull { capacity: 10 };
    receive(&mut session, 0.0, ServerMsg::JoinRejected { reason });
    assert_eq!(
        session.phase(),
        &Phase::Ended("The room is full (10 ships)".to_string())
    );

    let mut session = Session::new(Some("test"), "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
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
            item: WireItem::Weapon(0),
            target: Some(WireTarget::Ship(ENEMY)),
            aim: None,
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
    assert_eq!(flying.shots[0].kind, WireShot::Shell);
    assert_eq!(flying.shots[0].lift, 0.5, "a shell flies level");
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
            scores: Vec::new(),
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
    let mut session = Session::new(Some("test"), "KANG", None);
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
        item: WireItem::Weapon(0),
        target: None,
        aim: None,
        ready_at: 0,
    };
    let mine = WireShip {
        hull: 1,
        max_hp: 2000,
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
            supply_ready_at: 161,
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
    assert_eq!(me.mounts, [None, Some(WireItem::Weapon(0)), None, None]);
    let skill = SkillView {
        rank: 1,
        ready_secs: 19.0 / 20.0,
    };
    assert_eq!(me.skills, [skill]);
    assert_eq!(me.supply_secs, 3.0, "60 ticks after tick 101");
    assert_eq!(me.refusal, Some(WireRefusal::NoShopInRange));
}

fn lose_link(session: &mut Session, now: f64) {
    let reason = "the socket closed".to_string();
    session.handle(ClientEvent::Disconnected { reason }, now);
}

#[test]
fn a_session_that_loses_its_link_in_a_room_comes_back_with_its_seat_token() {
    let mut session = Session::new(None, "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
    receive(&mut session, 0.0, ServerMsg::Welcome(Box::new(welcome())));

    lose_link(&mut session, 10.0);
    assert_eq!(session.phase(), &Phase::Reconnecting);
    assert!(!session.should_reconnect(11.9), "not yet");
    assert!(session.should_reconnect(12.0));
    assert!(!session.should_reconnect(12.1), "one link at a time");

    // 새 접속은 입장 안내가 알려 준 방과 표식으로 입장합니다.
    let join = session.handle(ClientEvent::Connected, 12.2);
    assert_eq!(
        join,
        Some(ClientMsg::Join {
            version: PROTOCOL_VERSION,
            room: Some("test".to_string()),
            name: "KANG".to_string(),
            token: Some(TOKEN),
        })
    );
    receive(&mut session, 12.3, ServerMsg::Welcome(Box::new(welcome())));
    assert_eq!(session.phase(), &Phase::Playing);
    let first = session.sail_to(1.0, 1.0);
    assert!(
        matches!(first, Some(ClientMsg::Input(input)) if input.seq == 1),
        "the numbers start over"
    );
}

#[test]
fn a_session_keeps_trying_for_a_minute_and_then_ends() {
    let mut session = Session::new(None, "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
    receive(&mut session, 0.0, ServerMsg::Welcome(Box::new(welcome())));
    lose_link(&mut session, 10.0);

    // 새 접속이 열리지 않으면 2초 뒤에 다시 엽니다.
    assert!(session.should_reconnect(12.0));
    lose_link(&mut session, 12.5);
    assert!(!session.should_reconnect(14.0));
    assert!(session.should_reconnect(14.5));
    lose_link(&mut session, 69.0);

    assert!(!session.should_reconnect(70.1));
    assert!(matches!(session.phase(), Phase::Ended(_)));
}

#[test]
fn a_session_that_never_joined_or_lost_its_seat_does_not_come_back() {
    let mut early = Session::new(None, "KANG", None);
    lose_link(&mut early, 1.0);
    assert!(matches!(early.phase(), Phase::Ended(_)));

    let mut replaced = Session::new(None, "KANG", None);
    replaced.handle(ClientEvent::Connected, 0.0);
    receive(&mut replaced, 0.0, ServerMsg::Welcome(Box::new(welcome())));
    receive(&mut replaced, 5.0, ServerMsg::SeatTakenOver);
    lose_link(&mut replaced, 6.0);
    assert!(matches!(replaced.phase(), Phase::Ended(_)));
    assert!(!replaced.should_reconnect(100.0));
}

#[test]
fn a_session_in_a_room_pings_every_five_seconds_and_keeps_the_roster() {
    let mut session = Session::new(None, "KANG", None);
    assert_eq!(session.ping(100.0), None, "not in a room");
    session.handle(ClientEvent::Connected, 0.0);
    receive(&mut session, 0.0, ServerMsg::Welcome(Box::new(welcome())));

    assert_eq!(session.ping(4.9), None);
    assert_eq!(session.ping(5.0), Some(ClientMsg::Ping));
    assert_eq!(session.ping(9.9), None);

    let member = WireMember {
        ship: ME,
        name: "KANG".to_string(),
        connected: true,
    };
    receive(&mut session, 1.0, ServerMsg::Roster(vec![member.clone()]));
    assert_eq!(session.roster(), [member]);
}

#[test]
fn a_session_sends_the_remembered_token_with_its_first_join_and_leaves_once() {
    let mut session = Session::new(Some("test"), "KANG", Some(TOKEN));
    let join = session.handle(ClientEvent::Connected, 0.0);
    assert!(matches!(join, Some(ClientMsg::Join { token, .. }) if token == Some(TOKEN)));

    // 입장 안내를 받기 전에 끊기면, 표식이 있어도 다시 접속하지 않습니다.
    lose_link(&mut session, 1.0);
    assert!(matches!(session.phase(), Phase::Ended(_)));

    let mut session = playing();
    assert_eq!(session.leave(), Some(ClientMsg::Leave));
    assert_eq!(session.leave(), None);
    assert!(matches!(session.phase(), Phase::Ended(_)));
}

#[test]
fn the_shop_has_a_tab_for_each_part_of_the_home_shop_and_for_each_port() {
    let mut welcome = welcome();
    welcome.weapons.push(WireWeapon {
        name: "twin".to_string(),
        price: 350,
        buyable: false,
        ..welcome.weapons[0].clone()
    });
    welcome.weapons.push(WireWeapon {
        name: "mortar".to_string(),
        price: 400,
        ..welcome.weapons[0].clone()
    });
    welcome.gear.push(WireGear {
        name: "planks".to_string(),
        price: 200,
        effect: "+250 HP".to_string(),
    });
    welcome.recipes.push(WireRecipe {
        parts: vec![0, 0],
        result: 1,
        price: 150,
    });
    welcome.trades.push(WireTrade {
        name: "salt".to_string(),
        price: 60,
        reward: 90,
        port: 1,
    });

    welcome.supplies.push(WireSupply {
        name: "kit".to_string(),
        price: 80,
        effect: "+200 HP".to_string(),
    });

    // 맵의 높이가 40이므로, y가 30인 라인은 북쪽 라인입니다.
    let lane = vec![WireVec2::new(5.0, 30.0), WireVec2::new(35.0, 30.0)];
    welcome.lanes.push(lane);
    let nothing = WireShop {
        name: String::new(),
        weapons: Vec::new(),
        gear: Vec::new(),
        supplies: Vec::new(),
        recipes: Vec::new(),
        hulls: false,
        hire: false,
    };
    welcome.shops = vec![
        WireShop {
            name: "Base and harbors".to_string(),
            weapons: vec![0, 2],
            gear: vec![0],
            supplies: vec![0],
            recipes: vec![0],
            hulls: true,
            ..nothing.clone()
        },
        WireShop {
            name: "north port".to_string(),
            hire: true,
            ..nothing.clone()
        },
        WireShop {
            name: "south port".to_string(),
            ..nothing
        },
    ];

    let tabs = shop_tabs(&welcome);

    // 자기 진영의 상점은 탭이 셋이고, 항구는 파는 것이 있는 항구만 탭이 됩니다.
    let titles: Vec<(&str, usize)> = tabs
        .iter()
        .map(|tab| (tab.title.as_str(), tab.shop))
        .collect();
    assert_eq!(
        titles,
        [
            ("Armory", 0),
            ("Shipyard", 0),
            ("Trade", 0),
            ("north port", 1)
        ]
    );
    let seen = |tab: usize| -> Vec<(String, String, u32)> {
        let rows = tabs[tab].rows.iter();
        rows.map(|row| (row.name.clone(), row.note.clone(), row.price))
            .collect()
    };
    let row = |name: &str, note: &str, price| (name.to_string(), note.to_string(), price);
    assert_eq!(
        seen(0),
        [
            row("gun", "10 dmg, range 5, 0.5s", 100),
            row("mortar", "10 dmg, range 5, 0.5s", 400),
            row("planks", "+250 HP", 200),
            row("kit", "+200 HP, one use", 80),
            row("twin", "combine: 2 x gun", 150),
        ]
    );
    assert_eq!(
        seen(1),
        [
            row("small", "1000 HP, 2 mounts", 0),
            row("big", "2000 HP, 4 mounts, trader, -15% damage", 400),
        ]
    );
    assert_eq!(seen(2), [row("salt", "to south port, pays 90g", 60)]);
    assert_eq!(seen(3), [row("Hire: North lane", "2 x creep", 220)]);

    // 줄마다 명령과 드는 골드가 있습니다. 선체는 돌려받는 값이 있어서 드는 골드를 적지 않습니다.
    let orders: Vec<WireCommand> = tabs
        .iter()
        .flat_map(|tab| &tab.rows)
        .map(|row| row.order)
        .collect();
    assert_eq!(
        orders,
        [
            WireCommand::Buy { weapon: 0 },
            WireCommand::Buy { weapon: 2 },
            WireCommand::BuyGear { gear: 0 },
            WireCommand::BuySupply { supply: 0 },
            WireCommand::Combine { recipe: 0 },
            WireCommand::ChangeHull { hull: 0 },
            WireCommand::ChangeHull { hull: 1 },
            WireCommand::Contract { trade: 0 },
            WireCommand::Hire { lane: 0 },
        ]
    );
    let costs: Vec<Option<u32>> = tabs
        .iter()
        .flat_map(|tab| &tab.rows)
        .map(|row| row.cost)
        .collect();
    assert_eq!(
        costs,
        [
            Some(100),
            Some(400),
            Some(200),
            Some(80),
            Some(150),
            None,
            None,
            Some(60),
            Some(220)
        ]
    );
    // 무기와 조합의 줄은 무기의 수치를, 선체의 줄은 스킬을 설명합니다.
    assert_eq!(tabs[0].rows[4].detail, "damage 10, range 5, reload 0.5s");
    assert_eq!(tabs[1].rows[0].detail, "skills: skill");
    // 아이콘은 종류와 이름으로 정해집니다. 조합의 아이콘은 결과 무기의 아이콘입니다.
    let icons: Vec<String> = tabs
        .iter()
        .flat_map(|tab| &tab.rows)
        .map(|row| row.icon.path())
        .collect();
    assert_eq!(
        icons,
        [
            "icons/weapons/gun.png",
            "icons/weapons/mortar.png",
            "icons/gear/planks.png",
            "icons/supplies/kit.png",
            "icons/weapons/twin.png",
            "icons/hulls/small.png",
            "icons/hulls/big.png",
            "icons/trades/salt.png",
            "icons/hire/creep.png",
        ]
    );
}

#[test]
fn an_icon_has_a_file_named_after_it_and_initials_for_when_the_file_is_missing() {
    let icon = Icon::new(IconKind::Hull, "Man-of-War");
    assert_eq!(icon.path(), "icons/hulls/man_of_war.png");
    assert_eq!(icon.initials(), "M");

    let icon = Icon::new(IconKind::Weapon, "Twin Deck Guns");
    assert_eq!(icon.path(), "icons/weapons/twin_deck_guns.png");
    assert_eq!(icon.initials(), "TD");
}

#[test]
fn the_shop_opens_on_the_tab_of_the_shop_the_ship_is_at() {
    // 내 배는 (10, 10)에 있습니다. 첫 번째 항구는 (12, 10)에, 두 번째 항구는 (30, 10)에 있습니다. 상점 거리는 5입니다.
    let mut welcome = welcome();
    welcome.ports = vec![WireVec2::new(12.0, 10.0), WireVec2::new(30.0, 10.0)];
    let mut session = Session::new(Some("test"), "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
    session.handle(
        ClientEvent::Message(ServerMsg::Welcome(Box::new(welcome.clone()))),
        0.0,
    );
    receive(&mut session, 1.0, snapshot(20, vec![ship(ME, 10.0)]));
    let frame = session.frame(1.15).unwrap();

    let near = shops_near(&welcome, &frame);

    assert_eq!(
        near,
        [1],
        "no home structure is near, and the far port is not"
    );
    let tab = |shop| crate::ShopTab {
        title: String::new(),
        shop,
        rows: Vec::new(),
    };
    let tabs = [tab(0), tab(0), tab(1), tab(2)];
    assert_eq!(first_tab(&tabs, &near), 2);
    assert_eq!(first_tab(&tabs, &[]), 0);
}

#[test]
fn a_slowed_ship_a_cloaked_ship_and_a_burning_ship_are_marked_in_the_frame() {
    let mut session = playing();
    let slowed = WireShip {
        slowed: true,
        cloaked: true,
        burning: true,
        ..ship(ENEMY, 12.0)
    };
    receive(
        &mut session,
        1.0,
        snapshot(20, vec![ship(ME, 10.0), slowed]),
    );

    let frame = session.frame(1.15).unwrap();

    let marks: Vec<bool> = frame.ships.iter().map(|ship| ship.slowed).collect();
    assert_eq!(marks, [false, true]);
    let marks: Vec<bool> = frame.ships.iter().map(|ship| ship.cloaked).collect();
    assert_eq!(marks, [false, true]);
    let marks: Vec<bool> = frame.ships.iter().map(|ship| ship.burning).collect();
    assert_eq!(marks, [false, true]);
}

#[test]
fn the_mount_row_names_each_item_and_says_what_to_do_with_it() {
    let mut welcome = welcome();
    welcome.supplies.push(WireSupply {
        name: "kit".to_string(),
        price: 80,
        effect: "+200 HP".to_string(),
    });
    welcome.trades.push(WireTrade {
        name: "salt".to_string(),
        price: 60,
        reward: 90,
        port: 0,
    });
    let items = [
        Some(WireItem::Weapon(0)),
        Some(WireItem::Supply(0)),
        Some(WireItem::Contract(0)),
        Some(WireItem::Cargo(0)),
        None,
    ];
    let mounts = items.iter().enumerate().filter_map(|(slot, item)| {
        Some(WireMount {
            slot: slot as u8,
            item: (*item)?,
            target: None,
            aim: None,
            ready_at: 0,
        })
    });
    let mine = WireShip {
        hull: 1,
        mounts: mounts.collect(),
        ..ship(ME, 10.0)
    };
    let mut session = playing();
    receive(&mut session, 1.00, snapshot(100, vec![mine]));
    let mut me = session.frame(1.15).unwrap().me.expect("my ship");
    // 큰 선체의 칸은 4개입니다.
    assert_eq!(me.mounts.len(), 4);

    let seen = |me: &crate::MyShip| -> Vec<(String, String, bool)> {
        let views = mount_views(&welcome, me);
        let parts = |view: MountView| (view.name, view.note, view.usable);
        views.into_iter().map(parts).collect()
    };
    let text = |name: &str, note: &str, usable| (name.to_string(), note.to_string(), usable);
    assert_eq!(
        seen(&me),
        [
            text("gun", "", false),
            text("kit", "click: use", true),
            text("salt", "take to port", false),
            text("salt", "bring to base", false),
        ]
    );

    // 칸의 아이콘은 달린 아이템의 종류와 이름으로 정해지고, 빈 칸에는 없습니다.
    let icons = mount_views(&welcome, &me).into_iter().map(|view| view.icon);
    let paths: Vec<String> = icons.flatten().map(|icon| icon.path()).collect();
    assert_eq!(
        paths,
        [
            "icons/weapons/gun.png",
            "icons/supplies/kit.png",
            "icons/trades/salt.png",
            "icons/trades/salt.png",
        ]
    );

    me.supply_secs = 11.2;
    me.mounts[2] = None;
    assert_eq!(mount_views(&welcome, &me)[2].icon, None);
    assert_eq!(seen(&me)[1], text("kit", "wait 12s", true));
    assert_eq!(seen(&me)[2], text("-", "", false));
}

#[test]
fn a_bomb_flies_in_an_arc_to_the_place_it_was_aimed_at() {
    // 폭탄(무기 1번)은 20번째 Tick에 (14, 10)을 겨누고 쏘았습니다. 표적은 그 뒤에 (18, 10)으로 갔습니다.
    let mut welcome = welcome();
    welcome.weapons.push(WireWeapon {
        name: "bomb".to_string(),
        damage: 60,
        shot: WireShot::Bomb { radius: 2.0 },
        ..welcome.weapons[0].clone()
    });
    let mount = WireMount {
        slot: 0,
        item: WireItem::Weapon(1),
        target: Some(WireTarget::Ship(ENEMY)),
        aim: Some(WireVec2::new(14.0, 10.0)),
        ready_at: 30,
    };
    let armed = WireShip {
        mounts: vec![mount],
        ..ship(ME, 10.0)
    };
    let room = |tick| snapshot(tick, vec![armed.clone(), ship(ENEMY, 18.0)]);
    let mut session = Session::new(Some("test"), "KANG", None);
    session.handle(ClientEvent::Connected, 0.0);
    session.handle(
        ClientEvent::Message(ServerMsg::Welcome(Box::new(welcome))),
        0.0,
    );
    receive(&mut session, 1.00, room(21));
    receive(&mut session, 1.05, room(22));

    // 22번째 Tick에 폭탄은 4타일의 한가운데, 포물선의 가장 높은 곳에 있습니다.
    let frame = session.frame(1.15).unwrap();
    let bomb = frame.shots[0];
    assert!(
        (bomb.pos.x - 12.0).abs() < 1e-3,
        "it flies to the aim, not the ship"
    );
    assert_eq!(bomb.kind, WireShot::Bomb { radius: 2.0 });
    assert!((bomb.lift - 3.0).abs() < 1e-3);
    assert_eq!(bomb.size, 2.1, "a heavy shot is drawn larger");
    assert_eq!(frame.blasts, []);

    // 폭탄은 24번째 Tick에 떨어지고, 그 뒤 8Tick 동안 터진 범위가 겨눈 자리에 보입니다.
    receive(&mut session, 1.20, room(25));
    receive(&mut session, 1.25, room(26));
    let frame = session.frame(1.35).unwrap();
    assert_eq!(frame.shots, []);
    assert_eq!(
        frame.blasts,
        [BlastView {
            pos: Vec2::new(14.0, 10.0),
            team: armed.team,
            radius: 2.0,
            age: 0.25,
        }]
    );
}

#[test]
fn the_ship_panel_shows_hp_the_xp_of_this_level_and_the_skill_slots() {
    // 2레벨의 배가 경험치 150을 가지고 있습니다. 2레벨은 100에서, 3레벨은 300에서 시작합니다.
    // 스킬은 2단계 가운데 1단계이고, 쿨다운 5초 가운데 2초가 남았습니다.
    let mut session = playing();
    let mut me = ship(ME, 10.0);
    me.level = 2;
    let own = me.own.as_mut().unwrap();
    own.xp = 150;
    own.skills[0] = WireSkill {
        rank: 1,
        ready_at: 60,
    };
    receive(&mut session, 1.0, snapshot(20, vec![me]));
    let frame = session.frame(1.15).unwrap();

    let view = hud_view(session.welcome().unwrap(), frame.me.as_ref().unwrap());

    let slot = SkillSlot {
        icon: Icon::new(IconKind::Skill, "skill"),
        key: 'Q',
        name: "skill".to_string(),
        rank: 1,
        max_rank: 2,
        ready_secs: 2.0,
        wait: 0.4,
        learnable: true,
    };
    let expected = HudView {
        hull: "small".to_string(),
        level: 2,
        skill_points: 1,
        hp: 500,
        max_hp: 1000,
        xp: Some((50, 200)),
        skills: vec![slot],
    };
    assert_eq!(view, expected);
    assert_eq!((view.hp_share(), view.xp_share()), (0.5, 0.25));
}

#[test]
fn the_ship_panel_has_a_full_xp_bar_at_the_highest_level() {
    let mut session = playing();
    let mut me = ship(ME, 10.0);
    me.level = 3;
    me.own.as_mut().unwrap().xp = 900;
    receive(&mut session, 1.0, snapshot(20, vec![me]));
    let frame = session.frame(1.15).unwrap();

    let view = hud_view(session.welcome().unwrap(), frame.me.as_ref().unwrap());

    assert_eq!(view.xp, None);
    assert_eq!(view.xp_share(), 1.0);
}

#[test]
fn the_top_bar_shows_the_clock_and_the_kills_of_each_team() {
    let score = |ship, team, kills| WireScore {
        ship,
        team,
        level: 1,
        kills,
        deaths: 0,
        creep_kills: 0,
    };
    let scores = [
        score(1, WireTeam::Red, 2),
        score(2, WireTeam::Blue, 5),
        score(3, WireTeam::Red, 1),
    ];

    let bar = top_bar(754.9, &scores);

    let expected = TopBar {
        clock: "12:34".to_string(),
        red_kills: 3,
        blue_kills: 5,
    };
    assert_eq!(bar, expected);
}
