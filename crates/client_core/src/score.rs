//! 점수판입니다. 진영마다 배의 이름, 레벨, 가라앉힌 적의 배의 수, 가라앉은 횟수, 가라앉힌 NPC 배의 수를 보여 줍니다.

use warship_protocol::{WireMember, WireScore, WireTeam};

/// 점수판의 줄입니다. 진영마다 제목 줄이 있고, 그 아래에 그 진영의 배가 가라앉힌 적의 배가 많은 순서로 있습니다.
/// 사람이 조종하는 배는 그 사람의 이름이고, 빈자리를 채우는 배는 `Bot`과 배의 번호입니다. `me`의 줄은 `>`로 표시합니다.
pub fn score_lines(scores: &[WireScore], roster: &[WireMember], me: u32) -> String {
    let mut lines = Vec::new();
    for (team, title) in [(WireTeam::Red, "Red"), (WireTeam::Blue, "Blue")] {
        lines.push(format!(
            "{title:<18}{:>3}{:>5}{:>5}{:>8}",
            "Lv", "K", "D", "Creeps"
        ));
        let mut ships: Vec<&WireScore> = scores.iter().filter(|s| s.team == team).collect();
        ships.sort_by_key(|score| (std::cmp::Reverse(score.kills), score.deaths, score.ship));
        for score in ships {
            let member = roster.iter().find(|member| member.ship == score.ship);
            let name = member.map_or(format!("Bot {}", score.ship), |member| member.name.clone());
            let mark = if score.ship == me { '>' } else { ' ' };
            lines.push(format!(
                "{mark} {name:<16}{:>3}{:>5}{:>5}{:>8}",
                score.level, score.kills, score.deaths, score.creep_kills
            ));
        }
    }
    lines.join("\n")
}
