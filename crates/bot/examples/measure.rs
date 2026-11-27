//! 수치의 균형을 재는 도구입니다. 봇끼리 5대5 대전을 한 판 돌리고, 교역선 한 척이 교역만 하는 방을 교역마다 돌립니다.
//!
//! 실행: `cargo run -p warship_bot --example measure`
//! 대전에서는 건물이 파괴된 시각, 정해진 시각마다의 레벨과 골드, 대전이 끝난 시각, 배 한 척이 1분에 번 골드를 출력합니다.
//! 교역에서는 교역마다 배 한 척이 1분에 남긴 이익을 출력합니다. 두 값을 비교해서 교역의 보상을 정합니다.
//! 마지막으로 같은 값의 선체 가운데 첫째만 쓰는 진영과 둘째만 쓰는 진영의 대전을 돌려서, 선체 계열끼리의 균형을 잽니다.
//! 규칙에 무작위가 없으므로 같은 수치에서는 항상 같은 결과가 나옵니다.

use std::collections::BTreeMap;

use warship_bot::plan;
use warship_rules::{
    Command, HullId, Item, PlayerInput, PlayerInputs, ShipId, Sim, StructureId, StructureKind,
    Team, TradeId, Tuning, Waypoint,
};

/// 봇이 이 Tick 수마다 한 번 계획을 세웁니다. 서버와 같은 값입니다.
const THINK_TICKS: u64 = 10;
/// 이 시간(분)이 지나도 끝나지 않으면 멈춥니다.
const LIMIT_MINUTES: u64 = 90;
/// 이 시간(분)마다 배의 상태를 출력합니다.
const REPORT_MINUTES: u64 = 5;
/// 교역선 한 척이 교역만 하는 시간(분)입니다.
const TRADE_MINUTES: u64 = 10;

fn clock(tick: u64, tick_hz: u32) -> String {
    let secs = tick / u64::from(tick_hz);
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn team_name(team: Team) -> &'static str {
    if team == Team::RED { "red" } else { "blue" }
}

fn main() {
    let (outcome, tick, per_ship) = battle([None, None], true);
    let tick_hz = Tuning::default().tick_hz;
    println!("{}  end: {outcome:?}", clock(tick, tick_hz));
    println!("a fighting ship earns {per_ship:.0} gold a minute (income and bounties)");

    let trades = Tuning::default().trades.len();
    for trade in 0..trades {
        trade_run(TradeId(trade as u16));
    }

    // 진영마다 같은 값의 선체 가운데 몇째를 쓰는지입니다. 자리를 바꾼 대전도 돌려서 진영의 차이를 뺍니다.
    for lines in [[0, 0], [0, 1], [1, 0], [1, 1]] {
        let (outcome, tick, _) = battle(lines.map(Some), false);
        println!(
            "hull lines red {} blue {}: {outcome:?} at {}",
            lines[0],
            lines[1],
            clock(tick, tick_hz)
        );
    }
}

/// 봇이 고른 선체를, 그 값의 전투용 선체 가운데 `line`째 선체로 바꿉니다. 그 값의 선체가 `line`개보다 적으면 마지막 선체입니다.
fn on_line(tuning: &Tuning, command: Command, line: Option<usize>) -> Command {
    let (Command::ChangeHull(hull), Some(line)) = (command, line) else {
        return command;
    };
    let price = tuning.hull(hull).price;
    let same: Vec<usize> = (0..tuning.hulls.len())
        .filter(|index| !tuning.hulls[*index].trader && tuning.hulls[*index].price == price)
        .collect();
    let index = same[line.min(same.len() - 1)];
    Command::ChangeHull(HullId(index as u16))
}

