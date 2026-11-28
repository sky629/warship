//! 자동 사격입니다. 배의 무기, NPC 배의 포, 건물의 포는 같은 규칙으로 쏩니다.
//!
//! 쏠 수 있는 무기는 사거리 안에 있고 진영에 보이는 적 가운데 가장 가까운 적을 쏩니다. 플레이어는 표적을 고르지 않습니다.
//! 포탄은 개체로 만들지 않습니다. 쏜 순간에 포탄이 닿을 Tick을 정해서 `Impact`로 적어 두고, 그 Tick에 피해를 줍니다.
//! 쏜 포탄은 표적을 따라가므로 표적이 사거리 밖으로 나가도 맞고, 표적이 그 전에 사라지면 피해가 없습니다.
//! 무기의 공격 방식(`Shot`)에 따라, 닿을 때 둘레의 적을 함께 맞히거나, 쏠 때 표적이 있던 자리에 떨어지거나, 맞은 뒤에도 피해를 더 줍니다.

use crate::ids::{CreepId, ShipId, StructureId, Team};
use crate::math::Vec2;
use crate::note::NoteKind;
use crate::tuning::{Hits, Shot, Tuning, WeaponSpec};
use crate::vision::Sight;
use crate::world::World;

/// 무기가 쏠 수 있는 개체입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Ship(ShipId),
    Creep(CreepId),
    Structure(StructureId),
}

/// 쏘았지만 아직 표적에 닿지 않은 포탄 하나의 피해입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Impact {
    /// 피해가 들어가는 Tick입니다.
    pub due: u64,
    pub target: Target,
    pub damage: u32,
    /// 포탄을 쏜 플레이어의 배입니다. NPC 배와 건물이 쏜 포탄에는 없습니다.
    pub by: Option<ShipId>,
    /// 둘레의 적을 함께 맞히는 포탄이면 그 범위입니다.
    pub blast: Option<Blast>,
    /// 맞은 배에 불이 붙어 있는 Tick 수입니다. 불을 붙이지 않는 포탄은 0입니다.
    pub burn_ticks: u64,
}

/// 포탄이 닿을 때 피해를 받는 범위입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Blast {
    /// 범위의 가운데입니다. 없으면 포탄이 닿을 때 표적이 있는 자리입니다.
    pub center: Option<Vec2>,
    pub radius: f32,
    /// 포탄을 쏜 쪽의 진영입니다. 이 진영의 적만 피해를 받습니다.
    pub team: Team,
    /// 건물도 피해를 받는지입니다.
    pub hits: Hits,
}

/// 포탄 하나가 표적의 HP를 0으로 만든 일입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Kill {
    pub target: Target,
    pub by: Option<ShipId>,
}

/// 쏘는 쪽입니다.
enum Shooter {
    Mount(ShipId, usize),
    Creep(CreepId),
    Structure(StructureId),
}

/// 쏠 수 있는 무기 하나가 이번 Tick에 내린 결정입니다.
struct Decision<'a> {
    shooter: Shooter,
    spec: &'a WeaponSpec,
    /// 쏘는 쪽의 진영입니다.
    team: Team,
    /// 쏠 표적, 그 거리, 표적이 있는 자리입니다. 쏠 적이 없으면 `None`입니다.
    aim: Option<(Target, f32, Vec2)>,
}

/// 무기가 이 종류의 표적을 쏠 수 있는지입니다.
pub(crate) fn can_hit(spec: &WeaponSpec, target: Target) -> bool {
    spec.hits == Hits::ShipsAndStructures || !matches!(target, Target::Structure(_))
}

