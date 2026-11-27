//! 봇끼리 5대5 대전을 한 판 돌려서 수치의 균형을 재는 도구입니다.
//!
//! 실행: `cargo run -p warship_bot --example measure`
//! 건물이 파괴된 시각, 정해진 시각마다의 레벨과 골드, 대전이 끝난 시각을 출력합니다.
//! 규칙에 무작위가 없으므로 같은 수치에서는 항상 같은 결과가 나옵니다.

use std::collections::BTreeMap;

use warship_bot::plan;
use warship_rules::{
    PlayerInput, PlayerInputs, ShipId, Sim, StructureId, StructureKind, Team, Tuning, Waypoint,
};

/// 봇이 이 Tick 수마다 한 번 계획을 세웁니다. 서버와 같은 값입니다.
const THINK_TICKS: u64 = 10;
/// 이 시간(분)이 지나도 끝나지 않으면 멈춥니다.
const LIMIT_MINUTES: u64 = 90;
/// 이 시간(분)마다 배의 상태를 출력합니다.
const REPORT_MINUTES: u64 = 5;

fn clock(tick: u64, tick_hz: u32) -> String {
    let secs = tick / u64::from(tick_hz);
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn team_name(team: Team) -> &'static str {
    if team == Team::RED { "red" } else { "blue" }
}

fn main() {
    let tuning = Tuning::default();
    let tick_hz = tuning.tick_hz;
    let mut sim = Sim::new(tuning);
    let ships: Vec<ShipId> = (0..10).filter_map(|_| sim.join().ok()).collect();
    let mut standing: BTreeMap<StructureId, (StructureKind, Team)> = sim
        .structures()
        .map(|site| (site.id, (site.kind, site.team)))
        .collect();
    let mut seq = 0;
    let per_minute = 60 * u64::from(tick_hz);

    while sim.outcome().is_none() && sim.tick() < LIMIT_MINUTES * per_minute {
        let mut inputs = PlayerInputs::new();
        if sim.tick().is_multiple_of(THINK_TICKS) {
            for &id in &ships {
                let plan = plan(&sim, id);
                seq += 1;
                let destination = plan.destination.and_then(|p| Waypoint::new(p.x, p.y));
                inputs.insert(id, PlayerInput { seq, destination });
                for command in plan.commands {
                    seq += 1;
                    sim.command(id, seq, command);
                }
            }
        }
        sim.step(&inputs);

        let fallen: Vec<StructureId> = standing
            .keys()
            .copied()
            .filter(|id| sim.structure(*id).is_none())
            .collect();
        for id in fallen {
            if let Some((kind, team)) = standing.remove(&id) {
                let now = clock(sim.tick(), tick_hz);
                println!("{now}  {} {kind:?} destroyed", team_name(team));
            }
        }
        if sim.tick().is_multiple_of(REPORT_MINUTES * per_minute) {
            report(&sim);
        }
    }
    println!("{}  end: {:?}", clock(sim.tick(), tick_hz), sim.outcome());
    report(&sim);
}

/// 진영마다 배의 레벨, 선체, 무기의 수, 골드의 평균과 배가 있는 자리, 남은 건물의 HP를 출력합니다.
fn report(sim: &Sim) {
    let tuning = sim.tuning();
    for team in Team::ALL {
        let sites = sim.structures().filter(|site| site.team == team);
        let hps: Vec<String> = sites
            .map(|site| format!("{:?} {}", site.kind, site.hp))
            .collect();
        let xs: Vec<String> = sim
            .ships()
            .filter(|ship| ship.team == team)
            .map(|ship| format!("{:.0}", ship.pos.x))
            .collect();
        let creeps = sim.creeps().filter(|creep| creep.team == team).count();
        println!(
            "       {:<4} ships at x {}  creeps {creeps}  {}",
            team_name(team),
            xs.join(" "),
            hps.join(", ")
        );
        let ships: Vec<_> = sim.ships().filter(|ship| ship.team == team).collect();
        let count = ships.len().max(1) as f32;
        let mean = |total: u32| total as f32 / count;
        let level: u32 = ships.iter().map(|ship| ship.level(tuning)).sum();
        let hull: u32 = ships.iter().map(|ship| u32::from(ship.hull.0)).sum();
        let weapons = ships
            .iter()
            .map(|ship| ship.mounts.iter().flatten().count());
        let gold: u32 = ships.iter().map(|ship| ship.gold).sum();
        println!(
            "{}  {:<4} level {:.1}  hull {:.1}  weapons {:.1}  gold {:.0}",
            clock(sim.tick(), tuning.tick_hz),
            team_name(team),
            mean(level),
            mean(hull),
            mean(weapons.sum::<usize>() as u32),
            mean(gold),
        );
    }
}
