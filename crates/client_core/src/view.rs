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
    pub const PX_PER_TILE: f32 = 2.0;
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

/// 창의 왼쪽 위에 놓이는 상점 창의 자리(픽셀)입니다. 위에서부터 탭 줄, 열린 탭의 줄(두 열), 커서 아래의 줄을 설명하는 줄입니다.
/// 탭의 너비, 줄의 높이, 열의 너비가 모두 같아서, 커서의 자리로 탭과 줄을 찾습니다. 줄은 첫 열을 위에서부터 채우고 둘째 열로 이어집니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopWindow {
    pub left: f32,
    pub top: f32,
    pub tabs: usize,
    /// 열린 탭의 줄의 수입니다.
    pub rows: usize,
    pub columns: usize,
}

impl ShopWindow {
    /// 열 하나의 너비입니다.
    pub const COLUMN: f32 = 350.0;
    /// 탭 줄의 높이입니다.
    pub const TAB: f32 = 28.0;
    /// 줄 하나의 높이입니다. 아이콘과 두 줄의 글자가 들어갑니다.
    pub const ROW: f32 = 40.0;
    /// 설명 줄의 높이입니다.
    pub const FOOTER: f32 = 24.0;
    /// 창의 왼쪽 가장자리와 상점 창 사이의 간격입니다.
    pub const MARGIN: f32 = 10.0;
    /// 창의 위쪽 가장자리와 상점 창 사이의 간격입니다. 맨 위의 상태 막대와 안내 줄을 가리지 않습니다.
    pub const TOP: f32 = 60.0;

    /// 너비가 `window_width`인 창에서, 탭이 `tabs`개이고 열린 탭의 줄이 `rows`개인 상점 창의 자리입니다.
    /// 창이 두 열을 놓기에 좁으면 한 열로 놓습니다.
    pub fn in_window(window_width: f32, tabs: usize, rows: usize) -> ShopWindow {
        let fits = window_width >= Self::COLUMN * 2.0 + Self::MARGIN * 2.0;
        ShopWindow {
            left: Self::MARGIN,
            top: Self::TOP,
            tabs,
            rows,
            columns: if fits { 2 } else { 1 },
        }
    }

    pub fn width(&self) -> f32 {
        Self::COLUMN * self.columns as f32
    }

    /// 열 하나에 놓이는 줄의 수입니다.
    fn lines(&self) -> usize {
        self.rows.div_ceil(self.columns)
    }

    /// 탭 줄, 줄, 설명 줄을 합한 높이입니다.
    pub fn height(&self) -> f32 {
        Self::TAB + self.lines() as f32 * Self::ROW + Self::FOOTER
    }

    pub fn tab_width(&self) -> f32 {
        self.width() / self.tabs.max(1) as f32
    }

    /// `index`번 줄이 상점 창의 왼쪽 위에서 얼마나 떨어져 있는지입니다.
    pub fn place(&self, index: usize) -> (f32, f32) {
        let lines = self.lines().max(1);
        let (column, line) = (index / lines, index % lines);
        (
            column as f32 * Self::COLUMN,
            Self::TAB + line as f32 * Self::ROW,
        )
    }

    /// 설명 줄이 상점 창의 왼쪽 위에서 얼마나 떨어져 있는지입니다.
    pub fn footer(&self) -> (f32, f32) {
        (0.0, Self::TAB + self.lines() as f32 * Self::ROW)
    }

