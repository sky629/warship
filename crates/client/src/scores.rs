//! 점수판입니다. Tab을 누르고 있는 동안 화면 가운데에 보이고, 대전이 끝나면 결과와 함께 계속 보입니다.
//! 표의 값은 `warship_client_core::score_table`이 만듭니다.

use bevy::prelude::*;
use warship_client_core::score_table;
use warship_protocol::{WireOutcome, WireTeam};

use crate::net::{CurrentFrame, Net};
use crate::ui::{self, BAD, BLUE, DIM, GOLD, GOOD, LIT, Panel, RED, TEXT, words};

/// 점수판의 너비입니다.
const WIDTH: f32 = 460.0;
/// 열의 너비입니다. 이름, 레벨, 가라앉힌 배, 가라앉은 횟수, 가라앉힌 NPC 배의 순서입니다.
const COLUMNS: [f32; 5] = [200.0, 44.0, 48.0, 48.0, 80.0];

/// 점수판의 바탕입니다.
#[derive(Component)]
pub struct ScoreRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        ScoreRoot,
        Panel,
        Node {
            position_type: PositionType::Absolute,
            top: percent(16),
            left: percent(50),
            margin: UiRect::left(px(-WIDTH / 2.0)),
            width: px(WIDTH),
            padding: UiRect::all(px(14)),
            row_gap: px(2),
            flex_direction: FlexDirection::Column,
            display: Display::None,
            ..default()
        },
        BackgroundColor(ui::PANEL),
    ));
}

/// 한 줄의 칸들입니다. 첫 칸은 이름이고 나머지는 오른쪽으로 맞춘 숫자입니다.
fn line(parent: &mut ChildSpawnerCommands, cells: [String; 5], color: Color, back: Color) {
    let row = Node {
        padding: UiRect::axes(px(6), px(2)),
        ..default()
    };
    parent
        .spawn((row, BackgroundColor(back)))
        .with_children(|row| {
            for (index, cell) in cells.into_iter().enumerate() {
                let column = Node {
                    width: px(COLUMNS[index]),
                    justify_content: if index == 0 {
                        JustifyContent::Start
                    } else {
                        JustifyContent::End
                    },
                    overflow: Overflow::clip(),
                    ..default()
                };
                row.spawn(column).with_children(|column| {
                    column.spawn(words(cell, 15.0, color));
                });
            }
        });
}

pub fn show_scores(
    keys: Res<ButtonInput<KeyCode>>,
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    root: Single<(Entity, &mut Node), With<ScoreRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let shown = welcome.zip(net.as_ref()).zip(frame.0.as_ref());
    let shown = shown.filter(|(_, frame)| frame.outcome.is_some() || keys.pressed(KeyCode::Tab));
    let Some(((welcome, net), frame)) = shown else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    let result = frame.outcome.map(|outcome| match outcome {
        WireOutcome::Victory(winner) if winner == welcome.team => ("VICTORY", GOOD),
        WireOutcome::Victory(_) => ("DEFEAT", BAD),
        WireOutcome::Draw => ("DRAW", GOLD),
    });
    let table = score_table(&frame.scores, net.session.roster(), welcome.ship);
    commands.entity(root).with_children(|parent| {
        if let Some((result, color)) = result {
            parent.spawn(words(result, 30.0, color));
        }
        for team in table {
            let (title, color) = match team.team {
                WireTeam::Red => ("Red", RED),
                WireTeam::Blue => ("Blue", BLUE),
                WireTeam::Wild => ("Wild", DIM),
            };
            let heading = [title, "Lv", "K", "D", "Creeps"].map(str::to_string);
            line(parent, heading, color, Color::NONE);
            for row in team.rows {
                let back = if row.mine { LIT } else { Color::NONE };
                let cells = [
                    row.name,
                    row.level.to_string(),
                    row.kills.to_string(),
                    row.deaths.to_string(),
                    row.creep_kills.to_string(),
                ];
                line(parent, cells, TEXT, back);
            }
        }
    });
}