/// 봇끼리 5대5 대전을 한 판 돌리고, 결과와 끝난 Tick과 배 한 척이 1분에 번 골드를 돌려줍니다.
/// `lines`는 진영마다 쓰는 선체 계열이고, 없으면 봇이 고른 선체를 그대로 씁니다. `verbose`이면 진행을 출력합니다.
fn battle(lines: [Option<usize>; 2], verbose: bool) -> (Option<warship_rules::Outcome>, u64, f32) {
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
    // 배마다 지금까지 번 골드입니다. 골드가 늘어난 양만 더하므로 산 것은 빼지 않습니다.
    let mut earned: BTreeMap<ShipId, u32> = BTreeMap::new();

    while sim.outcome().is_none() && sim.tick() < LIMIT_MINUTES * per_minute {
        let mut inputs = PlayerInputs::new();
        if sim.tick().is_multiple_of(THINK_TICKS) {
            for &id in &ships {
                let plan = plan(&sim, id);
                seq += 1;
                let destination = plan.destination.and_then(|p| Waypoint::new(p.x, p.y));
                inputs.insert(id, PlayerInput { seq, destination });
                let line = sim.ship(id).and_then(|ship| lines[ship.team.index()]);
                for command in plan.commands {
                    seq += 1;
                    let command = on_line(sim.tuning(), command, line);
                    sim.command(id, seq, command);
                }
            }
        }
        let before: Vec<u32> = ships
            .iter()
            .map(|id| sim.ship(*id).map_or(0, |s| s.gold))
            .collect();
        sim.step(&inputs);
        for (id, before) in ships.iter().zip(before) {
            let after = sim.ship(*id).map_or(0, |ship| ship.gold);
            *earned.entry(*id).or_default() += after.saturating_sub(before);
        }

        let fallen: Vec<StructureId> = standing
            .keys()
            .copied()
            .filter(|id| sim.structure(*id).is_none())
            .collect();
        for id in fallen {
            if let Some((kind, team)) = standing.remove(&id)
                && verbose
            {
                let now = clock(sim.tick(), tick_hz);
                println!("{now}  {} {kind:?} destroyed", team_name(team));
            }
        }
        if verbose && sim.tick().is_multiple_of(REPORT_MINUTES * per_minute) {
            report(&sim);
        }
    }
    if verbose {
        report(&sim);
    }
    let minutes = sim.tick() as f32 / per_minute as f32;
    let total: u32 = earned.values().sum();
    let per_ship = total as f32 / ships.len().max(1) as f32 / minutes;
    (sim.outcome(), sim.tick(), per_ship)
}

/// 교역선 한 척이 `trade`만 되풀이하는 방을 돌리고, 1분에 남긴 이익(보상에서 계약의 값을 뺀 것)을 출력합니다.
/// 배는 골드가 모이면 가장 싼 교역선으로 바꾸고, 계약을 사서 교역항에 갔다가 본진으로 돌아옵니다. 적의 배는 없고 NPC 배는 있습니다.
fn trade_run(trade: TradeId) {
    let tuning = Tuning::default();
    let per_minute = 60 * u64::from(tuning.tick_hz);
    let spec = tuning.trades[usize::from(trade.0)].clone();
    let port = tuning.map.ports[spec.port];
    let trader = tuning.hulls.iter().position(|hull| hull.trader);
    let Some(trader) = trader.map(|index| HullId(index as u16)) else {
        return;
    };
    let mut sim = Sim::new(tuning);
    let Ok(id) = sim.join() else { return };
    let base = sim.tuning().map.base_of(Team::RED);
    let (mut seq, mut trips, mut sunk, mut first) = (0, 0u32, 0u32, None);
    let mut afloat = true;
    while sim.tick() < TRADE_MINUTES * per_minute {
        let Some(ship) = sim.ship(id) else { return };
        let holds = |wanted: fn(&Item) -> bool| {
            let mut items = ship.mounts.iter().flatten().map(|mount| mount.item);
            items.any(|item| wanted(&item))
        };
        let contract = holds(|item| matches!(item, Item::Contract(_)));
        let cargo = holds(|item| matches!(item, Item::Cargo(_)));
        if afloat && !ship.is_afloat() {
            sunk += 1;
        }
        afloat = ship.is_afloat();
        let command = if ship.hull != trader {
            Some(Command::ChangeHull(trader))
        } else if !contract && !cargo {
            Some(Command::Contract(trade))
        } else {
            None
        };
        let destination = if contract { Some(port) } else { base };
        let mut inputs = PlayerInputs::new();
        if sim.tick().is_multiple_of(THINK_TICKS) {
            seq += 1;
            let destination = destination.and_then(|p| Waypoint::new(p.x, p.y));
            inputs.insert(id, PlayerInput { seq, destination });
            if let Some(command) = command.filter(|_| sim.at_dock(id)) {
                seq += 1;
                sim.command(id, seq, command);
            }
        }
        sim.step(&inputs);
        let unloaded = cargo
            && sim.ship(id).is_some_and(|ship| {
                let mut items = ship.mounts.iter().flatten().map(|mount| mount.item);
                ship.is_afloat() && !items.any(|item| matches!(item, Item::Cargo(_)))
            });
        if unloaded {
            trips += 1;
            first.get_or_insert(sim.tick());
        }
    }
    let profit = trips * (spec.reward - spec.price);
    let first = first.map_or("never".to_string(), |tick| {
        clock(tick, sim.tuning().tick_hz)
    });
    println!(
        "trade {:<7} {trips} trips in {TRADE_MINUTES} minutes, first delivery at {first}, sunk {sunk} times: {} gold a minute",
        spec.name,
        profit as f32 / TRADE_MINUTES as f32,
    );
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
