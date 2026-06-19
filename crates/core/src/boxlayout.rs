//! boxlayout(M13 / Plan 13 / 0023)— chat 级 **Taffy 盒子布局**:角色左右分栏 + 内容竖直堆叠。
//!
//! 把 `build_frame` 的「手搓 `top += height`」升级为 **Flexbox 盒树**(0023):每 view 一个叶子盒,
//! 按 **0005 角色**放进 UserBox(`align_self:FlexEnd` 右)/ AsstBox(`FlexStart` 左);assistant **一回合
//! 一个 AsstBox**(回合内多 part 作盒内块,守 §2.1)。输出 = 每 view 的**世界绝对 origin**(top-left),
//! build_frame 据此整体平移该 view 的字/框(view 内相对位不变 → 0016 morph 身份稳定)。
//!
//! 纯 Rust(taffy,无 wasm-bindgen/web-sys/wgpu),native 可测(CR1)。Tier A(本文件)只做 chat 级
//! 分栏 + per-part 叶子;Tier B/C(块内嵌套、measure 回调)= Plan 13 ③④。

use taffy::prelude::*;
use taffy::{NodeId, Style, TaffyTree};

/// 回合间距(px)。
const TURN_GAP: f32 = 20.0;
/// 同回合内 message/part 间距(px)。
const MSG_GAP: f32 = 8.0;
/// user 气泡最大宽(px;超长文本折行不铺满)。
const BUBBLE_MAX: f32 = 560.0;
/// assistant 内容最大宽(px)。
const CONTENT_MAX: f32 = 760.0;

/// 一个回合的角色分组(0005 投影,纯计算;Plan 13 §4.3)。`user` = 该回合 user part 的 view 下标
/// (可空);`assistant` = 该回合**连续** assistant part 的 view 下标(跨 message,守「一回合一盒」)。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TurnGroup {
    pub(crate) user: Option<usize>,
    pub(crate) assistant: Vec<usize>,
}

/// 列容器样式(flex column + 行距 + 可选左右对齐 + 可选最大宽)。
fn col(gap: f32, align: Option<AlignItems>, max_w: Option<f32>) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        align_self: align,
        max_size: Size {
            width: max_w.map_or_else(auto, length),
            height: auto(),
        },
        ..Default::default()
    }
}

/// 一个 view 盒在世界里的位置(Plan 13 §4):`origin` = 左上角 world 坐标;`width` = 盒宽(taffy
/// 算出,= 内容宽夹到 max)。build_frame 据此整体平移该 view 的字 + 据 `width` 摆全宽装饰/裁剪。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoxPos {
    pub(crate) origin: [f32; 2],
    pub(crate) width: f32,
}

