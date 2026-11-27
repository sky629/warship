//! 방 하나입니다. 게임 하나(`Sim`)와, 그 게임의 배를 조종하는 접속을 가집니다.
//!
//! 빈자리를 채우는 방에서는 사람이 조종하지 않는 자리마다 배가 하나씩 있고, `Pilot`이 그 배를 조종합니다.
//! 사람이 들어오면 사람이 적은 진영의 빈자리 배 하나가 자리를 내주고, 사람이 나가면 그 자리에 새 배가 들어옵니다.

use std::collections::BTreeMap;

use warship_protocol::{CommandMsg, InputMsg, LinkId, RejectReason, RoomCode, Snapshot, Welcome};
use warship_rules::{PlayerInput, PlayerInputs, ShipId, Sim, Team, Tuning, Waypoint};

use crate::pilot::Pilot;

pub(crate) struct Room {
    sim: Sim,
    /// 접속마다 그 접속이 조종하는 배입니다.
    ships: BTreeMap<LinkId, ShipId>,
    /// 다음 Tick에 처리할 입력입니다. 배마다 가장 늦게 받은 것 하나입니다.
    pending: PlayerInputs,
    /// 사람이 조종하지 않는 자리를 배로 채우는 방인지입니다.
    fills_seats: bool,
    /// 빈자리의 배와, 그 배에 마지막으로 붙인 입력과 명령의 순서 번호입니다.
    bots: BTreeMap<ShipId, u32>,
}

impl Room {
    /// 빈자리의 배가 이 Tick 수마다 한 번 지시를 받습니다.
    pub const THINK_TICKS: u64 = 10;

    /// `fills_seats`이면 모든 자리를 빈자리의 배로 채운 방을 만듭니다.
    pub fn new(tuning: Tuning, fills_seats: bool) -> Room {
        let mut room = Room {
            sim: Sim::new(tuning),
            ships: BTreeMap::new(),
            pending: PlayerInputs::new(),
            fills_seats,
            bots: BTreeMap::new(),
        };
        room.fill_seats();
        room
    }

    /// 남은 자리를 모두 빈자리의 배로 채웁니다.
    fn fill_seats(&mut self) {
        while self.fills_seats
            && let Ok(ship) = self.sim.join()
        {
            self.bots.insert(ship, 0);
        }
    }

    /// 사람이 적은 진영의 빈자리 배 하나를 방에서 뺍니다. 그 진영에 빈자리 배가 없으면 다른 진영에서 뺍니다.
    fn free_a_seat(&mut self) {
        let team_of = |ship: &ShipId| self.sim.ship(*ship).map(|ship| ship.team);
        let humans = |team: Team| {
            let teams = self.ships.values().filter_map(team_of);
            teams.filter(|found| *found == team).count()
        };
        let short = if humans(Team::BLUE) < humans(Team::RED) {
            Team::BLUE
        } else {
            Team::RED
        };
        let on_short_team = |ship: &&ShipId| team_of(ship) == Some(short);
        let leaving = self.bots.keys().rev().find(on_short_team);
        if let Some(&ship) = leaving.or(self.bots.keys().next_back()) {
            self.bots.remove(&ship);
            self.pending.remove(&ship);
            self.sim.remove_ship(ship);
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
        self.free_a_seat();
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
        self.fill_seats();
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

    /// 명령을 바로 처리합니다. 조건의 검사와 순서 번호의 확인은 규칙이 합니다.
    pub fn take_command(&mut self, link: LinkId, msg: CommandMsg) {
        if let Some(&ship) = self.ships.get(&link) {
            self.sim.command(ship, msg.seq, msg.command.into());
        }
    }

    /// 빈자리의 배마다 지시를 받아서, 이동은 다음 Tick의 입력으로 받아 두고 명령은 바로 처리합니다.
    fn steer_bots(&mut self, pilot: &mut dyn Pilot) {
        for (&ship, seq) in &mut self.bots {
            let orders = pilot.steer(&self.sim, ship);
            *seq += 1;
            let input = PlayerInput {
                seq: *seq,
                destination: orders.destination,
            };
            self.pending.insert(ship, input);
            for command in orders.commands {
                *seq += 1;
                self.sim.command(ship, *seq, command);
            }
        }
    }

    /// Tick 하나를 실행하고, 접속마다 보낼 스냅샷을 돌려줍니다. 스냅샷은 진영마다 한 번 만듭니다.
    /// `pilot`이 있으면 `THINK_TICKS`마다 빈자리의 배가 지시를 받습니다.
    pub fn tick(&mut self, pilot: Option<&mut (dyn Pilot + '_)>) -> Vec<(LinkId, Box<Snapshot>)> {
        if let Some(pilot) = pilot
            && self.sim.tick().is_multiple_of(Room::THINK_TICKS)
        {
            self.steer_bots(pilot);
        }
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
