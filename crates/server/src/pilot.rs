//! 빈자리의 배를 조종하는 쪽의 인터페이스입니다. 서버는 봇의 구현을 알지 못하고, 실행 파일이 구현을 넘깁니다.

use warship_rules::{Command, ShipId, Sim, Waypoint};

/// 빈자리의 배를 조종합니다. 방은 `Room::THINK_TICKS`마다 빈자리의 배마다 한 번 묻습니다.
pub trait Pilot {
    fn steer(&mut self, sim: &Sim, ship: ShipId) -> Orders;
}

/// 조종하는 쪽이 한 번에 내리는 지시입니다. 사람의 입력과 같은 경로로 규칙에 들어갑니다.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Orders {
    /// 배가 향할 지점입니다. 없으면 멈춥니다.
    pub destination: Option<Waypoint>,
    pub commands: Vec<Command>,
}
