//! 명령입니다. 구매, 판매, 조합, 선체 교체, 스킬처럼 한 번만 실행해야 하는 조작입니다.
//!
//! 클라이언트는 명령마다 순서 번호를 붙여서 보냅니다. 규칙은 이미 처리한 번호보다 크지 않은 명령을 무시하고,
//! 처리한 명령의 번호와 거절한 이유를 배에 적어 둡니다. 모든 조건(골드, 상점과의 거리, 빈 칸, 쿨다운)은 여기서 검사합니다.

use crate::ids::{GearId, HullId, RecipeId, ShipId, SkillId, TradeId, WeaponId};
use crate::tuning::Tuning;
use crate::world::World;
use crate::{shop, skill, trade};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// 무기를 사서 첫 빈 칸에 답니다.
    Buy(WeaponId),
    /// 장비를 사서 첫 빈 칸에 답니다.
    BuyGear(GearId),
    /// 이 칸의 아이템을 팝니다.
    Sell { slot: u8 },
    /// 달고 있는 재료 무기를 조합 값과 함께 결과 무기 하나로 바꿉니다.
    Combine(RecipeId),
    /// 선체를 바꿉니다. 지금 선체의 가격의 일부를 돌려받습니다.
    ChangeHull(HullId),
    /// 교역의 계약을 사서 첫 빈 칸에 답니다. 자기 진영의 본진에서만 삽니다.
    Contract(TradeId),
    /// 스킬 포인트 하나를 이 스킬에 넣습니다.
    Learn(SkillId),
    /// 스킬을 씁니다.
    Cast(SkillId),
}

/// 명령을 실행하지 않은 이유입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// 배가 가라앉아 있습니다.
    Sunk,
    /// 자기 진영의 본진이나 항구가 가까이에 없습니다.
    NoShopInRange,
    /// 표에 없는 무기, 선체, 스킬입니다.
    UnknownItem,
    NotEnoughGold,
    /// 선체의 칸이 모두 차 있습니다.
    NoFreeMount,
    /// 팔려는 칸에 아이템이 없습니다.
    EmptyMount,
    /// 새 선체에 없는 칸에 아이템이 달려 있습니다.
    MountsInUse,
    /// 이미 그 선체입니다.
    SameHull,
    NoSkillPoint,
    /// 스킬 포인트를 더 넣을 수 없는 스킬입니다.
    MaxRank,
    /// 배우지 않은 스킬입니다.
    NotLearned,
    OnCooldown,
    /// 지금 선체의 배는 쓸 수 없는 스킬입니다.
    NotOnThisHull,
    /// 상점에서 바로 살 수 없는 무기입니다. 조합으로 얻습니다.
    NotForSale,
    /// 조합의 재료 무기를 모두 달고 있지 않습니다.
    MissingParts,
    /// 자기 진영의 본진이 가까이에 없습니다.
    NoBaseInRange,
    /// 교역선이 아닌 선체는 교역을 하지 못합니다.
    NotATrader,
    /// 교역의 계약이나 화물을 단 채로는 교역선이 아닌 선체로 바꾸지 못합니다.
    CarryingFreight,
}

/// 배의 명령 하나를 처리합니다. 없는 배의 명령과 이미 처리한 순서 번호의 명령은 무시합니다.
pub(crate) fn apply(
    world: &mut World,
    tuning: &Tuning,
    tick: u64,
    id: ShipId,
    seq: u32,
    command: Command,
) {
    let fresh = |world: &World| {
        world
            .ships
            .get(&id)
            .is_some_and(|ship| seq > ship.last_command_seq)
    };
    if !fresh(world) {
        return;
    }
    let result = match command {
        Command::Buy(weapon) => shop::buy(world, tuning, id, weapon),
        Command::Sell { slot } => shop::sell(world, tuning, id, usize::from(slot)),
        Command::BuyGear(gear) => shop::buy_gear(world, tuning, id, gear),
        Command::Combine(recipe) => shop::combine(world, tuning, id, recipe),
        Command::ChangeHull(hull) => shop::change_hull(world, tuning, id, hull),
        Command::Contract(which) => trade::contract(world, tuning, id, which),
        Command::Learn(which) => skill::learn(world, tuning, id, which),
        Command::Cast(which) => skill::cast(world, tuning, tick, id, which),
    };
    if let Some(ship) = world.ships.get_mut(&id) {
        ship.last_command_seq = seq;
        ship.refusal = result.err();
    }
}
