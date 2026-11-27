//! 효과음과 배경 음악입니다. 소리의 파일이 에셋 폴더에 있을 때만 틉니다.
//!
//! 효과음은 `warship_client_core::sound_events`가 프레임 사이에 일어난 일에서 고릅니다. 자리가 있는 소리는 화면이 보는 곳에서 멀수록 작게 틀고, 들리는 거리 밖의 소리는 틀지 않습니다.
//! 배경 음악은 시작 화면과 방에 하나씩 있고, 화면이 바뀌면 음악도 바뀝니다.

use bevy::audio::Volume;
use bevy::prelude::*;
use warship_client_core::{Frame, sound_events};

use crate::Screen;
use crate::camera::Rig;
use crate::net::{CurrentFrame, Net};
use crate::ui;

/// 화면이 보는 곳에서 이 거리(타일)보다 먼 소리는 들리지 않습니다.
const HEAR_RANGE: f32 = 28.0;
/// 한 프레임에 새로 트는 효과음의 수의 한도입니다. 여러 배가 한꺼번에 쏠 때 소리가 뭉개지지 않게 합니다.
const MOST_AT_ONCE: usize = 6;
/// 배경 음악의 크기입니다. 효과음이 1입니다.
const MUSIC_VOLUME: f32 = 0.45;
const START_MUSIC: &str = "music/start.ogg";
const BATTLE_MUSIC: &str = "music/battle.ogg";

/// 소리를 고를 때 비교하는 앞 프레임입니다.
#[derive(Resource, Default)]
pub struct Heard(Option<Frame>);

/// 지금 틀고 있는 배경 음악입니다. 값은 그 음악의 파일입니다.
#[derive(Component)]
pub struct Music(&'static str);

/// 앞 프레임과 이번 프레임 사이에 일어난 일의 효과음을 틉니다.
pub fn play_sounds(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    rig: Res<Rig>,
    assets: Res<AssetServer>,
    mut heard: ResMut<Heard>,
    mut commands: Commands,
) {
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let (Some(welcome), Some(frame)) = (welcome, frame.0.as_ref()) else {
        heard.0 = None;
        return;
    };
    let events = match &heard.0 {
        Some(prev) => sound_events(welcome, prev, frame),
        None => Vec::new(),
    };
    heard.0 = Some(frame.clone());
    let focus = rig.0.as_ref().map(|rig| rig.focus);
    let volume = |at: Option<warship_rules::Vec2>| match (at, focus) {
        (Some(at), Some(focus)) => 1.0 - at.distance(focus) / HEAR_RANGE,
        _ => 1.0,
    };
    let audible = events
        .iter()
        .map(|event| (event.sound.path(), volume(event.at)))
        .filter(|(path, volume)| *volume > 0.0 && ui::has(path));
    for (path, volume) in audible.take(MOST_AT_ONCE) {
        let settings = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume));
        commands.spawn((AudioPlayer::new(assets.load(path)), settings));
    }
}

/// 화면에 맞는 배경 음악을 틉니다. 화면이 바뀌면 틀던 음악을 끄고 새 음악을 틉니다.
pub fn play_music(
    screen: Res<Screen>,
    assets: Res<AssetServer>,
    playing: Query<(Entity, &Music)>,
    mut commands: Commands,
) {
    let wanted = match *screen {
        Screen::Start => START_MUSIC,
        Screen::Room => BATTLE_MUSIC,
    };
    let mut on = false;
    for (entity, music) in &playing {
        if music.0 == wanted {
            on = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if !on && ui::has(wanted) {
        let settings = PlaybackSettings::LOOP.with_volume(Volume::Linear(MUSIC_VOLUME));
        commands.spawn((
            Music(wanted),
            AudioPlayer::new(assets.load(wanted)),
            settings,
        ));
    }
}
