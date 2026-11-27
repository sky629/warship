//! 빈자리를 채우는 봇입니다. 자기 진영에 보이는 상태만 읽고, 사람과 같은 입력(이동 지점과 명령)을 만듭니다.
//!
//! 봇은 기억을 가지지 않습니다. `plan`은 지금의 방 상태만 보고 결정하므로, 같은 상태에서는 항상 같은 계획이 나옵니다.
//!
//! 봇이 하는 일은 네 가지입니다.
//! - 스킬 포인트가 있으면 지금 선체의 스킬 가운데 단계가 가장 낮은 스킬에 넣습니다.
//! - 사고 싶은 것(조합, 빈 칸에 달 무기나 장비, 다음 선체)의 값이 모이면 상점으로 가서 삽니다.
//! - HP가 적으면 본진으로 돌아가서 수리를 받습니다.
//! - 그 밖에는 맡은 라인에서 가장 앞선 아군 NPC 배와 함께 싸웁니다. 아군 NPC 배가 없으면 그 라인의 적 건물을 치러 갑니다.

use warship_rules::{
    Command, GearId, HullId, RecipeId, Ship, ShipId, Sim, SkillEffect, StructureKind, Target, Team,
    Tuning, Vec2, WeaponId,
};

/// HP가 최대 HP의 이 백분율보다 적으면 본진으로 돌아갑니다.
const RETREAT_PERCENT: u32 = 35;
/// 본진에서 수리를 받는 동안 HP가 이 백분율이 될 때까지 떠나지 않습니다.
const REJOIN_PERCENT: u32 = 90;
/// HP가 이 백분율보다 적으면 수리 스킬을 씁니다.
const REPAIR_PERCENT: u32 = 60;
/// 라인의 입구에서 남북으로 이 거리 안에 있는 건물을 그 라인의 건물로 봅니다.
const LANE_WIDTH: f32 = 4.0;

/// 봇이 이번에 하려는 일입니다.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// 배가 향할 지점입니다. 없으면 멈춥니다.
    pub destination: Option<Vec2>,
    pub commands: Vec<Command>,
}

/// `id` 배의 계획입니다. 방에 없는 배의 계획은 비어 있습니다.
pub fn plan(sim: &Sim, id: ShipId) -> Plan {
    let Some(ship) = sim.ship(id) else {
        return Plan::default();
    };
    let tuning = sim.tuning();
    let mut commands = Vec::new();
    // 지금 선체의 스킬과 그 수치, 배의 상태입니다.
    let skills = tuning.hull(ship.hull).skills.iter().map(|&id| {
        let index = usize::from(id.0);
        (id, &tuning.skills[index], ship.skills[index])
    });
    if ship.skill_points(tuning) > 0 {
        let lowest = skills
            .clone()
            .filter(|(_, spec, state)| state.rank < spec.max_rank)
            .min_by_key(|(_, _, state)| state.rank);
        if let Some((id, _, _)) = lowest {
            commands.push(Command::Learn(id));
        }
    }
    if !ship.is_afloat() {
        return Plan {
            destination: None,
            commands,
        };
    }

    let max_hp = ship.max_hp(tuning);
    let below = |percent: u32| ship.hp * 100 < max_hp * percent;
    let hurt = below(RETREAT_PERCENT) || (sim.at_dock(id) && below(REJOIN_PERCENT));
    let purchase = wish(ship, tuning).filter(|(_, cost)| ship.gold >= *cost);
    if sim.at_shop(id)
        && let Some((command, _)) = purchase
    {
        commands.push(command);
    }
    for (id, spec, state) in skills {
        let useful = match spec.effect {
            SkillEffect::Repair { .. } | SkillEffect::Brace { .. } => below(REPAIR_PERCENT),
            SkillEffect::Salvo { radius, .. } => enemy_within(sim, ship, radius),
            SkillEffect::Sprint { .. } => hurt,
            SkillEffect::Reload => enemy_within(sim, ship, reach(ship, tuning)),
        };
        if useful && state.rank > 0 && state.ready_at <= sim.tick() {
            commands.push(Command::Cast(id));
        }
    }
    // 수리는 본진에서만 받습니다. 본진이 없는 맵에서는 물러나지 않습니다.
    let dock = tuning.map.base_of(ship.team).filter(|_| hurt);
    let destination = if dock.is_some() {
        dock
    } else if purchase.is_some() {
        nearest_shop(sim, ship)
    } else {
        front(sim, ship)
    };
    Plan {
        destination,
        commands,
    }
}

