//! 화면의 아이콘과 3D 모형이 쓰는 에셋의 이름입니다. 종류와 이름으로 파일의 경로가 정해집니다.
//!
//! 아이콘의 경로는 `icons/<종류의 폴더>/<이름>.png`이고, 모형의 경로는 `models/<종류의 폴더>/<이름>.glb`입니다.
//! 이름은 소문자로 바꾸고, 글자와 숫자가 아닌 것은 `_`로 바꿉니다.
//! 아이콘의 파일이 없으면 화면은 종류의 색의 네모에 이름의 머리글자를 적어서 보여 주고, 모형의 파일이 없으면 기본 도형을 합친 모양을 보여 줍니다.

use warship_protocol::WireShot;

/// 아이콘의 종류입니다. 종류마다 폴더가 하나씩 있습니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Weapon,
    Gear,
    Supply,
    Hull,
    Skill,
    Trade,
    Hire,
}

/// 아이콘 하나입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icon {
    pub kind: IconKind,
    /// 아이템, 선체, 스킬의 이름입니다.
    pub name: String,
}

impl Icon {
    pub fn new(kind: IconKind, name: &str) -> Icon {
        Icon {
            kind,
            name: name.to_string(),
        }
    }

    /// 에셋 폴더 안에서 이 아이콘의 파일의 경로입니다.
    pub fn path(&self) -> String {
        let folder = match self.kind {
            IconKind::Weapon => "weapons",
            IconKind::Gear => "gear",
            IconKind::Supply => "supplies",
            IconKind::Hull => "hulls",
            IconKind::Skill => "skills",
            IconKind::Trade => "trades",
            IconKind::Hire => "hire",
        };
        format!("icons/{folder}/{}.png", slug(&self.name))
    }

    /// 파일이 없을 때 네모에 적는 글자입니다. 이름의 낱말마다 첫 글자를 따서 두 글자까지 씁니다.
    pub fn initials(&self) -> String {
        let letters = self
            .name
            .split_whitespace()
            .filter_map(|word| word.chars().next());
        letters.take(2).collect::<String>().to_uppercase()
    }
}

/// 이름을 파일의 이름으로 바꿉니다. 소문자로 바꾸고, 글자와 숫자가 아닌 것은 `_`로 바꿉니다.
pub(crate) fn slug(name: &str) -> String {
    let letters = name.chars().map(|c| match c {
        c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
        _ => '_',
    });
    letters.collect()
}

/// 3D 모형의 종류입니다. 종류마다 폴더가 하나씩 있습니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    Hull,
    Creep,
    /// 타워, 항구, 본진, 그리고 어느 진영의 것도 아닌 항구입니다.
    Structure,
    /// 포탄입니다. 무기마다 하나씩 있고, 공격 방식마다 하나씩 있습니다.
    Shot,
    /// 맵의 지형(섬)입니다.
    Scenery,
}

/// 포탄의 모형의 경로입니다. 앞의 것부터 찾습니다: 그 무기의 포탄, 그 공격 방식의 포탄.
pub fn shot_models(weapon: &str, kind: WireShot) -> [String; 2] {
    [
        model_path(ModelKind::Shot, weapon),
        model_path(ModelKind::Shot, shot_name(kind)),
    ]
}

/// 공격 방식의 이름입니다. 포탄의 모형과 발사음의 파일의 이름에 씁니다.
pub(crate) fn shot_name(kind: WireShot) -> &'static str {
    match kind {
        WireShot::Shell => "shell",
        WireShot::Scatter { .. } => "scatter",
        WireShot::Bomb { .. } => "bomb",
        WireShot::Fire { .. } => "flame",
    }
}

/// 에셋 폴더 안에서 이 모형의 파일의 경로입니다.
pub fn model_path(kind: ModelKind, name: &str) -> String {
    let folder = match kind {
        ModelKind::Hull => "hulls",
        ModelKind::Creep => "creeps",
        ModelKind::Structure => "structures",
        ModelKind::Shot => "shots",
        ModelKind::Scenery => "scenery",
    };
    format!("models/{folder}/{}.glb", slug(name))
}
