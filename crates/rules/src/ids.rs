//! 개체를 구분하는 번호와 개체가 속한 진영입니다.
//!
//! 번호는 종류마다 별도의 타입으로 감쌉니다. 건물 번호를 배 번호 자리에 넣는 실수를 컴파일러가 막습니다.
//! `Sim`은 번호를 하나씩 늘려서 나누어 주고 다시 사용하지 않습니다.
//! 그래서 이미 사라진 개체를 가리키는 번호(무기의 표적, 날아가는 포탄의 표적)가 새 개체를 가리킬 수 없습니다.

/// 플레이어가 조종하는 배의 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShipId(pub u32);

/// 라인을 따라가는 NPC 배의 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreepId(pub u32);

/// 건물(타워, 항구, 본진)의 번호입니다. 맵에 적힌 순서대로 1부터 붙습니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructureId(pub u32);

/// 무기 표(`Tuning::weapons`)의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeaponId(pub u16);

/// 선체 표(`Tuning::hulls`)의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HullId(pub u16);

/// 스킬 표(`Tuning::skills`)의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillId(pub u16);

/// `Tuning::gear`의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GearId(pub u16);

/// `Tuning::recipes`의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipeId(pub u16);

/// `Tuning::supplies`의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SupplyId(pub u16);

/// `Tuning::trades`의 행 번호입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TradeId(pub u16);

/// 개체가 편들어 싸우는 쪽입니다. 플레이어의 진영은 `RED`와 `BLUE`이고, `WILD`는 두 진영 모두와 싸우는 중립 몬스터의 쪽입니다.
///
/// 「누가 누구를 공격할 수 있는가」는 `is_hostile_to`로 판정합니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Team(u8);

impl Team {
    pub const RED: Team = Team(0);
    pub const BLUE: Team = Team(1);
    /// 어느 진영의 편도 아닌 중립 몬스터의 쪽입니다. 배와 건물은 이 쪽에 속하지 않습니다.
    pub const WILD: Team = Team(2);
    /// 플레이어의 진영입니다.
    pub const ALL: [Team; 2] = [Team::RED, Team::BLUE];

    pub fn is_hostile_to(self, other: Team) -> bool {
        self != other
    }

    /// 플레이어의 진영의 상대 진영입니다.
    pub fn enemy(self) -> Team {
        Team(1 - self.0)
    }

    /// 쪽마다 하나씩 있는 배열의 자리입니다. 플레이어의 진영이 앞의 두 자리입니다.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}