/// **Tier A 盒树布局**(Plan 13 §4):turns(角色分组)+ 各 view 叶子尺寸 `sizes[vi]=(w,h)` →
/// ChatRoot ▸ Turn ▸ UserBox(右)/AsstBox(左)▸ per-part 叶子 → 算出**每 view 的 [`BoxPos`]**。
/// `viewport_w` = 文档宽(左右对齐锚它,§4.5)。任意失败/越界 → 该 view 退 `origin[0,0]`(防御)。
pub(crate) fn layout_chat(
    turns: &[TurnGroup],
    sizes: &[(f32, f32)],
    viewport_w: f32,
) -> Vec<BoxPos> {
    let n = sizes.len();
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let mut leaf_of: Vec<Option<NodeId>> = vec![None; n];
    let vw = viewport_w.max(1.0);

    // assistant 内容列宽(固定):内容块(分隔线/代码底/表格/Alert 底)铺满此列,而非各自收缩
    // 到本行文字宽——否则 `---` 这种零墨块会塌成 0 宽(§2.2 盒内异构块共用一列)。夹到视口。
    let content_col = CONTENT_MAX.min(vw);

    // 建一个 view 叶子。`fixed_w=Some(w)` → 显式宽(user 气泡,收缩夹 max);`None` → auto 宽
    // (assistant,靠父 `align_items:Stretch` 铺满内容列)。高恒为内容高。
    let mut make_leaf =
        |tree: &mut TaffyTree<()>, vi: usize, fixed_w: Option<f32>| -> Option<NodeId> {
            let (_, h) = *sizes.get(vi)?;
            let node = tree
                .new_leaf(Style {
                    size: Size {
                        width: fixed_w.map_or_else(auto, length),
                        height: length(h.max(0.0)),
                    },
                    ..Default::default()
                })
                .ok()?;
            if let Some(slot) = leaf_of.get_mut(vi) {
                *slot = Some(node);
            }
            Some(node)
        };

    let mut turn_nodes: Vec<NodeId> = Vec::new();
    for t in turns {
        let mut turn_children: Vec<NodeId> = Vec::new();
        // UserBox(右):一个 user part 叶子,收缩到文字宽(夹 BUBBLE_MAX),气泡右对齐。
        if let Some(ui) = t.user {
            let (w, _) = sizes.get(ui).copied().unwrap_or((0.0, 0.0));
            if let Some(l) = make_leaf(&mut tree, ui, Some(w.clamp(0.0, BUBBLE_MAX))) {
                if let Ok(b) = tree
                    .new_with_children(col(0.0, Some(AlignItems::FlexEnd), Some(BUBBLE_MAX)), &[l])
                {
                    turn_children.push(b);
                }
            }
        }
        // AsstBox(左):该回合所有 assistant part 叶子(一个盒,守「一回合一盒」),固定内容列宽,
        // 子块 `align_items:Stretch`(taffy 默认)铺满该列。
        let mut ach: Vec<NodeId> = Vec::new();
        for &ai in &t.assistant {
            if let Some(l) = make_leaf(&mut tree, ai, None) {
                ach.push(l);
            }
        }
        if !ach.is_empty() {
            let asst_style = Style {
                size: Size {
                    width: length(content_col),
                    height: auto(),
                },
                ..col(MSG_GAP, Some(AlignItems::FlexStart), Some(CONTENT_MAX))
            };
            if let Ok(b) = tree.new_with_children(asst_style, &ach) {
                turn_children.push(b);
            }
        }
        if let Ok(turn) = tree.new_with_children(col(MSG_GAP, None, None), &turn_children) {
            turn_nodes.push(turn);
        }
    }

    let Ok(root) = tree.new_with_children(
        Style {
            size: Size {
                width: length(vw),
                height: auto(),
            },
            ..col(TURN_GAP, None, None)
        },
        &turn_nodes,
    ) else {
        return vec![BoxPos::default(); n];
    };
    if tree
        .compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(vw),
                height: AvailableSpace::MaxContent,
            },
        )
        .is_err()
    {
        return vec![BoxPos::default(); n];
    }

    // DFS 累加绝对 origin(taffy `Layout.location` 相对父);叶子节点 → 对应 view 的 origin + width。
    let mut out = vec![BoxPos::default(); n];
    let mut stack: Vec<(NodeId, [f32; 2])> = vec![(root, [0.0, 0.0])];
    while let Some((node, base)) = stack.pop() {
        let (lx, ly, w) = tree
            .layout(node)
            .map(|l| (l.location.x, l.location.y, l.size.width))
            .unwrap_or((0.0, 0.0, 0.0));
        let abs = [base[0] + lx, base[1] + ly];
        for (vi, slot) in leaf_of.iter().enumerate() {
            if *slot == Some(node) {
                out[vi] = BoxPos {
                    origin: abs,
                    width: w,
                };
            }
        }
        if let Ok(children) = tree.children(node) {
            for c in children {
                stack.push((c, abs));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_right_assistant_left() {
        // 一回合:user(右)+ assistant(左)。viewport 1000;user/asst 各宽 200/300。
        let turns = vec![TurnGroup {
            user: Some(0),
            assistant: vec![1],
        }];
        let sizes = vec![(200.0, 30.0), (300.0, 50.0)];
        let o = layout_chat(&turns, &sizes, 1000.0);
        // user 右对齐:x ≈ viewport - 宽 = 1000 - 200 = 800。assistant 左:x ≈ 0。
        assert!(o[0].origin[0] > 700.0, "user 应右对齐: {}", o[0].origin[0]);
        assert!(o[1].origin[0] < 50.0, "assistant 应左: {}", o[1].origin[0]);
        // assistant 在 user 下方(回合内 user 先 / asst 后,竖直堆叠)。
        assert!(
            o[1].origin[1] > o[0].origin[1],
            "asst 在 user 下: {} vs {}",
            o[1].origin[1],
            o[0].origin[1]
        );
    }

    #[test]
    fn user_bubble_clamped_to_max_width() {
        // 超长 user 文本(宽 2000)→ 盒宽夹到 BUBBLE_MAX(560),右对齐 x ≈ viewport - 560。
        let turns = vec![TurnGroup {
            user: Some(0),
            assistant: vec![],
        }];
        let sizes = vec![(2000.0, 40.0)];
        let o = layout_chat(&turns, &sizes, 1000.0);
        assert!(
            (o[0].origin[0] - (1000.0 - BUBBLE_MAX)).abs() < 2.0,
            "user 盒宽应夹到 BUBBLE_MAX,右对齐 x≈{}: 实 {}",
            1000.0 - BUBBLE_MAX,
            o[0].origin[0]
        );
    }

    #[test]
    fn turns_stack_vertically_with_gap() {
        // 两回合竖直堆叠:回合2 在回合1 下方,间距 ≥ TURN_GAP。
        let turns = vec![
            TurnGroup {
                user: Some(0),
                assistant: vec![1],
            },
            TurnGroup {
                user: Some(2),
                assistant: vec![3],
            },
        ];
        let sizes = vec![(200.0, 30.0), (300.0, 50.0), (200.0, 30.0), (300.0, 50.0)];
        let o = layout_chat(&turns, &sizes, 1000.0);
        // 回合2 的 user(view 2)在回合1 的 assistant(view 1)下方。
        assert!(
            o[2].origin[1] > o[1].origin[1],
            "回合2 在回合1 下: {} vs {}",
            o[2].origin[1],
            o[1].origin[1]
        );
    }

    #[test]
    fn assistant_parts_share_one_box_stacked() {
        // 一回合多 assistant part(一个 AsstBox):竖直堆叠、都左对齐。
        let turns = vec![TurnGroup {
            user: None,
            assistant: vec![0, 1],
        }];
        let sizes = vec![(300.0, 30.0), (300.0, 40.0)];
        let o = layout_chat(&turns, &sizes, 1000.0);
        assert!(o[0].origin[0] < 50.0 && o[1].origin[0] < 50.0, "都左对齐");
        assert!(
            o[1].origin[1] > o[0].origin[1],
            "part2 在 part1 下(同盒堆叠)"
        );
    }
}