/// 쏠 수 있는 모든 무기가 표적을 고르고 쏩니다. 모든 무기가 같은 상태를 보고 결정한 뒤 한꺼번에 적용합니다.
pub(crate) fn fire_all(world: &mut World, sight: &Sight, tuning: &Tuning, tick: u64) {
    let mut decisions: Vec<Decision> = Vec::new();
    let mut decide = |shooter, spec, team, pos| {
        let aim = nearest(sight, team, pos, spec);
        decisions.push(Decision {
            shooter,
            spec,
            team,
            aim,
        });
    };
    for ship in world.ships.values().filter(|ship| ship.is_afloat()) {
        for (slot, mount) in ship.mounts.iter().enumerate() {
            let Some(mount) = mount.filter(|mount| mount.ready_at <= tick) else {
                continue;
            };
            let weapon = mount.weapon();
            if let Some(spec) = weapon.and_then(|weapon| tuning.weapons.get(usize::from(weapon.0)))
            {
                decide(Shooter::Mount(ship.id, slot), spec, ship.team, ship.pos);
            }
        }
    }
    for creep in world.creeps.values() {
        if creep.hp > 0 && creep.ready_at <= tick {
            let spec = &tuning.creeps[creep.kind].weapon;
            decide(Shooter::Creep(creep.id), spec, creep.team, creep.pos);
        }
    }
    for structure in world.structures.values() {
        if structure.hp > 0
            && structure.ready_at <= tick
            && let Some(spec) = &tuning.structures.of(structure.kind).weapon
        {
            let shooter = Shooter::Structure(structure.id);
            decide(shooter, spec, structure.team, structure.pos);
        }
    }

    for Decision {
        shooter,
        spec,
        team,
        aim,
    } in decisions
    {
        let target = aim.map(|(target, _, _)| target);
        let ready_at = match aim {
            Some(_) => tick + u64::from(spec.cooldown_ticks),
            None => tick,
        };
        let by = match shooter {
            Shooter::Mount(id, slot) => {
                let mount = world
                    .ships
                    .get_mut(&id)
                    .and_then(|ship| ship.mounts[slot].as_mut());
                if let Some(mount) = mount {
                    mount.target = target;
                    mount.ready_at = ready_at;
                    mount.aim = aim.map(|(_, _, at)| at);
                }
                // 포를 쏜 배는 은신이 풀립니다.
                if let Some(ship) = world.ships.get_mut(&id).filter(|_| target.is_some()) {
                    ship.cloaked_until = 0;
                }
                Some(id)
            }
            Shooter::Creep(id) => {
                if let Some(creep) = world.creeps.get_mut(&id) {
                    creep.target = target;
                    creep.ready_at = ready_at;
                }
                None
            }
            Shooter::Structure(id) => {
                if let Some(structure) = world.structures.get_mut(&id) {
                    structure.target = target;
                    structure.ready_at = ready_at;
                }
                None
            }
        };
        if let Some((target, distance, at)) = aim {
            let due = tick + flight_ticks(distance, spec, tuning.tick_hz);
            let blast = |center, radius| Blast {
                center,
                radius,
                team,
                hits: spec.hits,
            };
            let blast = match spec.shot {
                Shot::Shell | Shot::Fire { .. } => None,
                Shot::Scatter { radius } => Some(blast(None, radius)),
                Shot::Bomb { radius } => Some(blast(Some(at), radius)),
            };
            world.impacts.push(Impact {
                due,
                target,
                damage: spec.damage,
                by,
                blast,
                burn_ticks: match spec.shot {
                    Shot::Fire { secs, .. } => u64::from(secs) * u64::from(tuning.tick_hz),
                    _ => 0,
                },
            });
            // 불은 포탄이 닿은 뒤 1초마다 표적에 피해를 줍니다.
            if let Shot::Fire {
                damage_per_sec,
                secs,
            } = spec.shot
            {
                let burns = (1..=u64::from(secs)).map(|sec| Impact {
                    due: due + sec * u64::from(tuning.tick_hz),
                    target,
                    damage: damage_per_sec,
                    by,
                    blast: None,
                    burn_ticks: 0,
                });
                world.impacts.extend(burns);
            }
        }
    }
}