/// 봇이 다음에 사려는 것과 그 값입니다. 다음 순서로 고릅니다. 더 살 것이 없으면 없습니다.
/// 1. 재료를 모두 단 조합이 있으면 조합합니다.
/// 2. 빈 칸이 있으면 아이템을 삽니다. 칸 셋마다 둘은 무기, 하나는 장비입니다.
///    무기와 장비는 이미 단 수에 따라 표를 돌아가며 골라서, 여러 종류를 섞어 답니다.
/// 3. 다음 선체로 바꿉니다.
fn wish(ship: &Ship, tuning: &Tuning) -> Option<(Command, u32)> {
    let hull = tuning.hull(ship.hull);
    let mounted = || ship.mounts.iter().flatten();
    let carried: Vec<WeaponId> = mounted().filter_map(|mount| mount.weapon()).collect();
    let gear = mounted().filter_map(|mount| mount.gear()).count();
    let ready = tuning.recipes.iter().position(|recipe| {
        let needed = |part: &WeaponId| recipe.parts.iter().filter(|p| *p == part).count();
        let held = |part: &WeaponId| carried.iter().filter(|w| *w == part).count();
        recipe.parts.iter().all(|part| held(part) >= needed(part))
    });
    if let Some(index) = ready {
        let recipe = RecipeId(index as u16);
        return Some((Command::Combine(recipe), tuning.recipes[index].price));
    }
    let for_sale: Vec<usize> = (0..tuning.weapons.len())
        .filter(|index| tuning.weapons[*index].buyable)
        .collect();
    let free = carried.len() + gear < hull.mounts;
    // 무기 둘에 장비 하나의 비율을 넘지 않는 동안에는 장비를 삽니다.
    let wants_gear = !tuning.gear.is_empty() && gear * 2 < carried.len();
    if free && wants_gear {
        let index = gear % tuning.gear.len();
        return Some((
            Command::BuyGear(GearId(index as u16)),
            tuning.gear[index].price,
        ));
    }
    if free && !for_sale.is_empty() {
        let index = for_sale[carried.len() % for_sale.len()];
        let weapon = WeaponId(index as u16);
        return Some((Command::Buy(weapon), tuning.weapons[index].price));
    }
    let next = HullId(ship.hull.0 + 1);
    let spec = tuning.hulls.get(usize::from(next.0))?;
    let refund = u64::from(hull.price) * u64::from(tuning.shop.refund_percent) / 100;
    let cost = spec.price.saturating_sub(refund as u32);
    Some((Command::ChangeHull(next), cost))
}

/// 배에 달린 무기 가운데 가장 짧은 사거리입니다. 이 거리 안의 적은 모든 무기가 쏠 수 있습니다. 무기가 없으면 0입니다.
fn reach(ship: &Ship, tuning: &Tuning) -> f32 {
    let ranges = ship.mounts.iter().flatten();
    let ranges = ranges
        .filter_map(|mount| mount.weapon())
        .map(|weapon| tuning.weapons[usize::from(weapon.0)].range);
    ranges.reduce(f32::min).unwrap_or(0.0)
}

/// 배의 진영에 보이는 적의 배나 NPC 배가 `radius` 안에 있는지입니다.
fn enemy_within(sim: &Sim, ship: &Ship, radius: f32) -> bool {
    let near = |pos: Vec2| pos.distance(ship.pos) <= radius;
    let hostile = |team: Team| team.is_hostile_to(ship.team);
    let ships = sim.ships().any(|other| {
        hostile(other.team)
            && other.is_afloat()
            && near(other.pos)
            && sim.sees(ship.team, Target::Ship(other.id))
    });
    let mut creeps = sim.creeps();
    ships
        || creeps.any(|creep| {
            hostile(creep.team) && near(creep.pos) && sim.sees(ship.team, Target::Creep(creep.id))
        })
}

/// 배에서 가장 가까운 상점(자기 진영의 서 있는 본진이나 항구)의 자리입니다.
fn nearest_shop(sim: &Sim, ship: &Ship) -> Option<Vec2> {
    let shops = sim
        .structures()
        .filter(|site| site.team == ship.team && site.kind != StructureKind::Tower);
    let nearest = shops.min_by(|a, b| {
        let (a, b) = (a.pos.distance(ship.pos), b.pos.distance(ship.pos));
        a.total_cmp(&b)
    });
    nearest.map(|site| site.pos)
}