    /// 창의 `cursor` 자리가 상점 창 위인지입니다.
    pub fn covers(&self, cursor: (f32, f32)) -> bool {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top);
        (0.0..self.width()).contains(&x) && (0.0..self.height()).contains(&y)
    }

    /// 창의 `cursor` 자리에 탭이 있으면 그 탭의 번호입니다.
    pub fn tab_at(&self, cursor: (f32, f32)) -> Option<usize> {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top);
        let on_tabs = self.covers(cursor) && y < Self::TAB;
        on_tabs.then(|| ((x / self.tab_width()) as usize).min(self.tabs.saturating_sub(1)))
    }

    /// 창의 `cursor` 자리에 열린 탭의 줄이 있으면 그 줄의 번호입니다.
    pub fn row_at(&self, cursor: (f32, f32)) -> Option<usize> {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top - Self::TAB);
        if !self.covers(cursor) || y < 0.0 {
            return None;
        }
        let (column, line) = ((x / Self::COLUMN) as usize, (y / Self::ROW) as usize);
        let index = column * self.lines() + line;
        (line < self.lines() && index < self.rows).then_some(index)
    }
}

/// 화면의 왼쪽 아래, 내 배 패널의 위에 놓이는 칸 줄의 자리(픽셀)입니다. 칸은 왼쪽부터 차례로 놓입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MountBar {
    pub left: f32,
    pub top: f32,
    pub slots: usize,
}

impl MountBar {
    /// 칸 하나의 너비와 높이입니다.
    pub const SLOT: (f32, f32) = (120.0, 34.0);
    /// 칸과 칸 사이의 간격입니다.
    pub const GAP: f32 = 4.0;
    /// 창의 왼쪽 가장자리와 칸 줄 사이의 간격입니다.
    pub const MARGIN: f32 = 10.0;
    /// 창의 아래쪽 가장자리에서 칸 줄의 위쪽까지의 거리입니다. 그 아래에 내 배 패널이 있습니다.
    pub const LIFT: f32 = 166.0;

    /// 높이가 `window_height`인 창에서, 칸이 `slots`개인 칸 줄의 자리입니다.
    pub fn in_window(window_height: f32, slots: usize) -> MountBar {
        MountBar {
            left: Self::MARGIN,
            top: window_height - Self::LIFT,
            slots,
        }
    }

    pub fn width(&self) -> f32 {
        self.slots as f32 * (Self::SLOT.0 + Self::GAP) - Self::GAP
    }

    /// `slot`번 칸이 칸 줄의 왼쪽에서 얼마나 떨어져 있는지입니다.
    pub fn place(&self, slot: usize) -> f32 {
        slot as f32 * (Self::SLOT.0 + Self::GAP)
    }

    /// 창의 `cursor` 자리에 칸이 있으면 그 칸의 번호입니다. 칸과 칸 사이의 간격은 칸이 아닙니다.
    pub fn slot_at(&self, cursor: (f32, f32)) -> Option<usize> {
        let (x, y) = (cursor.0 - self.left, cursor.1 - self.top);
        if x < 0.0 || !(0.0..Self::SLOT.1).contains(&y) {
            return None;
        }
        let pitch = Self::SLOT.0 + Self::GAP;
        let slot = (x / pitch) as usize;
        (slot < self.slots && x - slot as f32 * pitch < Self::SLOT.0).then_some(slot)
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
        assert_eq!(minimap.size(), (128.0, 96.0));
        assert_eq!((minimap.left, minimap.top), (862.0, 594.0));

        // 미니맵의 왼쪽 위는 맵의 북서쪽 끝이고, 오른쪽 아래는 남동쪽 끝입니다.
        assert_eq!(minimap.tile_at((862.0, 594.0)), Some(Vec2::new(0.0, 48.0)));
        assert_eq!(minimap.tile_at((990.0, 690.0)), Some(Vec2::new(64.0, 0.0)));
        assert_eq!(minimap.tile_at((861.0, 600.0)), None);
        assert_eq!(minimap.tile_at((900.0, 691.0)), None);
    }

    #[test]
    fn a_dot_on_the_minimap_points_back_at_its_tile() {
        let minimap = Minimap::in_window((1000.0, 700.0), MAP);
        let tile = Vec2::new(20.0, 36.0);

        let (x, y) = minimap.dot(tile);

        assert_eq!((x, y), (40.0, 24.0));
        let cursor = (minimap.left + x, minimap.top + y);
        assert_eq!(minimap.tile_at(cursor), Some(tile));
    }

