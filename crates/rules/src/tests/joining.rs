//! 입장과 퇴장을 확인합니다. 새 배는 배가 적은 진영에 들어가고, 한 진영의 정원은 5척입니다.

use super::support::Scenario;
use crate::ids::Team;
use crate::sim::RoomFull;
use crate::weapon::Target;

#[test]
fn new_ships_join_the_smaller_team_and_red_goes_first() {
    let mut s = Scenario::open_sea();

    let teams: Vec<Team> = (0..4)
        .map(|_| {
            let id = s.sim.join().unwrap();
            s.ship(id).team
        })
        .collect();

    assert_eq!(teams, [Team::RED, Team::BLUE, Team::RED, Team::BLUE]);
}

#[test]
fn a_full_room_refuses_the_eleventh_ship_and_admits_again_after_a_leave() {
    let mut s = Scenario::open_sea();
    let ships: Vec<_> = (0..10).map(|_| s.sim.join().unwrap()).collect();

    assert_eq!(s.sim.join(), Err(RoomFull { capacity: 10 }));

    s.sim.remove_ship(ships[1]);
    let newcomer = s.sim.join().unwrap();
    assert_eq!(s.ship(newcomer).team, Team::BLUE);
    assert!(s.sim.ship(ships[1]).is_none());
}

#[test]
fn a_team_sees_only_the_enemies_inside_its_vision() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let near = s.ship_at(Team::BLUE, 18.0, 10.0);
    let far = s.ship_at(Team::BLUE, 30.0, 10.0);

    s.run_ticks(1);

    assert!(s.sim.sees(Team::RED, Target::Ship(near)));
    assert!(!s.sim.sees(Team::RED, Target::Ship(far)));
    assert!(s.sim.sees(Team::BLUE, Target::Ship(a)));
}
