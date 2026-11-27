//! 방 하나입니다. 게임 하나(`Sim`)와, 그 게임의 배를 조종하는 사람의 자리를 가집니다.
//!
//! 자리는 사람 한 명의 이름, 표식, 접속입니다. 접속이 닫혀도 자리는 `hold_ticks` 동안 남고,
//! 그 안에 같은 표식으로 입장한 접속이 같은 배를 다시 조종합니다. 시간이 다 되면 자리가 끝납니다.
//!
//! 빈자리를 채우는 방에서는 사람이 조종하지 않는 자리마다 배가 하나씩 있고, `Pilot`이 그 배를 조종합니다.
//! 사람이 들어오면 사람이 적은 진영의 빈자리 배 하나가 자리를 내주고, 사람이 나가면 그 자리에 새 배가 들어옵니다.
//! 접속이 닫힌 사람의 배는 `Pilot`이 대신 조종하고, 자리가 끝나면 그 배가 그대로 빈자리의 배가 됩니다.
//! 빈자리를 채우지 않는 방에서는 접속이 닫힌 사람의 배가 멈춰 있고, 자리가 끝나면 방에서 빠집니다.

use std::collections::BTreeMap;

use warship_protocol::{
    CommandMsg, InputMsg, LinkId, PeerKey, PlayerName, RejectReason, RoomCode, SeatToken, Snapshot,
    Welcome, WireMember,
};
use warship_rules::{PlayerInput, PlayerInputs, ShipId, Sim, Team, Tuning, Waypoint};

use crate::pilot::Pilot;

/// 사람 한 명의 자리입니다.
struct Seat {
    name: PlayerName,
    token: SeatToken,
    /// 지금 이 자리를 쓰는 접속입니다. 접속이 닫혔으면 없습니다.
    link: Option<LinkId>,
    /// 접속이 닫혀서 자리를 유지하는 동안, 그 접속의 상대입니다. 상대마다 유지하는 자리의 수를 세는 데 씁니다.
    held_by: Option<PeerKey>,
    /// 접속이 닫힌 뒤 지나간 Tick의 수입니다.
    held_ticks: u32,
}

/// 입장한 결과입니다.
pub(crate) struct Entered {
    pub ship: ShipId,
    pub welcome: Welcome,
    /// 표식이 맞아서 있던 자리로 돌아왔는지입니다.
    pub returned: bool,
    /// 같은 표식으로 자리를 이어받아서 밀려난 이전 접속입니다.
    pub replaced: Option<LinkId>,
}

/// Tick 하나의 결과입니다.
pub(crate) struct RoomTick {
    /// 접속마다 보낼 스냅샷입니다.
    pub snapshots: Vec<(LinkId, Box<Snapshot>)>,
    /// 유지하는 시간이 다 되어서 끝난 자리의 배입니다.
    pub ended: Vec<ShipId>,
}

pub(crate) struct Room {
    code: RoomCode,
    sim: Sim,
    /// 사람의 자리입니다. 열쇠는 그 사람이 조종하는 배입니다.
    seats: BTreeMap<ShipId, Seat>,
    /// 다음 Tick에 처리할 입력입니다. 배마다 가장 늦게 받은 것 하나입니다.
    pending: PlayerInputs,
    /// 사람이 조종하지 않는 자리를 배로 채우는 방인지입니다.
    fills_seats: bool,
    /// `Pilot`이 조종하는 배와, 그 배에 마지막으로 붙인 입력과 명령의 순서 번호입니다.
    /// 빈자리의 배와, 접속이 닫힌 사람의 배가 들어 있습니다.
    bots: BTreeMap<ShipId, u32>,
    /// 접속이 닫힌 자리를 유지하는 Tick의 수입니다.
    hold_ticks: u32,
}

impl Room {
    /// `Pilot`이 조종하는 배가 이 Tick 수마다 한 번 지시를 받습니다.
    pub const THINK_TICKS: u64 = 10;

