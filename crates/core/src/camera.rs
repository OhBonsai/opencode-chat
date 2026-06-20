//! camera(M10/M13)— 2D 相机(Plan 3 L1)。
//!
//! 无边画布的平移 + 缩放。约定:`screen = (world - pan) * zoom`,`pan` 是屏幕左上角对应的
//! 世界坐标。相机变换在着色器里做(顶点喂世界坐标);此处只算可见世界矩形(喂空间索引)
//! 与屏幕↔世界换算(hit-test 用 base 位置,§3.2)。纯数学,native 可测(CR1)。

/// 世界坐标轴对齐矩形 `[x, y, w, h]`(左上 + 宽高)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// 两矩形是否相交(边界相接算相交)。
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x <= o.x + o.w
            && o.x <= self.x + self.w
            && self.y <= o.y + o.h
            && o.y <= self.y + self.h
    }

    /// 点 `(px,py)` 是否在矩形内(含左/上边,不含右/下边)。
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// 与另一矩形的重叠面积(无交叠 = 0)。ShaderBox 度量「屏上像素」用(Plan 16 §2.4)。
    pub fn overlap_area(&self, o: &Rect) -> f32 {
        let ix = (self.x + self.w).min(o.x + o.w) - self.x.max(o.x);
        let iy = (self.y + self.h).min(o.y + o.h) - self.y.max(o.y);
        if ix <= 0.0 || iy <= 0.0 {
            0.0
        } else {
            ix * iy
        }
    }
}

/// 2D 相机。
#[derive(Debug, Clone, Copy)]
pub struct Camera2D {
    /// 屏幕左上角对应的世界坐标。
    pan: [f32; 2],
    /// 缩放(>1 放大)。
    zoom: f32,
    /// 视口像素尺寸。
    viewport: [f32; 2],
}

impl Camera2D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            pan: [0.0, 0.0],
            zoom: 1.0,
            viewport: [viewport_w.max(1.0), viewport_h.max(1.0)],
        }
    }

    pub fn pan(&self) -> [f32; 2] {
        self.pan
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn viewport(&self) -> [f32; 2] {
        self.viewport
    }

    pub fn set_viewport(&mut self, w: f32, h: f32) {
        self.viewport = [w.max(1.0), h.max(1.0)];
    }

    /// 平移 `dx,dy`(屏幕像素;内部换成世界位移)。
    pub fn pan_by_screen(&mut self, dx: f32, dy: f32) {
        self.pan[0] += dx / self.zoom;
        self.pan[1] += dy / self.zoom;
    }

    /// 直接设世界 pan(锚底等策略用)。
    pub fn set_pan(&mut self, x: f32, y: f32) {
        self.pan = [x, y];
    }

    /// 围绕屏幕点 `(sx,sy)` 缩放 `factor`(该点世界坐标保持不动)。zoom 夹到 [0.1, 10]。
    pub fn zoom_at(&mut self, factor: f32, sx: f32, sy: f32) {
        let before = self.screen_to_world([sx, sy]);
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        // 调 pan 使该屏幕点仍对到同一世界点。
        self.pan[0] = before[0] - sx / self.zoom;
        self.pan[1] = before[1] - sy / self.zoom;
    }

    pub fn world_to_screen(&self, w: [f32; 2]) -> [f32; 2] {
        [
            (w[0] - self.pan[0]) * self.zoom,
            (w[1] - self.pan[1]) * self.zoom,
        ]
    }

    pub fn screen_to_world(&self, s: [f32; 2]) -> [f32; 2] {
        [
            self.pan[0] + s[0] / self.zoom,
            self.pan[1] + s[1] / self.zoom,
        ]
    }

    /// 当前视口覆盖的世界矩形(喂空间索引做 2D 裁剪)。
    pub fn visible_world_rect(&self) -> Rect {
        Rect::new(
            self.pan[0],
            self.pan[1],
            self.viewport[0] / self.zoom,
            self.viewport[1] / self.zoom,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_world_roundtrip() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.set_pan(100.0, 50.0);
        cam.zoom_at(2.0, 0.0, 0.0); // 围绕屏幕原点放大
        let w = [123.0, 456.0];
        let s = cam.world_to_screen(w);
        let back = cam.screen_to_world(s);
        assert!((back[0] - w[0]).abs() < 1e-3 && (back[1] - w[1]).abs() < 1e-3);
    }

    #[test]
    fn zoom_keeps_anchor_point_fixed() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let (sx, sy) = (400.0, 300.0);
        let before = cam.screen_to_world([sx, sy]);
        cam.zoom_at(1.5, sx, sy);
        let after = cam.screen_to_world([sx, sy]);
        assert!((before[0] - after[0]).abs() < 1e-3, "{before:?} {after:?}");
        assert!((before[1] - after[1]).abs() < 1e-3);
        assert!((cam.zoom() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn visible_rect_scales_with_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        let r1 = cam.visible_world_rect();
        assert_eq!((r1.w, r1.h), (800.0, 600.0)); // zoom 1
        cam.zoom_at(2.0, 0.0, 0.0);
        let r2 = cam.visible_world_rect();
        assert_eq!((r2.w, r2.h), (400.0, 300.0)); // 放大 → 看得少
    }

    #[test]
    fn pan_by_screen_respects_zoom() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.zoom_at(2.0, 0.0, 0.0);
        cam.pan_by_screen(0.0, 100.0); // 屏幕 100px → 世界 50
        assert!((cam.pan()[1] - 50.0).abs() < 1e-3);
    }

    #[test]
    fn overlap_area_clips_to_intersection() {
        // ShaderBox 度量(Plan 16 §2.4):重叠 = 交集面积,无交 = 0,全覆盖 = 自身面积。
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!((a.overlap_area(&Rect::new(5.0, 5.0, 10.0, 10.0)) - 25.0).abs() < 1e-3); // 5×5 角交
        assert!(a.overlap_area(&Rect::new(20.0, 0.0, 5.0, 5.0)).abs() < 1e-3); // 右侧无交
        assert!((a.overlap_area(&a) - 100.0).abs() < 1e-3); // 全覆盖
    }
}