    #[test]
    fn the_shop_window_has_tabs_two_columns_and_finds_what_is_under_the_cursor() {
        // 탭이 넷이고, 열린 탭의 다섯 줄이 첫 열에 셋, 둘째 열에 둘 놓입니다.
        let shop = ShopWindow::in_window(1000.0, 4, 5);
        assert_eq!((shop.left, shop.top), (10.0, 60.0));
        assert_eq!((shop.width(), shop.height()), (700.0, 172.0));
        assert_eq!(shop.tab_width(), 175.0);
        assert_eq!(shop.place(0), (0.0, 28.0));
        assert_eq!(shop.place(2), (0.0, 108.0));
        assert_eq!(shop.place(4), (350.0, 68.0));
        assert_eq!(shop.footer(), (0.0, 148.0));

        assert_eq!(shop.tab_at((10.0, 60.0)), Some(0));
        assert_eq!(shop.tab_at((185.0, 87.0)), Some(1));
        assert_eq!(shop.tab_at((709.0, 70.0)), Some(3));
        assert_eq!(shop.tab_at((200.0, 88.0)), None, "the rows");

        assert_eq!(shop.row_at((10.0, 88.0)), Some(0));
        assert_eq!(shop.row_at((359.0, 127.0)), Some(0));
        assert_eq!(shop.row_at((200.0, 128.0)), Some(1));
        assert_eq!(shop.row_at((360.0, 88.0)), Some(3), "the second column");
        assert_eq!(shop.row_at((500.0, 167.0)), Some(4));
        assert_eq!(shop.row_at((500.0, 168.0)), None, "the short column ends");
        assert_eq!(shop.row_at((200.0, 208.0)), None, "the detail line");
        assert_eq!(shop.row_at((200.0, 70.0)), None, "the tabs");

        assert!(shop.covers((200.0, 231.0)));
        assert!(!shop.covers((200.0, 232.0)));
        assert!(!shop.covers((710.0, 100.0)));
        assert!(!shop.covers((9.0, 100.0)));
        assert!(!shop.covers((200.0, 59.0)));
    }

    #[test]
    fn a_narrow_window_gets_the_shop_in_one_column() {
        assert_eq!(ShopWindow::in_window(720.0, 4, 5).columns, 2);

        let shop = ShopWindow::in_window(719.0, 4, 5);
        assert_eq!(shop.columns, 1);
        assert_eq!((shop.width(), shop.height()), (350.0, 252.0));
        assert_eq!(shop.place(4), (0.0, 188.0));
        assert_eq!(shop.row_at((200.0, 60.0 + 28.0 + 4.0 * 40.0)), Some(4));
        assert!(!shop.covers((360.0, 100.0)), "there is no second column");
    }

    #[test]
    fn the_mount_row_sits_above_the_ship_panel_and_finds_the_slot_under_the_cursor() {
        let bar = MountBar::in_window(736.0, 3);
        assert_eq!((bar.left, bar.top, bar.width()), (10.0, 570.0, 368.0));
        assert_eq!(bar.place(2), 248.0);

        assert_eq!(bar.slot_at((10.0, 570.0)), Some(0));
        assert_eq!(bar.slot_at((129.0, 603.0)), Some(0));
        assert_eq!(bar.slot_at((131.0, 580.0)), None, "the gap");
        assert_eq!(bar.slot_at((134.0, 580.0)), Some(1));
        assert_eq!(bar.slot_at((377.0, 580.0)), Some(2));
        assert_eq!(bar.slot_at((400.0, 580.0)), None, "past the last slot");
        assert_eq!(bar.slot_at((50.0, 604.0)), None);
        assert_eq!(bar.slot_at((50.0, 569.0)), None);
        assert_eq!(bar.slot_at((9.0, 580.0)), None);
    }
}
