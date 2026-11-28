//! 두 프레임을 비교해서 그 사이에 일어난 일의 소리를 고릅니다. 화면은 이 소리의 파일을 틀기만 합니다.
//!
//! 소리의 파일은 에셋 폴더의 `sounds/<이름>.ogg`입니다. 발사음은 무기마다 `sounds/fire/<무기의 이름>.ogg`이고, 없으면 공격 방식의 `sounds/fire_<방식>.ogg`입니다. 자리가 있는 소리는 화면이 보는 곳에서 멀수록 작게 틀고,
//! 자리가 없는 소리(내 배의 구매, 골드의 보상, 거절, 레벨, 승패)는 그대로 틉니다.

use warship_protocol::{Welcome, WireOutcome, WireShot};
use warship_rules::Vec2;

use crate::assets::{shot_name, slug};
use crate::interp::{FloatKind, FloatView, Frame};

/// 소리의 종류입니다.
#[derive(Clone, Debug, PartialEq)]
pub enum Sound {
    /// 포탄을 쏘는 소리입니다. 무기마다 다르고, 그 무기의 소리가 없으면 공격 방식의 소리입니다.
    Fire {
        weapon: String,
        kind: WireShot,
    },
    /// 산탄이나 폭탄이 터지는 소리입니다.
    Blast,
    /// 배가 가라앉는 소리입니다.
    Sink,
    /// 건물이 파괴되는 소리입니다.
    Wreck,
    Buy,
    /// 내 배가 보상으로 골드를 받는 소리입니다.
    Gain,
    Refuse,
    LevelUp,
    Victory,
    Defeat,
}

impl Sound {
    /// 에셋 폴더 안에서 이 소리의 파일의 경로입니다. 앞의 것부터 찾습니다.
    pub fn paths(&self) -> Vec<String> {
        let name = match self {
            Sound::Fire { weapon, kind } => {
                return vec![
                    format!("sounds/fire/{}.ogg", slug(weapon)),
                    format!("sounds/fire_{}.ogg", shot_name(*kind)),
                ];
            }
            Sound::Blast => "blast",
            Sound::Sink => "sink",
            Sound::Wreck => "wreck",
            Sound::Buy => "buy",
            Sound::Gain => "gain",
            Sound::Refuse => "refuse",
            Sound::LevelUp => "level_up",
            Sound::Victory => "victory",
            Sound::Defeat => "defeat",
        };
        vec![format!("sounds/{name}.ogg")]
    }
}

/// 틀 소리 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundEvent {
    pub sound: Sound,
    /// 소리가 난 자리입니다. 내 배의 일과 승패의 소리에는 없습니다.
    pub at: Option<Vec2>,
}

/// `prev`와 `next` 사이에 일어난 일의 소리입니다.
pub fn sound_events(welcome: &Welcome, prev: &Frame, next: &Frame) -> Vec<SoundEvent> {
    let mut events = Vec::new();
    let mut heard = |sound, at| events.push(SoundEvent { sound, at });
    // 앞 프레임에 없던 포탄은 그사이에 쏜 포탄입니다.
    for shot in &next.shots {
        if prev.shots.iter().any(|old| old.volley == shot.volley) {
            continue;
        }
        let sound = Sound::Fire {
            weapon: shot.weapon.clone(),
            kind: shot.kind,
        };
        heard(sound, Some(shot.pos));
    }
    for blast in &next.blasts {
        if !prev.blasts.iter().any(|old| old.volley == blast.volley) {
            heard(Sound::Blast, Some(blast.pos));
        }
    }
    for ship in next.ships.iter().filter(|ship| ship.sunk) {
        let before = prev.ships.iter().find(|old| old.id == ship.id);
        if before.is_some_and(|old| !old.sunk) {
            heard(Sound::Sink, Some(ship.pos));
        }
    }
    // 파괴된 건물은 프레임에서 사라집니다.
    for site in &prev.structures {
        if !next.structures.iter().any(|new| new.id == site.id) {
            heard(Sound::Wreck, Some(site.pos));
        }
    }
    // 앞 프레임에 없던 골드의 숫자는 그사이에 받은 보상입니다.
    let gold = |float: &&FloatView| float.kind == FloatKind::Gold;
    for float in next.floats.iter().filter(gold) {
        if !prev.floats.iter().any(|old| old.id == float.id) {
            heard(Sound::Gain, None);
        }
    }
    if let (Some(before), Some(me)) = (&prev.me, &next.me) {
        // 골드는 살 때만 줄어듭니다.
        if me.gold < before.gold {
            heard(Sound::Buy, None);
        }
        if me.command_seq != before.command_seq && me.refusal.is_some() {
            heard(Sound::Refuse, None);
        }
        if me.level > before.level {
            heard(Sound::LevelUp, None);
        }
    }
    if let (None, Some(outcome)) = (prev.outcome, next.outcome) {
        let won = outcome == WireOutcome::Victory(welcome.team);
        heard(if won { Sound::Victory } else { Sound::Defeat }, None);
    }
    events
}
