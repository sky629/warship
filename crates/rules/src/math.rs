//! 수면 좌표를 나타내는 2차원 벡터입니다. 단위는 타일이고, x는 동쪽, y는 북쪽입니다.
//!
//! 외부 수학 크레이트 대신 직접 정의합니다. 규칙 크레이트가 다른 크레이트의 타입을 노출하지 않게 하기 위해서입니다.
//! 이동과 거리 계산은 사칙연산과 제곱근만 씁니다. 이 연산은 어느 플랫폼에서나 같은 결과를 냅니다.

use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance(self, other: Vec2) -> f32 {
        (self - other).length()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}