/// 배가 싸우러 갈 자리입니다. 맡은 라인에서 적의 본진에 가장 가까운 아군 NPC 배의 자리입니다.
/// 아군 NPC 배가 없으면 그 라인에 서 있는 적의 건물 가운데 가장 가까운 건물로 가고, 그 라인의 건물이 모두 파괴되었으면 적의 본진으로 갑니다.
/// 라인이 없는 맵이면 없습니다.
fn front(sim: &Sim, ship: &Ship) -> Option<Vec2> {
    let map = &sim.tuning().map;
    if map.lanes.is_empty() {
        return None;
    }
    // 진영 안에서 번호가 몇 번째로 작은 배인지로 라인을 나눕니다. 그래서 한 진영의 배가 모든 라인에 고르게 갑니다.
    let ahead = |other: &&Ship| other.team == ship.team && other.id < ship.id;
    let lane = sim.ships().filter(ahead).count() % map.lanes.len();
    let entry = map.path_point(ship.team, lane, 0)?;
    let goal = map.base_of(ship.team.enemy()).unwrap_or(entry);
    let nearest_to =
        |from: Vec2| move |a: &Vec2, b: &Vec2| a.distance(from).total_cmp(&b.distance(from));
    let mine = sim
        .creeps()
        .filter(|creep| creep.team == ship.team && creep.lane == lane);
    let leader = mine.map(|creep| creep.pos).min_by(nearest_to(goal));
    // 라인은 동서로 뻗어 있으므로, 라인의 입구와 남북으로 가까운 건물이 그 라인의 건물입니다.
    let on_lane = sim.structures().filter(|site| {
        site.team != ship.team
            && site.kind != StructureKind::Base
            && (site.pos.y - entry.y).abs() <= LANE_WIDTH
    });
    let objective = on_lane.map(|site| site.pos).min_by(nearest_to(entry));
    Some(leader.or(objective).unwrap_or(goal))
}

#[cfg(test)]
mod tests {
    use warship_rules::{Outcome, PlayerInput, PlayerInputs, Waypoint};

    use super::*;

    /// 봇이 이 Tick 수마다 한 번 계획을 세웁니다.
    const THINK_TICKS: u64 = 10;

    /// 기본 수치의 방에 진영마다 배 다섯 척을 넣습니다. 번호가 홀수인 배가 붉은 진영입니다.
    fn full_room() -> (Sim, Vec<ShipId>) {
        let mut sim = Sim::new(Tuning::default());
        let ships = (0..10).map(|_| sim.join().expect("the room has a free seat"));
        let ships = ships.collect();
        (sim, ships)
    }

    /// `bots`의 배를 봇이 조종하면서 대전이 끝나거나 `ticks`가 지날 때까지 진행합니다. 나머지 배는 가만히 있습니다.
    fn play(sim: &mut Sim, bots: &[ShipId], ticks: u64) {
        let mut seq = 0;
        for _ in 0..ticks {
            let mut inputs = PlayerInputs::new();
            if sim.tick().is_multiple_of(THINK_TICKS) {
                for &id in bots {
                    let plan = plan(sim, id);
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
            if sim.outcome().is_some() {
                return;
            }
        }
    }

    #[test]
    fn a_team_of_bots_beats_a_team_that_does_nothing() {
        let (mut sim, ships) = full_room();
        let red: Vec<ShipId> = ships.iter().copied().step_by(2).collect();

        // 1초는 20Tick이고, 40분 안에 끝나야 합니다.
        play(&mut sim, &red, 48_000);

        assert_eq!(sim.outcome(), Some(Outcome::Victory(Team::RED)));
    }

    #[test]
    fn a_bot_learns_skills_and_spends_its_gold_at_the_shop() {
        let (mut sim, ships) = full_room();

        play(&mut sim, &ships, 6_000);

        for id in ships {
            let ship = sim.ship(id).expect("the ship is in the room");
            let weapons = ship.mounts.iter().flatten().count();
            assert!(weapons >= 2, "ship {id:?} carries {weapons} weapon(s)");
            assert_eq!(ship.skill_points(sim.tuning()), 0);
            assert_eq!(ship.refusal, None, "ship {id:?}");
        }
    }

    #[test]
    fn the_bots_of_a_team_split_over_the_lanes() {
        let (mut sim, ships) = full_room();
        // 첫 NPC 배가 나와서 두 진영이 라인의 가운데에서 만날 때까지 진행합니다.
        play(&mut sim, &ships, 300);

        let middle = sim.tuning().map.height / 2.0;
        for team in Team::ALL {
            let mine = sim.ships().filter(|ship| ship.team == team);
            let north = mine.filter(|ship| ship.pos.y > middle).count();
            assert!(
                (2..=3).contains(&north),
                "{team:?}: {north} of 5 in the north"
            );
        }
    }

    #[test]
    fn a_plan_for_a_ship_that_is_not_in_the_room_is_empty() {
        let (sim, _) = full_room();
        assert_eq!(plan(&sim, ShipId(99)), Plan::default());
    }
}
