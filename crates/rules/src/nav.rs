//! 섬을 피해서 가는 길입니다. 섬은 원이고, 배는 섬 위를 지나가지 못합니다. 포탄과 시야는 섬에 막히지 않습니다.
//!
//! 길은 선분의 목록입니다. 출발점에서 목적지까지 곧게 갈 수 있으면 선분 하나이고,
//! 아니면 섬마다 둘레에 놓은 지점들을 거쳐 가는 가장 짧은 길입니다.
//! NPC 배는 길을 찾지 않습니다. 라인이 섬을 지나지 않는다는 것을 `Tuning::validate`가 확인합니다.

use crate::math::Vec2;
use crate::tuning::{Island, MapSpec};

/// 목적지와 둘레의 지점이 섬의 가장자리에서 떨어지는 거리(타일)입니다.
pub(crate) const MARGIN: f32 = 0.25;
/// 섬과 섬 사이, 섬과 맵의 가장자리 사이에 있어야 하는 물길의 너비(타일)입니다.
pub(crate) const CHANNEL: f32 = 1.0;
/// 섬 하나의 둘레에 놓는 지점의 수입니다.
const RING: usize = 12;
/// 정십이각형의 꼭짓점까지의 거리를 변까지의 거리로 나눈 값입니다(1 / cos 15°).
/// 둘레의 지점을 이만큼 더 멀리 놓으면, 이웃한 두 지점을 잇는 선분이 섬에서 `MARGIN`만큼 떨어집니다.
const RING_REACH: f32 = 1.035_276_2;
/// 둘레의 지점이 놓이는 방향입니다(15°부터 30° 간격의 cos, sin). 삼각함수를 실행 중에 계산하지 않기 위해 적어 둡니다.
const RING_DIRECTIONS: [(f32, f32); RING] = {
    let (a, b) = (0.965_925_8, 0.258_819_05);
    let c = 0.707_106_77;
    [
        (a, b),
        (c, c),
        (b, a),
        (-b, a),
        (-c, c),
        (-a, b),
        (-a, -b),
        (-c, -c),
        (-b, -a),
        (b, -a),
        (c, -c),
        (a, -b),
    ]
};

impl Island {
    /// `a`에서 `b`로 곧게 가는 배가 이 섬에 막히는지입니다.
    /// 섬에서 멀어지는 방향으로 떠나는 선분은 막히지 않습니다. 그래서 섬의 가장자리에 붙어 있는 배도 항상 떠날 수 있습니다.
    fn blocks(&self, a: Vec2, b: Vec2) -> bool {
        let (along, to_center) = (b - a, self.center - a);
        let toward = along.x * to_center.x + along.y * to_center.y;
        if toward <= 0.0 {
            return false;
        }
        let length_squared = along.x * along.x + along.y * along.y;
        let nearest = a + along * (toward / length_squared).min(1.0);
        nearest.distance(self.center) < self.radius
    }

    /// 이 섬의 가장자리에서 `MARGIN` 안쪽에 있는 점인지입니다.
    fn crowds(&self, p: Vec2) -> bool {
        p.distance(self.center) < self.radius + MARGIN
    }
}

/// `a`에서 `b`로 곧게 갈 수 있는지입니다.
pub(crate) fn clear(map: &MapSpec, a: Vec2, b: Vec2) -> bool {
    !map.islands.iter().any(|island| island.blocks(a, b))
}

/// `p`가 어느 섬에서도 `MARGIN` 이상 떨어져 있는지입니다.
pub(crate) fn is_open(map: &MapSpec, p: Vec2) -> bool {
    !map.islands.iter().any(|island| island.crowds(p))
}

/// 배가 설 수 있는 점 가운데 `p`와 가까운 점입니다. 맵 밖의 점은 맵 안으로, 섬 위의 점은 그 섬의 가장자리 밖으로 옮깁니다.
pub(crate) fn settle(map: &MapSpec, p: Vec2) -> Vec2 {
    let p = map.clamp(p);
    let Some(island) = map.islands.iter().find(|island| island.crowds(p)) else {
        return p;
    };
    let offset = p - island.center;
    let distance = offset.length();
    // 섬의 한가운데를 고른 경우에는 동쪽 가장자리로 옮깁니다.
    let direction = if distance > 0.0 {
        offset * (1.0 / distance)
    } else {
        Vec2::new(1.0, 0.0)
    };
    island.center + direction * (island.radius + MARGIN)
}

/// `from`에서 `to`까지 가는 가장 짧은 길의 지점들입니다. 마지막 지점이 `to`입니다. 길이 없으면 비어 있습니다.
/// `to`는 `settle`을 거친 점이어야 합니다.
pub(crate) fn route(map: &MapSpec, from: Vec2, to: Vec2) -> Vec<Vec2> {
    if clear(map, from, to) {
        return vec![to];
    }
    const START: usize = 0;
    const GOAL: usize = 1;
    let mut nodes = vec![from, to];
    for island in &map.islands {
        let reach = (island.radius + MARGIN) * RING_REACH;
        for (cos, sin) in RING_DIRECTIONS {
            let node = island.center + Vec2::new(cos, sin) * reach;
            if map.contains(node) && is_open(map, node) {
                nodes.push(node);
            }
        }
    }

    // 지점의 수가 적으므로 모든 지점을 훑는 다익스트라 알고리즘을 씁니다. 두 지점이 곧게 이어지는지는 필요할 때 확인합니다.
    let mut cost = vec![f32::INFINITY; nodes.len()];
    let mut previous = vec![START; nodes.len()];
    let mut settled = vec![false; nodes.len()];
    cost[START] = 0.0;
    loop {
        let nearest = (0..nodes.len())
            .filter(|&node| !settled[node] && cost[node].is_finite())
            .min_by(|&a, &b| cost[a].total_cmp(&cost[b]));
        let Some(current) = nearest else {
            return Vec::new();
        };
        if current == GOAL {
            break;
        }
        settled[current] = true;
        for next in 0..nodes.len() {
            let through = cost[current] + nodes[current].distance(nodes[next]);
            if !settled[next] && through < cost[next] && clear(map, nodes[current], nodes[next]) {
                cost[next] = through;
                previous[next] = current;
            }
        }
    }
    let mut route = Vec::new();
    let mut node = GOAL;
    while node != START {
        route.push(nodes[node]);
        node = previous[node];
    }
    route.reverse();
    route
}
