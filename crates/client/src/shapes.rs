//! 배와 건물의 모양입니다. 기본 도형 여러 개를 메시 하나로 합쳐서 만듭니다.
//!
//! 도형마다 꼭짓점 색을 넣습니다. 꼭짓점 색은 재질의 색(진영의 색)에 곱해지므로, 부분마다 진영의 색의 밝기가 달라집니다.
//! 배는 뱃머리가 x축의 양의 방향이고, 모든 모양의 바닥은 높이 0입니다.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

/// 진영의 색을 그대로 내는 부분입니다. 값은 선형 색 공간의 배율입니다.
const BODY: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// 진영의 색보다 어두운 부분(선실, 지붕, 돛대)입니다.
const DARK: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
/// 진영의 색보다 밝은 부분(돛, 깃발)입니다. 곱하는 값이므로 흰색이 되지는 않고, 진영의 밝은 색이 됩니다.
const LIGHT: [f32; 4] = [2.5, 2.5, 2.5, 1.0];

/// 도형 하나에 색을 입히고 `at`으로 옮깁니다.
fn part(shape: impl Into<Mesh>, color: [f32; 4], at: Vec3) -> Mesh {
    let mesh: Mesh = shape.into();
    let colors = vec![color; mesh.count_vertices()];
    mesh.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .translated_by(at)
}

/// 도형들을 메시 하나로 합칩니다.
fn join(parts: Vec<Mesh>) -> Mesh {
    let mut parts = parts.into_iter();
    let mut whole = parts.next().expect("a shape has at least one part");
    for part in parts {
        whole
            .merge(&part)
            .expect("the parts are built the same way");
    }
    whole
}

/// 선체, 뾰족한 뱃머리, 선실, 돛대와 돛입니다. 길이는 약 1.3이고, 돛대는 `masts`개(1개에서 3개)입니다.
pub fn ship(masts: usize) -> Mesh {
    // 원뿔의 축을 x축으로 눕혀서 뱃머리로 씁니다.
    let bow = Mesh::from(Cone::new(0.2, 0.45))
        .rotated_by(Quat::from_rotation_z(-FRAC_PI_2))
        .scaled_by(Vec3::new(1.0, 0.65, 1.0));
    let mut parts = vec![
        part(
            Cuboid::new(0.9, 0.26, 0.4),
            BODY,
            Vec3::new(-0.1, 0.13, 0.0),
        ),
        part(bow, BODY, Vec3::new(0.57, 0.13, 0.0)),
        part(
            Cuboid::new(0.3, 0.12, 0.34),
            DARK,
            Vec3::new(-0.38, 0.32, 0.0),
        ),
    ];
    // 돛대를 선체의 길이에 고르게 놓습니다. 가운데 돛대가 가장 높습니다.
    let places: &[(f32, f32)] = match masts {
        1 => &[(0.05, 0.85)],
        2 => &[(-0.12, 0.8), (0.25, 0.9)],
        _ => &[(-0.2, 0.75), (0.08, 0.95), (0.36, 0.8)],
    };
    for &(x, height) in places {
        let mast = Cylinder::new(0.025, height);
        parts.push(part(mast, DARK, Vec3::new(x, 0.26 + height / 2.0, 0.0)));
        let sail = Cuboid::new(0.03, height * 0.5, 0.6);
        parts.push(part(
            sail,
            LIGHT,
            Vec3::new(x + 0.04, 0.3 + height * 0.6, 0.0),
        ));
    }
    join(parts)
}

/// 둥근 몸통과 뾰족한 지붕입니다.
pub fn tower() -> Mesh {
    join(vec![
        part(Cylinder::new(0.5, 1.3), BODY, Vec3::new(0.0, 0.65, 0.0)),
        part(Cylinder::new(0.6, 0.15), DARK, Vec3::new(0.0, 1.35, 0.0)),
        part(Cone::new(0.55, 0.6), DARK, Vec3::new(0.0, 1.72, 0.0)),
    ])
}

/// 낮은 부두와 창고 두 채입니다.
pub fn harbor() -> Mesh {
    join(vec![
        part(Cuboid::new(2.0, 0.2, 2.0), DARK, Vec3::new(0.0, 0.1, 0.0)),
        part(Cuboid::new(0.9, 0.6, 1.5), BODY, Vec3::new(-0.4, 0.5, 0.0)),
        part(Cuboid::new(0.6, 0.4, 0.8), BODY, Vec3::new(0.55, 0.4, -0.4)),
        part(Cylinder::new(0.04, 0.9), DARK, Vec3::new(0.6, 0.65, 0.5)),
        part(
            Cuboid::new(0.3, 0.2, 0.02),
            LIGHT,
            Vec3::new(0.76, 1.0, 0.5),
        ),
    ])
}

/// 성벽, 네 모서리의 망루, 가운데의 본채입니다.
pub fn base() -> Mesh {
    let mut parts = vec![
        part(Cuboid::new(2.4, 0.8, 2.4), BODY, Vec3::new(0.0, 0.4, 0.0)),
        part(Cuboid::new(1.2, 0.9, 1.2), BODY, Vec3::new(0.0, 1.25, 0.0)),
        part(Cone::new(0.8, 0.6), DARK, Vec3::new(0.0, 2.0, 0.0)),
    ];
    for (x, z) in [(-1.2, -1.2), (-1.2, 1.2), (1.2, -1.2), (1.2, 1.2)] {
        parts.push(part(Cylinder::new(0.3, 1.3), BODY, Vec3::new(x, 0.65, z)));
        parts.push(part(Cone::new(0.36, 0.4), DARK, Vec3::new(x, 1.5, z)));
    }
    join(parts)
}

/// 교역항입니다. 낮은 부두와 지붕이 있는 시장입니다.
pub fn port() -> Mesh {
    join(vec![
        part(Cuboid::new(1.6, 0.2, 1.6), DARK, Vec3::new(0.0, 0.1, 0.0)),
        part(Cuboid::new(1.0, 0.5, 1.0), BODY, Vec3::new(0.0, 0.45, 0.0)),
        part(Cone::new(0.85, 0.5), LIGHT, Vec3::new(0.0, 0.95, 0.0)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 모양의 꼭짓점 가운데 가장 낮은 높이와 가장 높은 높이입니다.
    fn heights(mesh: &Mesh) -> (f32, f32) {
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
        let positions = positions.and_then(|positions| positions.as_float3());
        let ys = positions
            .expect("a shape has positions")
            .iter()
            .map(|p| p[1]);
        ys.fold((f32::MAX, f32::MIN), |(low, high), y| {
            (low.min(y), high.max(y))
        })
    }

    #[test]
    fn every_shape_stands_on_the_water_and_carries_a_color_for_each_vertex() {
        for (name, mesh) in [
            ("ship", ship(1)),
            ("big ship", ship(3)),
            ("tower", tower()),
            ("harbor", harbor()),
            ("base", base()),
            ("port", port()),
        ] {
            let (low, high) = heights(&mesh);
            assert!(low.abs() < 1e-4, "{name} starts at {low}");
            assert!(high > 0.5, "{name} is {high} high");
            let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR);
            let colors = colors.map(|colors| colors.len());
            assert_eq!(colors, Some(mesh.count_vertices()), "{name}");
        }
    }
}