/// `team`에 보이고, 무기가 쏠 수 있는 종류이고, 사거리 안에 있는 적 가운데 가장 가까운 적과 그 거리입니다.
/// 거리가 같으면 `World::targets`의 순서에서 앞선 적입니다.
fn nearest(
    sight: &Sight,
    team: Team,
    from: Vec2,
    spec: &WeaponSpec,
) -> Option<(Target, f32, Vec2)> {
    let mut best: Option<(Target, f32, Vec2)> = None;
    for &(target, pos) in sight.enemies_seen_by(team) {
        if !can_hit(spec, target) {
            continue;
        }
        let distance = from.distance(pos);
        if distance <= spec.range && best.is_none_or(|(_, nearest, _)| distance < nearest) {
            best = Some((target, distance, pos));
        }
    }
    best
}

/// 포탄이 `distance`를 날아가는 데 걸리는 Tick 수입니다. 가장 가까운 표적에도 한 Tick은 걸립니다.
fn flight_ticks(distance: f32, spec: &WeaponSpec, tick_hz: u32) -> u64 {
    let per_tick = spec.shot_speed / tick_hz as f32;
    ((distance / per_tick).ceil() as u64).max(1)
}

/// 이번 Tick까지 닿을 포탄의 피해를 쏜 순서대로 주고, HP를 0으로 만든 포탄을 돌려줍니다.
/// 이번 Tick에 가라앉은 배는 `respawn_ticks` 뒤에 다시 나타납니다.
pub(crate) fn land_impacts(
    world: &mut World,
    sight: &Sight,
    tuning: &Tuning,
    tick: u64,
) -> Vec<Kill> {
    let (due, flying): (Vec<Impact>, Vec<Impact>) = std::mem::take(&mut world.impacts)
        .into_iter()
        .partition(|impact| impact.due <= tick);
    world.impacts = flying;
    let respawn_at = tick + u64::from(tuning.ship.respawn_ticks);
    let mut kills = Vec::new();
    for impact in due {
        for victim in victims(world, &impact) {
            let damage = match victim {
                Target::Ship(id) => world.ships.get(&id).map_or(impact.damage, |ship| {
                    ship.braced(tuning, impact.damage, tick)
                }),
                _ => impact.damage,
            };
            if let Target::Ship(id) = victim
                && impact.burn_ticks > 0
                && let Some(ship) = world.ships.get_mut(&id)
            {
                ship.burning_until = tick + impact.burn_ticks;
            }
            // 이미 사라졌거나 가라앉은 표적은 피해를 받지 않으므로 적지 않습니다.
            let hit = world.targets().find(|(target, ..)| *target == victim);
            if let (Target::Ship(id), Some(by), Some(_)) = (victim, impact.by, hit)
                && let Some(ship) = world.ships.get_mut(&id)
            {
                ship.wound(by, damage.min(ship.hp), tuning, tick);
            }
            if let Some((_, _, pos)) = hit {
                let kind = NoteKind::Hit {
                    by: impact.by,
                    target: victim,
                    seen: Team::ALL.map(|team| sight.sees(team, victim)),
                };
                world.note(tick, pos, damage, kind);
            }
            if world.damage(victim, damage, respawn_at) {
                kills.push(Kill {
                    target: victim,
                    by: impact.by,
                });
            }
        }
    }
    kills
}

/// 닿은 포탄이 피해를 주는 개체입니다. 범위가 없는 포탄은 표적 하나이고, 범위가 있는 포탄은 그 범위 안에 있는 쏜 쪽의 적입니다.
/// 범위의 가운데가 표적의 자리인데 표적이 이미 사라졌으면 아무도 맞지 않습니다.
fn victims(world: &World, impact: &Impact) -> Vec<Target> {
    let Some(blast) = impact.blast else {
        return vec![impact.target];
    };
    let place = |wanted: Target| {
        let mut targets = world.targets();
        targets
            .find(|(target, _, _)| *target == wanted)
            .map(|(_, _, pos)| pos)
    };
    let Some(center) = blast.center.or_else(|| place(impact.target)) else {
        return Vec::new();
    };
    let hit = world.targets().filter(|(target, team, pos)| {
        let kind_fits =
            blast.hits == Hits::ShipsAndStructures || !matches!(target, Target::Structure(_));
        team.is_hostile_to(blast.team) && kind_fits && pos.distance(center) <= blast.radius
    });
    hit.map(|(target, _, _)| target).collect()
}
