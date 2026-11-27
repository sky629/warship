//! 화면 아래의 내 배 패널과 위쪽 상태 막대에 보여 줄 값입니다. 화면은 이 값을 막대와 칸으로 그리기만 합니다.

use warship_protocol::{Welcome, WireScore, WireTeam};

use crate::assets::{Icon, IconKind};
use crate::interp::MyShip;

/// 스킬 칸의 단축키입니다. 선체의 스킬의 순서입니다.
pub const SKILL_KEYS: [char; 4] = ['Q', 'W', 'E', 'R'];

/// 내 배 패널에 보여 줄 값입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct HudView {
    pub hull: String,
    pub level: u8,
    pub skill_points: u8,
    pub hp: u32,
    pub max_hp: u32,
    /// 지금 레벨에서 얻은 경험치와, 다음 레벨까지 필요한 경험치입니다. 가장 높은 레벨에서는 없습니다.
    pub xp: Option<(u32, u32)>,
    pub skills: Vec<SkillSlot>,
}

/// 스킬 칸 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillSlot {
    pub icon: Icon,
    pub key: char,
    pub name: String,
    pub rank: u8,
    pub max_rank: u8,
    /// 다시 쓸 수 있을 때까지 남은 시간(초)입니다. 쓸 수 있으면 0입니다.
    pub ready_secs: f32,
    /// 남은 시간이 쿨다운 전체에서 차지하는 비율입니다(0 이상 1 이하).
    pub wait: f32,
    /// 스킬 포인트를 넣을 수 있는지입니다.
    pub learnable: bool,
}

impl HudView {
    /// 최대 HP에 대한 HP의 비율입니다.
    pub fn hp_share(&self) -> f32 {
        self.hp as f32 / self.max_hp.max(1) as f32
    }

    /// 다음 레벨까지 필요한 경험치에 대한, 지금 레벨에서 얻은 경험치의 비율입니다. 가장 높은 레벨에서는 1입니다.
    pub fn xp_share(&self) -> f32 {
        self.xp
            .map_or(1.0, |(got, span)| got as f32 / span.max(1) as f32)
    }
}

pub fn hud_view(welcome: &Welcome, me: &MyShip) -> HudView {
    let hull = welcome.hulls.get(usize::from(me.hull));
    // 레벨 L의 배는 `level_xp[L - 2]`를 넘었고, `level_xp[L - 1]`이 되면 다음 레벨입니다.
    let level = usize::from(me.level.max(1));
    let floor = level.checked_sub(2).and_then(|at| welcome.level_xp.get(at));
    let floor = floor.copied().unwrap_or(0);
    let xp = welcome.level_xp.get(level - 1).map(|next| {
        (
            me.xp.saturating_sub(floor),
            next.saturating_sub(floor).max(1),
        )
    });
    let ids = hull.map_or(&[][..], |hull| hull.skills.as_slice());
    let skills = ids.iter().zip(SKILL_KEYS).filter_map(|(&id, key)| {
        let spec = welcome.skills.get(usize::from(id))?;
        let state = me.skills.get(usize::from(id))?;
        let cooldown = spec.cooldown_ticks as f32 / welcome.tick_hz.max(1) as f32;
        Some(SkillSlot {
            icon: Icon::new(IconKind::Skill, &spec.name),
            key,
            name: spec.name.clone(),
            rank: state.rank,
            max_rank: spec.max_rank,
            ready_secs: state.ready_secs,
            wait: (state.ready_secs / cooldown.max(f32::EPSILON)).clamp(0.0, 1.0),
            learnable: me.skill_points > 0 && state.rank < spec.max_rank,
        })
    });
    HudView {
        hull: hull.map_or("?".to_string(), |hull| hull.name.clone()),
        level: me.level,
        skill_points: me.skill_points,
        hp: me.hp,
        max_hp: me.max_hp,
        xp,
        skills: skills.collect(),
    }
}

/// 위쪽 상태 막대에 보여 줄 값입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopBar {
    /// 대전이 시작한 뒤 지난 시간입니다(분:초).
    pub clock: String,
    /// 진영마다 가라앉힌 적의 배의 수입니다.
    pub red_kills: u32,
    pub blue_kills: u32,
}

pub fn top_bar(secs: f32, scores: &[WireScore]) -> TopBar {
    let secs = secs.max(0.0) as u32;
    let kills = |team: WireTeam| {
        let ships = scores.iter().filter(|score| score.team == team);
        ships.map(|score| score.kills).sum()
    };
    TopBar {
        clock: format!("{:02}:{:02}", secs / 60, secs % 60),
        red_kills: kills(WireTeam::Red),
        blue_kills: kills(WireTeam::Blue),
    }
}
