//! 방 하나입니다. 게임 하나(`Sim`)와, 그 게임의 배를 조종하는 접속을 가집니다.

use std::collections::BTreeMap;

use warship_protocol::{InputMsg, LinkId, RejectReason, RoomCode, Snapshot, Welcome};
use warship_rules::{PlayerInput, PlayerInputs, ShipId, Sim, Tuning, Waypoint};

pub(crate) struct Room {
    sim: Sim,
    /// 접속마다 그 접속이 조종하는 배입니다.
    ships: BTreeMap<LinkId, ShipId>,
    /// 다음 Tick에 처리할 입력입니다. 배마다 가장 늦게 받은 것 하나입니다.
    pending: PlayerInputs,
}

impl Room {
    pub fn new(tuning: Tuning) -> Room {
        Room {
            sim: Sim::new(tuning),
            ships: BTreeMap::new(),
            pending: PlayerInputs::new(),
        }
    }

    pub fn members(&self) -> usize {
        self.ships.len()
    }

    /// 새 배를 방에 넣고, 그 배의 번호와 입장 안내를 돌려줍니다. 방이 가득 차 있으면 거절합니다.
    pub fn enter(
        &mut self,
        link: LinkId,
        code: &RoomCode,
    ) -> Result<(ShipId, Welcome), RejectReason> {
        let ship = self.sim.join().map_err(|full| RejectReason::RoomFull {
            capacity: full.capacity as u32,
        })?;
        self.ships.insert(link, ship);
        let welcome = Welcome::from_sim(&self.sim, code.as_str(), ship)
            .expect("the ship that just joined is in the room");
        Ok((ship, welcome))
    }

    /// 접속의 배를 방에서 빼고 그 번호를 돌려줍니다.
    pub fn exit(&mut self, link: LinkId) -> Option<ShipId> {
        let ship = self.ships.remove(&link)?;
        self.pending.remove(&ship);
        self.sim.remove_ship(ship);
        Some(ship)
    }

    /// 입력을 다음 Tick에 처리하도록 받아 둡니다. 이미 받은 순서 번호보다 크지 않은 입력과, 좌표가 유한하지 않은 입력은 버립니다.
    pub fn take_input(&mut self, link: LinkId, msg: InputMsg) {
        let Some(&ship) = self.ships.get(&link) else {
            return;
        };
        let processed = self.sim.ship(ship).map_or(0, |ship| ship.last_input_seq);
        let latest = self.pending.get(&ship).map_or(processed, |input| input.seq);
        if msg.seq <= latest {
            return;
        }
        let destination = match msg.destination {
            None => None,
            Some(point) => match Waypoint::new(point.x, point.y) {
                Some(waypoint) => Some(waypoint),
                None => return,
            },
        };
        let input = PlayerInput {
            seq: msg.seq,
            destination,
        };
        self.pending.insert(ship, input);
    }

    /// Tick 하나를 실행하고, 접속마다 보낼 스냅샷을 돌려줍니다. 스냅샷은 진영마다 한 번 만듭니다.
    pub fn tick(&mut self) -> Vec<(LinkId, Box<Snapshot>)> {
        self.sim.step(&self.pending);
        self.pending.clear();
        let mut by_team: [Option<Snapshot>; 2] = [None, None];
        let mut out = Vec::new();
        for (&link, &ship) in &self.ships {
            let Some(team) = self.sim.ship(ship).map(|ship| ship.team) else {
                continue;
            };
            let snapshot =
                by_team[team.index()].get_or_insert_with(|| Snapshot::from_sim(&self.sim, team));
            out.push((link, Box::new(snapshot.clone())));
        }
        out
    }
}
