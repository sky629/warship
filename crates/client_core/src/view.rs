//! 화면을 보는 방법 가운데 Bevy가 필요 없는 계산입니다. 카메라가 보는 점과, 미니맵의 좌표입니다.
//!
//! 카메라는 처음에 내 배를 따라갑니다. 플레이어가 화면을 옮기면 따라가기를 멈추고, 다시 따라가게 하면 내 배로 돌아옵니다.
//! 미니맵은 창의 오른쪽 아래에 있고, 맵 전체를 타일마다 같은 수의 픽셀로 보여 줍니다.

use warship_rules::Vec2;

/// 카메라 거리의 배율이 1일 때 화면을 옮기는 속도(1초에 가는 타일의 수)입니다.
const PAN_SPEED: f32 = 24.0;
/// 카메라 거리의 배율의 범위입니다.
const ZOOM_RANGE: (f32, f32) = (0.4, 2.5);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraRig {
    /// 카메라가 보는 수면 위의 점(타일 좌표)입니다.
    pub focus: Vec2,
    /// 카메라 거리의 배율입니다.
    pub zoom: f32,
    /// 내 배를 따라가는지입니다.
    pub following: bool,
}

impl CameraRig {
    /// `focus`를 보면서 내 배를 따라가는 카메라입니다.
    pub fn new(focus: Vec2) -> CameraRig {
        CameraRig {
            focus,
            zoom: 1.0,
            following: true,
        }
    }

    /// 화면을 `direction` 쪽으로 `secs`초만큼 옮깁니다. x는 동쪽, y는 북쪽입니다. 방향이 없으면 아무 일도 하지 않습니다.
    /// 화면을 옮기면 내 배를 따라가기를 멈춥니다. 멀리서 볼수록 빨리 옮기고, 보는 점은 맵을 벗어나지 않습니다.
    pub fn pan(&mut self, direction: Vec2, secs: f32, map: Vec2) {
        if direction == Vec2::default() {
            return;
        }
        let moved = self.focus + direction * (PAN_SPEED * self.zoom * secs);
        self.look_at(moved, map);
    }

    /// 보는 점을 `point`로 옮기고 내 배를 따라가기를 멈춥니다. 맵 밖의 점은 맵 안으로 옮깁니다.
    pub fn look_at(&mut self, point: Vec2, map: Vec2) {
        self.following = false;
        self.focus = Vec2::new(point.x.clamp(0.0, map.x), point.y.clamp(0.0, map.y));
    }

    /// 내 배를 따라가는 동안에는 보는 점을 내 배의 자리로 옮깁니다. 내 배가 화면에 없으면 그대로입니다.
    pub fn track(&mut self, ship: Option<Vec2>) {
        if let (true, Some(ship)) = (self.following, ship) {
            self.focus = ship;
        }
    }

    /// 마우스 휠을 `wheel`칸 돌린 만큼 거리를 바꿉니다. 위로 돌리면 가까워집니다.
    pub fn zoom_by(&mut self, wheel: f32) {
        let (near, far) = ZOOM_RANGE;
        self.zoom = (self.zoom * (1.0 - wheel.clamp(-3.0, 3.0) * 0.1)).clamp(near, far);
    }
}

/// 창 안에서 미니맵이 차지하는 자리입니다. 단위는 창의 픽셀이고, 왼쪽 위가 (0, 0)입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Minimap {
    pub left: f32,
    pub top: f32,
    /// 맵의 크기(타일)입니다.
    pub map: Vec2,
}

impl Minimap {
    /// 타일 하나의 한 변의 길이(픽셀)입니다.
    pub const PX_PER_TILE: f32 = 3.0;
    /// 창의 오른쪽과 아래쪽 가장자리에서 떨어진 거리(픽셀)입니다.
    pub const MARGIN: f32 = 10.0;

    /// `window` 크기의 창의 오른쪽 아래에 놓인 미니맵입니다.
    pub fn in_window(window: (f32, f32), map: Vec2) -> Minimap {
        let (width, height) = (map.x * Self::PX_PER_TILE, map.y * Self::PX_PER_TILE);
        Minimap {
            left: window.0 - Self::MARGIN - width,
            top: window.1 - Self::MARGIN - height,
            map,
        }
    }

    /// 미니맵의 크기(픽셀)입니다.
    pub fn size(&self) -> (f32, f32) {
        (
            self.map.x * Self::PX_PER_TILE,
            self.map.y * Self::PX_PER_TILE,
        )
    }

    /// 창의 `cursor` 자리가 미니맵 위이면, 그 자리가 가리키는 타일 좌표입니다. 미니맵의 위쪽이 북쪽입니다.
    pub fn tile_at(&self, cursor: (f32, f32)) -> Option<Vec2> {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top);
        let (width, height) = self.size();
        let inside = (0.0..=width).contains(&x) && (0.0..=height).contains(&y);
        inside.then(|| Vec2::new(x / Self::PX_PER_TILE, self.map.y - y / Self::PX_PER_TILE))
    }

    /// 타일 좌표의 점이 미니맵의 왼쪽 위에서 얼마나 떨어져 있는지(픽셀)입니다.
    pub fn dot(&self, tile: Vec2) -> (f32, f32) {
        (
            tile.x * Self::PX_PER_TILE,
            (self.map.y - tile.y) * Self::PX_PER_TILE,
        )
    }
}

/// 창의 왼쪽 위에 놓이는 상점 창의 자리(픽셀)입니다. 줄의 높이가 모두 같아서, 커서의 자리로 줄을 찾습니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopWindow {
    pub left: f32,
    pub top: f32,
    pub rows: usize,
}

