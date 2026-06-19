//! embed(M14 / Plan 14)— 图片嵌入生命周期 **FSM** + 描述符(core 持元数据,解码/纹理在 JS)。
//!
//! 一张图 = 一个 [`Embed`]:`Placeholder → Loading → Ready → Failed`(0007 embed FSM)。core 只持
//! `{url, alt, state, tex_id, 尺寸}`;**解码/栅格/纹理上传全在浏览器**(0011 §3.3),完成后回调
//! `image_ready`/`image_failed` 推进状态。未就绪/失败 → 显 alt 文本兜底(不阻塞主循环)。

/// 图片嵌入生命周期状态(0007 / Plan 14 §2.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbedState {
    /// 内容已确认是图(`)` 到达)→ 占位框(估高),尚未发起解码。
    #[default]
    Placeholder,
    /// 已发起浏览器解码(fetch/decode),等回调。
    Loading,
    /// 纹理就绪(`tex_id`):出图(alpha 淡入,0025);`animated` = 是否动图(v1 显首帧静态,§2.5)。
    Ready,
    /// 解码/网络失败 → 占位 + alt 文本兜底。
    Failed,
}

/// 一张图片嵌入的运行态(core 持;`range` 为块内 glyph 占位区间,见 [`crate::content::EmbedRegion`])。
#[derive(Debug, Clone, PartialEq)]
pub struct Embed {
    pub url: String,
    pub alt: String,
    pub state: EmbedState,
    /// JS 上传后的纹理 id(Ready 才有意义;0 = 无)。
    pub tex_id: u32,
    /// 解码回报的自然像素尺寸(Ready 才有;喂 Taffy `reportSize` 防 reflow,Plan 14 ④)。
    pub natural_size: Option<(f32, f32)>,
    /// 是否动图(GIF/动 WebP/APNG/动画 SVG;v1 显首帧静态,② DOM overlay 自播,§2.5)。
    pub animated: bool,
    /// 进入 Ready 的帧时刻(ms);用于 alpha 淡入(0025,Plan 14 ④)。None = 未就绪。
    pub ready_at: Option<f32>,
}

impl Embed {
    /// 新建占位态嵌入(content 确认是图时)。
    pub fn new(url: impl Into<String>, alt: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            alt: alt.into(),
            state: EmbedState::Placeholder,
            tex_id: 0,
            natural_size: None,
            animated: false,
            ready_at: None,
        }
    }

    /// 发起解码:Placeholder → Loading(幂等:已离开 Placeholder 则不变)。
    pub fn begin_loading(&mut self) {
        if self.state == EmbedState::Placeholder {
            self.state = EmbedState::Loading;
        }
    }

    /// 解码 + 纹理就绪:→ Ready,记 tex_id / 自然尺寸 / 动图标志 / 就绪时刻(`now` ms,淡入用)。
    /// Failed 后不复活(终态)。幂等重入不重置 `ready_at`(淡入不重启)。
    pub fn on_ready(&mut self, tex_id: u32, w: f32, h: f32, animated: bool, now: f32) {
        if self.state == EmbedState::Failed {
            return;
        }
        if self.ready_at.is_none() {
            self.ready_at = Some(now);
        }
        self.state = EmbedState::Ready;
        self.tex_id = tex_id;
        self.natural_size = Some((w.max(0.0), h.max(0.0)));
        self.animated = animated;
    }

    /// 当前 alpha(0025 淡入):Ready 起 `now-ready_at` 在 `fade_ms` 内 0→1;未就绪 = 0。
    pub fn alpha(&self, now: f32, fade_ms: f32) -> f32 {
        match self.ready_at {
            Some(t) if fade_ms > 0.0 => ((now - t) / fade_ms).clamp(0.0, 1.0),
            Some(_) => 1.0,
            None => 0.0,
        }
    }

    /// 解码/网络失败:→ Failed(显 alt 兜底)。Ready 后不回退(已出图,终态)。
    pub fn on_failed(&mut self) {
        if self.state != EmbedState::Ready {
            self.state = EmbedState::Failed;
        }
    }

    /// 是否该出纹理 quad(Ready 且有纹理)。
    pub fn is_drawable(&self) -> bool {
        self.state == EmbedState::Ready && self.tex_id != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsm_happy_path_placeholder_to_ready() {
        let mut e = Embed::new("http://x/a.png", "cat");
        assert_eq!(e.state, EmbedState::Placeholder);
        e.begin_loading();
        assert_eq!(e.state, EmbedState::Loading);
        e.on_ready(7, 320.0, 200.0, false, 1000.0);
        assert_eq!(e.state, EmbedState::Ready);
        assert_eq!(e.tex_id, 7);
        assert_eq!(e.natural_size, Some((320.0, 200.0)));
        assert!(e.is_drawable());
    }

    #[test]
    fn fsm_failed_is_terminal_and_falls_back_to_alt() {
        let mut e = Embed::new("http://x/404.png", "missing");
        e.begin_loading();
        e.on_failed();
        assert_eq!(e.state, EmbedState::Failed);
        assert!(!e.is_drawable());
        // Failed 后不被 ready 复活(终态)。
        e.on_ready(1, 10.0, 10.0, false, 0.0);
        assert_eq!(e.state, EmbedState::Failed);
        assert_eq!(e.alt, "missing"); // alt 兜底文本仍在
    }

    #[test]
    fn ready_does_not_regress_to_failed() {
        // 已出图后迟到的失败(如 overlay 卸载)不抹掉已就绪纹理。
        let mut e = Embed::new("u", "a");
        e.on_ready(3, 100.0, 50.0, true, 0.0);
        e.on_failed();
        assert_eq!(e.state, EmbedState::Ready);
        assert!(e.animated);
    }

    #[test]
    fn alpha_fades_in_after_ready() {
        // 0025 淡入:未就绪 = 0;就绪起 fade_ms 内线性 0→1;之后夹 1;重入不重启淡入。
        let mut e = Embed::new("u", "a");
        assert!(e.alpha(100.0, 200.0).abs() < 1e-6, "未就绪 alpha=0");
        e.on_ready(1, 10.0, 10.0, false, 1000.0);
        assert!(
            (e.alpha(1000.0, 200.0) - 0.0).abs() < 1e-6,
            "刚就绪 alpha≈0"
        );
        assert!(
            (e.alpha(1100.0, 200.0) - 0.5).abs() < 1e-6,
            "半程 alpha=0.5"
        );
        assert!(
            (e.alpha(1300.0, 200.0) - 1.0).abs() < 1e-6,
            "过淡入期 alpha=1"
        );
        e.on_ready(1, 10.0, 10.0, false, 5000.0); // 重入
        assert!(
            (e.alpha(1300.0, 200.0) - 1.0).abs() < 1e-6,
            "重入不重启淡入(ready_at 不变)"
        );
    }
}
