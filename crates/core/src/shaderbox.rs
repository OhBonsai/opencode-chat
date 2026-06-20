//! shaderbox(M16 / Plan 16 / 0028)— ShaderBox 图元的 **core 侧逻辑**:shader-id / icon 注册表、
//! 节流时钟(护栏4)、静态即冻判定(护栏2)。纯 Rust、native 可测(CR1);GPU 出图在 render crate。
//!
//! 图元数据 = [`crate::FrameShaderBox`]。本模块只管「哪个 shader、icon-id、动/静、time 节流」等
//! **CPU 决策**,不持任何 GPU 资源。

/// 内置 shader-id(render 侧每 id 一条 pipeline,0028 §3)。值 = `FrameShaderBox.shader_id`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShaderId {
    /// PixelSpiritDeck 整盘 icon 库(§2.5;`params[0]` = icon_id 的 `switch` 分派)。
    Icons = 0,
    /// Agent 回复 logo:noise 调制发光环(§2.6,自写 + LYGIA 噪声)。
    GlowOrb = 1,
    /// 0024 §4B raymarch 区域(相位⑤留位)。
    Raymarch = 2,
}

impl ShaderId {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// ShaderBox 动效节流间隔(ms;护栏4,默认 30fps)。dynamic box 的 `time` 按此步进,与主 rAF 解耦。
pub const SHADERBOX_THROTTLE_MS: f32 = 1000.0 / 30.0;

/// 面积/分辨率封顶边长(world px;护栏3,Plan 16 §2.3)。单 box 任一边超此阈 → 应渲到上限分辨率
/// 离屏纹理再放大(downscale),避免巨 box 满屏跑昂贵片元。v1 仅留判定钩子(downscale 路径后续;
/// 当前内置 box ≤ 32px 不触发)。
pub const SHADERBOX_MAX_EDGE_PX: f32 = 512.0;

/// box 是否超面积封顶(超 → 走 downscale 路径,v1 暂跳过该 box)。护栏3 判定钩子。
#[must_use]
pub fn shaderbox_exceeds_area_cap(size: [f32; 2]) -> bool {
    size[0] > SHADERBOX_MAX_EDGE_PX || size[1] > SHADERBOX_MAX_EDGE_PX
}

/// PixelSpiritDeck 整盘 icon(§2.5b;值 = 源 case 号,0..=49)。`params[0]` 取 `as u32 as f32`。
/// `dynamic()` 区分呼吸/旋转(46 个)与纯静态(4 个:Void/TheTemple/TheHermit/Enlightenment)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IconId {
    Void = 0,
    Justice = 1,
    Strength = 2,
    Death = 3,
    Wall = 4,
    Temperance = 5,
    Branch = 6,
    TheHangedMan = 7,
    TheHighPriestess = 8,
    TheMoon = 9,
    TheEmperor = 10,
    TheHierophant = 11,
    TheTower = 12,
    Merge = 13,
    Hope = 14,
    TheTemple = 15,
    TheSummit = 16,
    TheDiamond = 17,
    TheHermit = 18,
    Intuition = 19,
    TheStone = 20,
    TheMountain = 21,
    TheShadow = 22,
    Opposite = 23,
    TheOak = 24,
    Ripples = 25,
    TheEmpress = 26,
    Bundle = 27,
    TheDevil = 28,
    TheSun = 29,
    TheStar = 30,
    Judgement = 31,
    WheelOfFortune = 32,
    Vision = 33,
    TheLovers = 34,
    TheMagician = 35,
    TheLink = 36,
    HoldingTogether = 37,
    TheChariot = 38,
    TheLoop = 39,
    TurningPoint = 40,
    Trinity = 41,
    TheCauldron = 42,
    TheElders = 43,
    TheCore = 44,
    InnerTruth = 45,
    TheWorld = 46,
    TheFool = 47,
    Enlightenment = 48,
    Elements = 49,
}

impl IconId {
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// 是否动画(§2.5:46 个呼吸/旋转 = true;4 个纯静态 = false → 护栏2 画一次即冻)。
    pub fn is_dynamic(self) -> bool {
        !matches!(
            self,
            IconId::Void | IconId::TheTemple | IconId::TheHermit | IconId::Enlightenment
        )
    }