impl ShopWindow {
    pub const WIDTH: f32 = 460.0;
    /// 줄 하나의 높이입니다.
    pub const ROW: f32 = 20.0;
    /// 창의 왼쪽 가장자리와 상점 창 사이의 간격입니다.
    pub const MARGIN: f32 = 10.0;
    /// 창의 위쪽 가장자리와 상점 창 사이의 간격입니다. 맨 위의 상태 줄 두 줄을 가리지 않습니다.
    pub const TOP: f32 = 60.0;

    /// 줄이 `rows`개인 상점 창의 자리입니다.
    pub fn with_rows(rows: usize) -> ShopWindow {
        ShopWindow {
            left: Self::MARGIN,
            top: Self::TOP,
            rows,
        }
    }

    pub fn height(&self) -> f32 {
        self.rows as f32 * Self::ROW
    }

    /// 창의 `cursor` 자리가 상점 창 위이면, 그 자리에 있는 줄의 번호입니다.
    pub fn row_at(&self, cursor: (f32, f32)) -> Option<usize> {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top);
        let inside = (0.0..Self::WIDTH).contains(&x) && (0.0..self.height()).contains(&y);
        inside.then(|| (y / Self::ROW) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: Vec2 = Vec2::new(64.0, 48.0);

    #[test]
    fn the_camera_follows_my_ship_until_the_player_moves_the_view() {
        let mut rig = CameraRig::new(Vec2::new(32.0, 24.0));
        rig.track(Some(Vec2::new(7.0, 24.0)));
        assert_eq!(rig.focus, Vec2::new(7.0, 24.0));
        rig.track(None);
        assert_eq!(
            rig.focus,
            Vec2::new(7.0, 24.0),
            "my ship is not on the screen"
        );

        rig.pan(Vec2::new(1.0, 0.0), 0.5, MAP);
        assert_eq!(rig.focus, Vec2::new(19.0, 24.0), "24 tiles a second");
        rig.track(Some(Vec2::new(8.0, 24.0)));
        assert_eq!(
            rig.focus,
            Vec2::new(19.0, 24.0),
            "the view stays where it was put"
        );

        rig.following = true;
        rig.track(Some(Vec2::new(8.0, 24.0)));
        assert_eq!(rig.focus, Vec2::new(8.0, 24.0));
    }

    #[test]
    fn a_pan_without_a_direction_keeps_following_and_the_view_stays_on_the_map() {
        let mut rig = CameraRig::new(Vec2::new(2.0, 2.0));
        rig.pan(Vec2::default(), 1.0, MAP);
        assert!(rig.following);

        rig.pan(Vec2::new(-1.0, 1.0), 10.0, MAP);
        assert_eq!(rig.focus, Vec2::new(0.0, 48.0));
        rig.look_at(Vec2::new(500.0, -3.0), MAP);
        assert_eq!(rig.focus, Vec2::new(64.0, 0.0));
    }

    #[test]
    fn the_wheel_changes_the_distance_within_its_range_and_a_far_view_pans_faster() {
        let mut rig = CameraRig::new(Vec2::default());
        rig.zoom_by(1.0);
        assert!((rig.zoom - 0.9).abs() < 1e-6, "one step closer");
        for _ in 0..100 {
            rig.zoom_by(-3.0);
        }
        assert_eq!(rig.zoom, 2.5);

        rig.pan(Vec2::new(1.0, 0.0), 0.5, MAP);
        assert_eq!(rig.focus.x, 30.0);
    }

    #[test]
    fn the_minimap_sits_in_the_lower_right_corner_with_north_at_its_top() {
        let minimap = Minimap::in_window((1000.0, 700.0), MAP);
        assert_eq!(minimap.size(), (192.0, 144.0));
        assert_eq!((minimap.left, minimap.top), (798.0, 546.0));

        // 미니맵의 왼쪽 위는 맵의 북서쪽 끝이고, 오른쪽 아래는 남동쪽 끝입니다.
        assert_eq!(minimap.tile_at((798.0, 546.0)), Some(Vec2::new(0.0, 48.0)));
        assert_eq!(minimap.tile_at((990.0, 690.0)), Some(Vec2::new(64.0, 0.0)));
        assert_eq!(minimap.tile_at((797.0, 600.0)), None);
        assert_eq!(minimap.tile_at((900.0, 691.0)), None);
    }

    #[test]
    fn a_dot_on_the_minimap_points_back_at_its_tile() {
        let minimap = Minimap::in_window((1000.0, 700.0), MAP);
        let tile = Vec2::new(20.0, 36.0);

        let (x, y) = minimap.dot(tile);

        assert_eq!((x, y), (60.0, 36.0));
        let cursor = (minimap.left + x, minimap.top + y);
        assert_eq!(minimap.tile_at(cursor), Some(tile));
    }

    #[test]
    fn the_shop_window_sits_in_the_upper_left_corner_and_finds_the_row_under_the_cursor() {
        let shop = ShopWindow::with_rows(5);
        assert_eq!((shop.left, shop.top, shop.height()), (10.0, 60.0, 100.0));

        assert_eq!(shop.row_at((10.0, 60.0)), Some(0));
        assert_eq!(shop.row_at((469.0, 79.0)), Some(0));
        assert_eq!(shop.row_at((200.0, 80.0)), Some(1));
        assert_eq!(shop.row_at((200.0, 159.0)), Some(4));
        assert_eq!(shop.row_at((200.0, 160.0)), None, "below the last row");
        assert_eq!(shop.row_at((9.0, 100.0)), None);
        assert_eq!(shop.row_at((470.0, 100.0)), None);
        assert_eq!(shop.row_at((200.0, 59.0)), None, "the status lines");
    }
}
