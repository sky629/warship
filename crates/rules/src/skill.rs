//! 스킬입니다. 배는 레벨마다 받는 스킬 포인트를 스킬에 넣어서 배우고, 배운 스킬을 쿨다운마다 한 번 씁니다.
//!
//! 모든 스킬은 자기 배를 중심으로 일어나므로 표적을 고르지 않습니다.
//! 배는 지금 선체에 있는 스킬만 배우고 씁니다. 선체를 바꾸면 새 선체에 없는 스킬에 넣은 스킬 포인트가 돌아옵니다.

use crate::command::Refusal;
use crate::ids::{ShipId, SkillId};
use crate::ship::Boost;
use crate::tuning::{SkillEffect, Tuning};
use crate::weapon::{Impact, Target};
use crate::world::World;

/// 스킬 포인트 하나를 스킬에 넣습니다. 가라앉은 배도 할 수 있습니다.
pub(crate) fn learn(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    which: SkillId,
) -> Result<(), Refusal> {
    let index = usize::from(which.0);
    let spec = tuning.skills.get(index).ok_or(Refusal::UnknownItem)?;
    let ship = world.ships.get_mut(&id).ok_or(Refusal::Sunk)?;
    if !ship.has_skill(tuning, which) {
        return Err(Refusal::NotOnThisHull);
    }
    if ship.skill_points(tuning) == 0 {
        return Err(Refusal::NoSkillPoint);
    }
    if ship.skills[index].rank >= spec.max_rank {
        return Err(Refusal::MaxRank);
    }
    ship.skills[index].rank += 1;
    Ok(())
}

/// 스킬을 씁니다. 피해는 다음 Tick에 들어가고, 그 피해로 적이 가라앉으면 포탄과 같이 보상을 받습니다.
pub(crate) fn cast(
    world: &mut World,
    tuning: &Tuning,
    tick: u64,
    id: ShipId,
    which: SkillId,
) -> Result<(), Refusal> {
    let index = usize::from(which.0);
    let spec = tuning.skills.get(index).ok_or(Refusal::UnknownItem)?;
    let ship = world.ships.get(&id).ok_or(Refusal::Sunk)?;
    if !ship.has_skill(tuning, which) {
        return Err(Refusal::NotOnThisHull);
    }
    let state = ship.skills[index];
    let rank = u32::from(state.rank);
    if rank == 0 {
        return Err(Refusal::NotLearned);
    }
    if !ship.is_afloat() {
        return Err(Refusal::Sunk);
    }
    if state.ready_at > tick {
        return Err(Refusal::OnCooldown);
    }
    let (team, pos) = (ship.team, ship.pos);
    if let SkillEffect::Salvo {
        damage_per_rank,
        radius,
    } = spec.effect
    {
        let hit: Vec<Target> = world
            .targets()
            .filter(|(target, owner, at)| {
                !matches!(target, Target::Structure(_))
                    && owner.is_hostile_to(team)
                    && at.distance(pos) <= radius
            })
            .map(|(target, _, _)| target)
            .collect();
        world.impacts.extend(hit.into_iter().map(|target| Impact {
            due: tick + 1,
            target,
            damage: damage_per_rank * rank,
            by: Some(id),
        }));
    }
    let ship = world.ships.get_mut(&id).ok_or(Refusal::Sunk)?;
    match spec.effect {
        SkillEffect::Repair { hp_per_rank } => {
            let max_hp = tuning.hull(ship.hull).max_hp;
            ship.hp = (ship.hp + hp_per_rank * rank).min(max_hp);
        }
        SkillEffect::Sprint {
            percent_per_rank,
            ticks,
        } => {
            ship.sprint = Some(Boost {
                until: tick + u64::from(ticks),
                percent: percent_per_rank * rank,
            });
        }
        SkillEffect::Brace {
            percent_per_rank,
            ticks,
        } => {
            ship.brace = Some(Boost {
                until: tick + u64::from(ticks),
                percent: percent_per_rank * rank,
            });
        }
        SkillEffect::Reload => {
            for mount in ship.mounts.iter_mut().flatten() {
                mount.ready_at = tick;
            }
        }
        SkillEffect::Salvo { .. } => {}
    }
    ship.skills[index].ready_at = tick + u64::from(spec.cooldown_ticks);
    Ok(())
}
