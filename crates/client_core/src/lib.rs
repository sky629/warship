//! 클라이언트에서 Bevy가 필요하지 않은 로직입니다. 이 크레이트의 테스트는 Bevy를 컴파일하지 않고 실행됩니다.
//!
//! - `launch`: 실행 인자를 `LaunchOptions`로 해석합니다.
//! - `assets`: 아이콘의 에셋 경로와, 에셋이 없을 때 보여 줄 글자입니다.
//! - `shop`: 상점 창의 탭과 줄, 그 줄이 보내는 명령, 내 배가 있는 상점입니다.
//! - `start`: 시작 화면의 이름 칸과 방 코드 칸입니다.
//! - `view`: 카메라가 보는 점(따라가기, 화면 옮기기, 거리)과 미니맵의 좌표, 상점 창과 칸 줄의 자리입니다.
//! - `mounts`: 화면 아래의 칸 줄에 보여 줄 아이템의 이름과 안내입니다.
//! - `memory`: 기기가 기억하는 이름, 방 코드, 자리의 표식입니다.
//! - `hud`: 내 배의 HP, 경험치, 스킬 칸과 위쪽 상태 막대에 보여 줄 값입니다.
//! - `sound`: 두 프레임 사이에 일어난 일의 소리와 그 파일의 경로입니다.
//! - `score`: 점수판에 보여 줄 줄입니다.
//! - `session`: 접속 방식의 이벤트에 따라 바뀌는 접속 상태입니다. 입장한 뒤에는 입장 안내, 스냅샷 보관, 입력과 명령의 순서 번호를 가집니다.
//! - `interp`: 스냅샷을 보관하고 100ms 이전의 방 모습을 보간해서 `Frame`으로 만듭니다. 날아가는 포탄의 위치도 여기서 계산합니다.

mod assets;
mod hud;
mod interp;
mod launch;
mod memory;
mod mounts;
mod score;
mod session;
mod shop;
mod sound;
mod start;
mod view;

pub use assets::{Icon, IconKind, ModelKind, model_path, shot_models};
pub use hud::{HudView, SkillSlot, TopBar, hud_view, top_bar};
pub use interp::{
    BlastView, CreepView, Frame, MyShip, ShipView, ShotView, SkillView, StructureView, Volley,
};
pub use launch::{DEFAULT_NAME, DEFAULT_SERVER, LaunchOptions, RoomChoice};
pub use memory::{Memory, RememberedSeat};
pub use mounts::{MountView, mount_views};
pub use score::{ScoreRow, ScoreTeam, score_table};
pub use session::{Phase, Session};
pub use shop::{ShopRow, ShopTab, first_tab, shop_tabs, shops_near};
pub use sound::{Sound, SoundEvent, sound_events};
pub use start::{Field, FormKey, JoinRequest, StartForm};
pub use view::{CameraRig, Minimap, MountBar, ShopWindow};

#[cfg(test)]
mod tests;
