//! 점수판입니다. 진영마다 배의 이름, 레벨, 가라앉힌 적의 배의 수, 가라앉은 횟수, 가라앉힌 NPC 배의 수를 보여 줍니다.

use warship_protocol::{WireMember, WireScore, WireTeam};

/// 점수판에서 한 진영의 줄들입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreTeam {
    pub team: WireTeam,
    /// 그 진영의 배가 가라앉힌 적의 배의 수의 합입니다.
    pub kills: u32,
    pub rows: Vec<ScoreRow>,
}

/// 점수판에서 배 하나의 줄입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreRow {
    /// 사람이 조종하는 배는 그 사람의 이름이고, 빈자리를 채우는 배는 `Bot`과 배의 번호입니다.
    pub name: String,
    /// 이 접속이 조종하는 배인지입니다.
    pub mine: bool,
    pub level: u8,
    pub kills: u32,
    pub deaths: u32,
    pub creep_kills: u32,
}

/// 점수판의 표입니다. 진영마다 그 진영의 배가 가라앉힌 적의 배가 많은 순서로 있습니다.
pub fn score_table(scores: &[WireScore], roster: &[WireMember], me: u32) -> Vec<ScoreTeam> {
    let team = |team: WireTeam| {
        let mut ships: Vec<&WireScore> = scores.iter().filter(|s| s.team == team).collect();
        ships.sort_by_key(|score| (std::cmp::Reverse(score.kills), score.deaths, score.ship));
        let rows = ships.iter().map(|score| {
            let member = roster.iter().find(|member| member.ship == score.ship);
            ScoreRow {
                name: member.map_or(format!("Bot {}", score.ship), |member| member.name.clone()),
                mine: score.ship == me,
                level: score.level,
                kills: score.kills,
                deaths: score.deaths,
                creep_kills: score.creep_kills,
            }
        });
        ScoreTeam {
            team,
            kills: ships.iter().map(|score| score.kills).sum(),
            rows: rows.collect(),
        }
    };
    vec![team(WireTeam::Red), team(WireTeam::Blue)]
}