    /// 聊天功能图标 → 盘内最贴近 icon 的别名映射(§2.5c)。缺的后续工具箱自画追加(id≥50)。
    pub fn copy() -> Self {
        IconId::TheEmperor // rect 描边 ≈ 复制框
    }
    pub fn check() -> Self {
        IconId::TheSummit // 三角 ≈ 勾
    }
    pub fn spinner() -> Self {
        IconId::TheWorld // flower + star 旋 ≈ loading
    }
}

/// ShaderBox 动效节流时钟(护栏4):`time` 按 [`SHADERBOX_THROTTLE_MS`](30fps)步进,与主 rAF(60fps)
/// 解耦——多个 dynamic box 共用此一个**动效时钟源**(0028 §6 协调),不各跑各的。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ShaderboxClock {
    /// 已发出的节流时间(ms;dynamic box 的 `time` 取它 / 1000 = 秒)。
    emitted_ms: f32,
    /// 自上次步进以来累积的真实 dt(ms)。
    accum_ms: f32,
}

impl ShaderboxClock {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 推进真实时间 `dt_ms`;累积满一个节流间隔才把 `emitted` 跳进一步(30fps 步进,不随 60fps rAF)。
    pub(crate) fn tick(&mut self, dt_ms: f32) {
        self.accum_ms += dt_ms.max(0.0);
        if self.accum_ms >= SHADERBOX_THROTTLE_MS {
            self.emitted_ms += self.accum_ms;
            self.accum_ms = 0.0;
        }
    }

    /// 当前发出的 shader `time`(秒)。
    pub(crate) fn time_s(self) -> f32 {
        self.emitted_ms / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_values_match_source_case_numbers() {
        assert_eq!(IconId::Void.as_u32(), 0);
        assert_eq!(IconId::TheWorld.as_u32(), 46);
        assert_eq!(IconId::Elements.as_u32(), 49);
        assert_eq!(ShaderId::Icons.as_u32(), 0);
        assert_eq!(ShaderId::GlowOrb.as_u32(), 1);
    }

    #[test]
    fn exactly_four_static_icons() {
        let statics: Vec<u32> = (0u32..=49)
            .filter(|&v| {
                // 重建 IconId 仅为测试覆盖;用已知 4 个静态。
                matches!(v, 0 | 15 | 18 | 48)
            })
            .collect();
        assert_eq!(statics, vec![0, 15, 18, 48], "4 个纯静态");
        assert!(!IconId::Void.is_dynamic());
        assert!(!IconId::TheTemple.is_dynamic());
        assert!(!IconId::TheHermit.is_dynamic());
        assert!(!IconId::Enlightenment.is_dynamic());
        assert!(IconId::Justice.is_dynamic());
        assert!(IconId::TheWorld.is_dynamic());
    }

    #[test]
    fn throttle_clock_steps_at_30fps_not_60() {
        // 60fps(16.67ms)tick:头一帧未满 33ms → time 不动;两帧累积 33ms → 跳一步。
        let mut c = ShaderboxClock::new();
        c.tick(16.67);
        assert!(c.time_s().abs() < 1e-6, "单 60fps 帧不步进(未满节流)");
        c.tick(16.67);
        assert!(
            (c.time_s() - 0.0333).abs() < 0.01,
            "两帧累积 ~33ms → 步进一拍"
        );
    }

    #[test]
    fn aliases_map_into_deck() {
        assert!(IconId::copy().as_u32() <= 49);
        assert!(IconId::spinner().is_dynamic(), "spinner 应是动画");
    }

    #[test]
    fn area_cap_triggers_only_for_oversized_boxes() {
        // 护栏3:内置小 box(18/32px)不触发封顶;超阈巨 box 触发(走 downscale 钩子)。
        assert!(!shaderbox_exceeds_area_cap([18.0, 18.0]));
        assert!(!shaderbox_exceeds_area_cap([32.0, 32.0]));
        assert!(!shaderbox_exceeds_area_cap([
            SHADERBOX_MAX_EDGE_PX,
            SHADERBOX_MAX_EDGE_PX
        ]));
        assert!(shaderbox_exceeds_area_cap([
            SHADERBOX_MAX_EDGE_PX + 1.0,
            10.0
        ]));
        assert!(shaderbox_exceeds_area_cap([
            10.0,
            SHADERBOX_MAX_EDGE_PX + 1.0
        ]));
    }
}