    /// `fills_seats`이면 모든 자리를 빈자리의 배로 채운 방을 만듭니다.
    pub fn new(code: RoomCode, tuning: Tuning, fills_seats: bool, hold_ticks: u32) -> Room {
        let mut room = Room {
            code,
            sim: Sim::new(tuning),
            seats: BTreeMap::new(),
            pending: PlayerInputs::new(),
            fills_seats,
            bots: BTreeMap::new(),
            hold_ticks,
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
    /// 접속이 닫힌 사람의 배는 빼지 않습니다.
    fn free_a_seat(&mut self) {
        let team_of = |ship: &ShipId| self.sim.ship(*ship).map(|ship| ship.team);
        let humans = |team: Team| {
            let teams = self.seats.keys().filter_map(team_of);
            teams.filter(|found| *found == team).count()
        };
        let short = if humans(Team::BLUE) < humans(Team::RED) {
            Team::BLUE
        } else {
            Team::RED
        };
        let free = || {
            let ships = self.bots.keys().rev();
            ships.filter(|ship| !self.seats.contains_key(ship))
        };
        let leaving = free().find(|ship| team_of(ship) == Some(short));
        if let Some(&ship) = leaving.or(free().next()) {
            self.bots.remove(&ship);
            self.pending.remove(&ship);
            self.sim.remove_ship(ship);
        }
    }

    /// 자리의 수입니다. 접속이 닫혀서 유지 중인 자리도 셉니다.
    pub fn members(&self) -> usize {
        self.seats.len()
    }

    /// 접속이 닫혀서 유지 중인 자리의 수입니다.
    pub fn held_count(&self) -> usize {
        self.seats
            .values()
            .filter(|seat| seat.link.is_none())
            .count()
    }

    /// `peer`의 접속이 닫혀서 유지 중인 자리의 수입니다.
    pub fn held_from(&self, peer: PeerKey) -> usize {
        let held = |seat: &&Seat| seat.held_by == Some(peer);
        self.seats.values().filter(held).count()
    }

    /// 이 접속이 조종하는 배입니다.
    pub fn ship_of(&self, link: LinkId) -> Option<ShipId> {
        let mine = |(_, seat): &(&ShipId, &Seat)| seat.link == Some(link);
        self.seats.iter().find(mine).map(|(&ship, _)| ship)
    }

    /// 방의 열려 있는 접속입니다.
    pub fn links(&self) -> impl Iterator<Item = LinkId> + '_ {
        self.seats.values().filter_map(|seat| seat.link)
    }

    /// 방에 있는 사람의 명단입니다. 배의 번호 순서입니다.
    pub fn roster(&self) -> Vec<WireMember> {
        let member = |(ship, seat): (&ShipId, &Seat)| WireMember {
            ship: ship.0,
            name: seat.name.as_str().to_string(),
            connected: seat.link.is_some(),
        };
        self.seats.iter().map(member).collect()
    }

    /// 접속을 방에 넣습니다. `token`이 이 방의 자리와 맞으면 그 자리의 배를 다시 조종하고, 이름은 새 이름으로 바뀝니다.
    /// 맞지 않으면 새 배를 넣고 자리의 표식을 `fresh`로 정합니다. 방이 가득 차 있으면 거절합니다.
    pub fn enter(
        &mut self,
        link: LinkId,
        name: PlayerName,
        token: Option<SeatToken>,
        fresh: SeatToken,
    ) -> Result<Entered, RejectReason> {
        let known = |(_, seat): &(&ShipId, &Seat)| Some(seat.token) == token;
        let found = self.seats.iter().find(known).map(|(&ship, _)| ship);
        let (ship, replaced) = match found {
            Some(ship) => {
                let seat = self.seats.get_mut(&ship).expect("the seat was just found");
                let replaced = seat.link.replace(link);
                seat.held_by = None;
                seat.name = name;
                self.release(ship);
                (ship, replaced)
            }
            None => {
                self.free_a_seat();
                let ship = self.sim.join().map_err(|full| RejectReason::RoomFull {
                    capacity: full.capacity as u32,
                })?;
                let seat = Seat {
                    name,
                    token: fresh,
                    link: Some(link),
                    held_by: None,
                    held_ticks: 0,
                };
                self.seats.insert(ship, seat);
                (ship, None)
            }
        };
        let token = self.seats[&ship].token;
        let welcome = Welcome::from_sim(&self.sim, self.code.as_str(), ship, token)
            .expect("the ship of a seat is in the room");
        Ok(Entered {
            ship,
            welcome,
            returned: found.is_some(),
            replaced,
        })
    }

    /// 배를 조종하던 쪽의 흔적을 지웁니다. 새 조종자는 순서 번호를 1부터 셉니다.
    fn release(&mut self, ship: ShipId) {
        self.bots.remove(&ship);
        self.pending.remove(&ship);
        self.sim.hand_over(ship);
    }

    /// `link`가 닫혔습니다. 그 접속의 자리를 유지하고 그 배를 돌려줍니다. 이 방의 자리를 쓰는 접속이 아니면 없습니다.
    /// 유지하는 동안 배는 `Pilot`이 조종하고, 빈자리를 채우지 않는 방에서는 멈춰 있습니다.
    pub fn link_closed(&mut self, link: LinkId, peer: PeerKey) -> Option<ShipId> {
        let ship = self.ship_of(link)?;
        let seat = self.seats.get_mut(&ship)?;
        seat.link = None;
        seat.held_by = Some(peer);
        seat.held_ticks = 0;
        self.release(ship);
        if self.fills_seats {
            self.bots.insert(ship, 0);
        }
        Some(ship)
    }

    /// 자리를 곧바로 끝내고 그 배를 방에서 뺍니다. 빈자리를 채우는 방에서는 그 자리에 새 배가 들어옵니다.
    pub fn leave(&mut self, ship: ShipId) {
        if self.seats.remove(&ship).is_some() {
            self.release(ship);
            self.sim.remove_ship(ship);
            self.fill_seats();
        }
    }

    /// 입력을 다음 Tick에 처리하도록 받아 둡니다. 이미 받은 순서 번호보다 크지 않은 입력과, 좌표가 유한하지 않은 입력은 버립니다.
    pub fn take_input(&mut self, link: LinkId, msg: InputMsg) {
        let Some(ship) = self.ship_of(link) else {
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
        if let Some(ship) = self.ship_of(link) {
            self.sim.command(ship, msg.seq, msg.command.into());
        }
    }

    /// `Pilot`이 조종하는 배마다 지시를 받아서, 이동은 다음 Tick의 입력으로 받아 두고 명령은 바로 처리합니다.
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

    /// 접속이 닫힌 자리의 시간을 세고, 유지하는 시간이 다 된 자리를 끝냅니다. 끝난 자리의 배를 돌려줍니다.
    /// 빈자리를 채우는 방에서는 그 배가 빈자리의 배로 남고, 아니면 방에서 빠집니다.
    fn end_held_seats(&mut self) -> Vec<ShipId> {
        let mut ended = Vec::new();
        for (&ship, seat) in &mut self.seats {
            if seat.link.is_none() {
                seat.held_ticks += 1;
                if seat.held_ticks >= self.hold_ticks {
                    ended.push(ship);
                }
            }
        }
        for ship in &ended {
            self.seats.remove(ship);
            if !self.fills_seats {
                self.release(*ship);
                self.sim.remove_ship(*ship);
            }
        }
        ended
    }

    /// Tick 하나를 실행하고, 접속마다 보낼 스냅샷과 끝난 자리를 돌려줍니다. 스냅샷은 진영마다 한 번 만듭니다.
    /// `pilot`이 있으면 `THINK_TICKS`마다 `Pilot`이 조종하는 배가 지시를 받습니다.
    pub fn tick(&mut self, pilot: Option<&mut (dyn Pilot + '_)>) -> RoomTick {
        let ended = self.end_held_seats();
        if let Some(pilot) = pilot
            && self.sim.tick().is_multiple_of(Room::THINK_TICKS)
        {
            self.steer_bots(pilot);
        }
        self.sim.step(&self.pending);
        self.pending.clear();
        let mut by_team: [Option<Snapshot>; 2] = [None, None];
        let mut snapshots = Vec::new();
        for (&ship, seat) in &self.seats {
            let (Some(link), Some(ship)) = (seat.link, self.sim.ship(ship)) else {
                continue;
            };
            let team = ship.team;
            let snapshot =
                by_team[team.index()].get_or_insert_with(|| Snapshot::from_sim(&self.sim, team));
            snapshots.push((link, Box::new(snapshot.clone())));
        }
        RoomTick { snapshots, ended }
    }
}
