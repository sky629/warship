//! Warship의 게임 규칙입니다.
//!
//! 규칙은 시계, 소켓, 파일, 화면을 알지 못합니다. 시간은 `Sim::step` 한 번이 서버 Tick 하나라는 약속으로만 흐릅니다.
//! 같은 수치와 같은 입력은 항상 같은 결과를 내므로, 같은 코드가 서버와 테스트에서 실행됩니다.
//!
//! 바깥에서 쓰는 것은 `Sim`(생성, 입장, 퇴장, `command`, `step`, 읽기 함수), `Tuning`(모든 수치), `PlayerInput`, `Command`, 그리고 스냅샷이 읽는 상태 타입입니다.
//!
//! 읽는 순서: `tuning`(모든 수치), `sim`(Tick 하나의 처리 순서), `ship`(이동과 다시 나타남), `nav`(섬을 피해서 가는 길), `creep`(라인의 NPC 배),
//! `vision`(진영별 시야), `weapon`(자동 사격과 늦게 닿는 피해), `economy`(수입, 보상, 경험치), `structures`(타워, 항구, 본진),
//! `command`(명령의 순서 번호와 거절), `shop`(구매, 판매, 선체 교체), `skill`(스킬을 배우고 쓰는 것).
//! `world`, `math`, `ids`, `input`은 이 모듈들이 쓰는 도구입니다.

mod command;
mod creep;
mod economy;
mod ids;
mod input;
mod math;
mod nav;
mod ship;
mod shop;
mod sim;
mod skill;
mod structures;
mod tuning;
mod vision;
mod weapon;
mod world;

#[cfg(test)]
mod tests;

pub use command::{Command, Refusal};
pub use creep::Creep;
pub use ids::{CreepId, HullId, ShipId, SkillId, StructureId, Team, WeaponId};
pub use input::{PlayerInput, PlayerInputs, Waypoint};
pub use math::Vec2;
pub use ship::{Boost, Life, MOUNTS, Mount, Ship, SkillState};
pub use sim::{Outcome, RoomFull, Sim};
pub use structures::{Structure, StructureKind};
pub use tuning::{
    CreepSpec, EconomyTuning, GrowthTuning, Hits, HullSpec, Island, MapSpec, ShipTuning,
    ShopTuning, SkillEffect, SkillSpec, StructureSite, StructureSpec, StructureTable, Tuning, Wave,
    WeaponSpec,
};
pub use weapon::Target;
