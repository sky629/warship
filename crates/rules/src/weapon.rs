//! 자동 사격입니다. 배의 무기와 타워의 포는 같은 규칙으로 쏩니다.
//!
//! 쏠 수 있는 무기는 사거리 안에 있고 진영에 보이는 적 가운데 가장 가까운 적을 쏩니다. 플레이어는 표적을 고르지 않습니다.
//! 포탄은 개체로 만들지 않습니다. 쏜 순간에 포탄이 닿을 Tick을 정해서 `Impact`로 적어 두고, 그 Tick에 피해를 줍니다.
//! 쏜 포탄은 표적을 따라가므로 표적이 사거리 밖으로 나가도 맞고, 표적이 그 전에 사라지면 피해가 없습니다.

use crate::ids::{ShipId, Team, TowerId};
use crate::math::Vec2;
use crate::tuning::{Hits, Tuning, WeaponSpec};
use crate::vision::Sight;
use crate::world::World;

/// 무기가 쏠 수 있는 개체입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Ship(ShipId),
    Tower(TowerId),
}

/// 쏘았지만 아직 표적에 닿지 않은 포탄 하나의 피해입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Impact {
    /// 피해가 들어가는 Tick입니다.
    pub due: u64,
    pub target: Target,
    pub damage: u32,
}

/// 쏘는 쪽입니다.
enum Shooter {
    Mount(ShipId, usize),
    Tower(TowerId),
}

/// 쏠 수 있는 무기 하나가 이번 Tick에 내린 결정입니다.
struct Decision<'a> {
    shooter: Shooter,
    spec: &'a WeaponSpec,
    /// 쏠 표적과 그 거리입니다. 쏠 적이 없으면 `None`입니다.
    aim: Option<(Target, f32)>,
}

/// 쏠 수 있는 모든 무기가 표적을 고르고 쏩니다. 모든 무기가 같은 상태를 보고 결정한 뒤 한꺼번에 적용합니다.
pub(crate) fn fire_all(world: &mut World, sight: &Sight, tuning: &Tuning, tick: u64) {
    let mut decisions: Vec<Decision> = Vec::new();
    for ship in world.ships.values().filter(|ship| ship.is_afloat()) {
        for (slot, mount) in ship.mounts.iter().enumerate() {
            let Some(mount) = mount.filter(|mount| mount.ready_at <= tick) else {
                continue;
            };
            let Some(spec) = tuning.weapons.get(usize::from(mount.weapon.0)) else {
                continue;
            };
            let aim = nearest(sight, ship.team, ship.pos, spec);
            decisions.push(Decision {
                shooter: Shooter::Mount(ship.id, slot),
                spec,
                aim,
            });
        }
    }
    for tower in world.towers.values() {
        if tower.hp > 0 && tower.ready_at <= tick {
            let spec = &tuning.tower.weapon;
            let aim = nearest(sight, tower.team, tower.pos, spec);
            decisions.push(Decision {
                shooter: Shooter::Tower(tower.id),
                spec,
                aim,
            });
        }
    }

    for Decision { shooter, spec, aim } in decisions {
        let target = aim.map(|(target, _)| target);
        let ready_at = match aim {
            Some(_) => tick + u64::from(spec.cooldown_ticks),
            None => tick,
        };
        match shooter {
            Shooter::Mount(id, slot) => {
                let mount = world
                    .ships
                    .get_mut(&id)
                    .and_then(|ship| ship.mounts[slot].as_mut());
                if let Some(mount) = mount {
                    mount.target = target;
                    mount.ready_at = ready_at;
                }
            }
            Shooter::Tower(id) => {
                if let Some(tower) = world.towers.get_mut(&id) {
                    tower.target = target;
                    tower.ready_at = ready_at;
                }
            }
        }
        if let Some((target, distance)) = aim {
            world.impacts.push(Impact {
                due: tick + flight_ticks(distance, spec, tuning.tick_hz),
                target,
                damage: spec.damage,
            });
        }
    }
}

/// `team`에 보이고, 무기가 쏠 수 있는 종류이고, 사거리 안에 있는 적 가운데 가장 가까운 적과 그 거리입니다.
/// 거리가 같으면 `World::targets`의 순서에서 앞선 적입니다.
fn nearest(sight: &Sight, team: Team, from: Vec2, spec: &WeaponSpec) -> Option<(Target, f32)> {
    let mut best: Option<(Target, f32)> = None;
    for &(target, pos) in sight.enemies_seen_by(team) {
        if spec.hits == Hits::ShipsOnly && matches!(target, Target::Tower(_)) {
            continue;
        }
        let distance = from.distance(pos);
        if distance <= spec.range && best.is_none_or(|(_, nearest)| distance < nearest) {
            best = Some((target, distance));
        }
    }
    best
}

/// 포탄이 `distance`를 날아가는 데 걸리는 Tick 수입니다. 가장 가까운 표적에도 한 Tick은 걸립니다.
fn flight_ticks(distance: f32, spec: &WeaponSpec, tick_hz: u32) -> u64 {
    let per_tick = spec.shot_speed / tick_hz as f32;
    ((distance / per_tick).ceil() as u64).max(1)
}

/// 이번 Tick까지 닿을 포탄의 피해를 쏜 순서대로 줍니다.
pub(crate) fn land_impacts(world: &mut World, tick: u64) {
    let (due, flying): (Vec<Impact>, Vec<Impact>) = std::mem::take(&mut world.impacts)
        .into_iter()
        .partition(|impact| impact.due <= tick);
    world.impacts = flying;
    for impact in due {
        world.damage(impact.target, impact.damage);
    }
}
