//! 클라이언트에서 Bevy가 필요하지 않은 로직입니다. 이 크레이트의 테스트는 Bevy를 컴파일하지 않고 실행됩니다.
//!
//! - `launch`: 실행 인자를 `LaunchOptions`로 해석합니다.
//! - `session`: 접속 방식의 이벤트에 따라 바뀌는 접속 상태입니다. 입장한 뒤에는 입장 안내, 스냅샷 보관, 입력과 명령의 순서 번호를 가집니다.
//! - `interp`: 스냅샷을 보관하고 100ms 이전의 방 모습을 보간해서 `Frame`으로 만듭니다. 날아가는 포탄의 위치도 여기서 계산합니다.

mod interp;
mod launch;
mod session;

pub use interp::{CreepView, Frame, MyShip, ShipView, ShotView, SkillView, StructureView};
pub use launch::{DEFAULT_ROOM, DEFAULT_SERVER, LaunchOptions};
pub use session::{Phase, Session};

#[cfg(test)]
mod tests;
