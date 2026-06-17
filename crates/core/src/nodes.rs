//! nodes(M6 / 0020)— 内容节点树地基:**append-only 扁平节点表**(嵌套区间 + parent 下标 +
//! 稳定 key)。统一三处零散身份(0016 glyph `NodeId`、0014 `TableRegion`、未来 0019 Selector /
//! 0022 embed / 0023 Taffy)为单一身份源。
//!
//! 取巧编码(0020 §3):append-only + 文档序渲染 ⇒ 任何节点的后代在扁平 glyph 数组里**连续**,
//! 故节点 = `[start,end)` 一个区间(nested-set / Euler-tour),免子指针;子树 = 区间包含、祖先 =
//! 包住你的更大区间。`range` 是**块内扁平 glyph(grapheme)下标**(与 0016 `glyph_idx` 同空间)。
//!
//! 纯 CPU、native 可测(CR1),不进扁平 glyph 流(旁挂,同 `TableRegion` 精神,AR10 不破)。
//!
//! ## 下游消费点(Plan 7D:只定接口面,不写消费者)
//! 本地基对四个后续 plan 暴露统一查询,各省一套身份轮子:
//! - **0019 reveal `Selector`** = [`NodeTree::nodes_of_kind`] + 区间(按 kind 选要骨架先行/限速的块)。
//! - **0022 embed** = [`NodeKind::Embed`] 占位 + `range`(图片/公式/mermaid 容器;本期 content 暂不产)。
//! - **0023 Taffy 盒子布局** = 节点 + `parent`(节点表直接喂 Taffy 树,无需另建)。
//! - **0016 节点级 morph** = 按节点 `range` 整体补间 + `key` 化身份(与渲染 buffer 顺序解耦,0020 §6)。
//!
//! 三处旧身份**折进本树为单一源**:0016 glyph `NodeId` = [`glyph_key`](= `Glyph` 叶,不入表);
//! 0014 `TableRegion` = 一组 `TableCell` 节点区间;0019 `Selector` = 按 kind/range 查询。

/// 内容节点类型(语义层级;leaf = `Glyph`)。`Embed` 为 0022 占位(本期 content 暂不产)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    Doc,
    Paragraph,
    Heading,
    List,
    ListItem,
    Quote,
    CodeBlock,
    Table,
    TableRow,
    TableCell,
    /// 显示公式块(`$$…$$`,0019/Plan 9 §9.0):有字(源)+ 框,骨架先行。
    MathDisplay,
    /// 分隔线(`---`,Plan 9 §9.0):**无字块**(只一个零墨 Rule 锚),走 NodeSpawn。
    ThematicBreak,
    /// HTML 块(Plan 9 §9.0):逐字(或留 raw 容器)。
    HtmlBlock,
    /// 同样式连续段(一个 `StyledSpan`)。
    Run,
    /// 单 grapheme 叶(`range` 长度 1)。
    Glyph,
    /// 嵌入块占位(图片/公式/mermaid,0022;本期不产,接口先留)。**无字块**,走 NodeSpawn。
    Embed,
}

/// 一个内容节点(0020 §4)。扁平表按 `range.start`(= 文档序 = 追加序)有序。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    /// 父节点下标(根 = 自身)。
    pub parent: u32,
    /// `[start,end)` into 块内扁平 glyph 数组(嵌套区间,0020 §3A)。
    pub range: (u32, u32),
    /// 跨帧稳定身份(0020 §3C)。v1 = `(block_seq<<32)|node_seq`(append-only ⇒ 稳定)。
    pub key: u64,
}

impl Node {
    /// 是否包含 glyph 下标 `g`(半开区间)。
    pub fn contains(&self, g: u32) -> bool {
        g >= self.range.0 && g < self.range.1
    }
    /// 区间长度(glyph 数)。
    pub fn len(&self) -> u32 {
        self.range.1.saturating_sub(self.range.0)
    }
    pub fn is_empty(&self) -> bool {
        self.range.1 <= self.range.0
    }
}

/// 一个块(part)的节点表(0020 §4):append-only 维护,冻结后只读。文档序、`nodes[0]` = 根 `Doc`。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeTree {
    nodes: Vec<Node>,
}

impl NodeTree {
    pub(crate) fn from_nodes(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 根节点(`Doc`),空树时 None。
    pub fn root(&self) -> Option<&Node> {
        self.nodes.first()
    }

    /// 按 kind 过滤(0020 §4 查询;Selector/embed 基础)。
    pub fn nodes_of_kind(&self, kind: NodeKind) -> impl Iterator<Item = (u32, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, n)| n.kind == kind)
            .map(|(i, n)| (i as u32, n))
    }

    /// 命中测试:含 glyph `g` 的**最内层**节点下标(区间最小者;命中 cell/list-item 用)。
    pub fn node_at(&self, g: u32) -> Option<u32> {
        let mut best: Option<(u32, u32)> = None; // (idx, len)
        for (i, n) in self.nodes.iter().enumerate() {
            if n.contains(g) {
                let l = n.len();
                if best.is_none_or(|(_, bl)| l < bl) {
                    best = Some((i as u32, l));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// 祖先链(从直接父到根),不含自身。
    pub fn ancestors(&self, idx: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut cur = idx;
        loop {
            let Some(n) = self.nodes.get(cur as usize) else {
                break;
            };
            if n.parent == cur {
                break; // 根:parent 指向自身
            }
            out.push(n.parent);
            cur = n.parent;
        }
        out
    }

    /// 直接子节点下标(parent == idx)。
    pub fn children(&self, idx: u32) -> impl Iterator<Item = u32> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(i, n)| *i as u32 != idx && n.parent == idx)
            .map(|(i, _)| i as u32)
    }

    /// 按稳定 key 取节点的 `range`(0019/0016 用)。
    pub fn range_of(&self, key: u64) -> Option<(u32, u32)> {
        self.nodes.iter().find(|n| n.key == key).map(|n| n.range)
    }
}

/// 一个块的构建规格(content 拍平时记录):kind + list 深度 + 块的 span 区间 + 表格 cell 区间。
pub(crate) struct BlockSpec {
    pub kind: NodeKind,
    pub depth: u32,
    /// 块覆盖的 span 下标 `[start,end)`。
    pub spans: (u32, u32),
    /// 表格:每行的 cell span 区间(= `TableRegion.rows`);非表格 None。
    pub table: Option<Vec<Vec<(u32, u32)>>>,
}

/// 节点稳定 key(0020 §3C v1):`(block_seq<<32) | node_seq`(append-only ⇒ 前缀稳定)。
fn key(block_seq: u32, node_seq: usize) -> u64 {
    (u64::from(block_seq) << 32) | node_seq as u64
}

/// 虚拟 glyph 叶身份(0020/0016):`(block_seq<<32) | glyph_idx`,= 0016 `NodeId` 打包。叶不入表
/// (退化节点),按需算 key。
pub fn glyph_key(block_seq: u32, glyph_idx: u32) -> u64 {
    (u64::from(block_seq) << 32) | u64::from(glyph_idx)
}

/// 从块规格 + span→glyph 前缀和构建节点树(0020 §3/§4)。`span_glyph[k]` = 第 k 个 span 的首
/// grapheme 下标,`span_glyph[nspans]` = 块总 glyph 数。容器 range 由子节点自动撑开(保证包含)。
pub(crate) fn build(block_seq: u32, span_glyph: &[u32], blocks: &[BlockSpec]) -> NodeTree {
    let total = span_glyph.last().copied().unwrap_or(0);
    let mut nodes: Vec<Node> = vec![Node {
        kind: NodeKind::Doc,
        parent: 0,
        range: (0, total),
        key: key(block_seq, 0),
    }];
    let gr = |s: u32, e: u32| (span_glyph[s as usize], span_glyph[e as usize]);

    // 撑开祖先 range 以包含 [gs,ge);push 一个节点,返回其下标。
    let push = |nodes: &mut Vec<Node>, kind: NodeKind, parent: u32, range: (u32, u32)| -> u32 {
        let idx = nodes.len() as u32;
        nodes.push(Node {
            kind,
            parent,
            range,
            key: key(block_seq, idx as usize),
        });
        let mut cur = parent;
        loop {
            let n = &mut nodes[cur as usize];
            n.range.0 = n.range.0.min(range.0);
            n.range.1 = n.range.1.max(range.1);
            if cur == 0 {
                break;
            }
            cur = n.parent;
        }
        idx
    };

    // span k 是否被某块覆盖(否则 = 块间分隔,挂 Doc)。
    let mut covered = vec![false; span_glyph.len().saturating_sub(1)];
    // (List 节点下标, 该层最近 ListItem 下标, depth):嵌套 List 的父 = 外层最近 ListItem(忠实树)。
    let mut list_stack: Vec<(u32, Option<u32>, u32)> = Vec::new();

    for b in blocks {
        for k in b.spans.0..b.spans.1 {
            if (k as usize) < covered.len() {
                covered[k as usize] = true;
            }
        }
        let brange = gr(b.spans.0, b.spans.1);

        // List 嵌套:为 ListItem 按 depth 开/复用 List 容器。
        let parent = if b.kind == NodeKind::ListItem {
            while list_stack.last().is_some_and(|&(_, _, d)| d > b.depth) {
                list_stack.pop();
            }
            let open_new = list_stack.last().is_none_or(|&(_, _, d)| d < b.depth);
            if open_new {
                // 嵌套 List 的父 = 外层 List 的**最近 ListItem**(忠实 ListItem→List;评审 #5)。
                let lp = list_stack.last().and_then(|&(_, it, _)| it).unwrap_or(0);
                let li = push(&mut nodes, NodeKind::List, lp, (brange.0, brange.0));
                list_stack.push((li, None, b.depth));
            }
            list_stack.last().map_or(0, |&(li, _, _)| li)
        } else {
            list_stack.clear();
            0
        };

        let container = push(&mut nodes, b.kind, parent, (brange.0, brange.0));
        if b.kind == NodeKind::ListItem {
            if let Some(top) = list_stack.last_mut() {
                top.1 = Some(container); // 记本层最近 ListItem,供更深 List 挂为父
            }
        }

        if let Some(rows) = &b.table {
            // Table → Row → Cell → Run(cell 内各 span)。
            for row in rows {
                if row.is_empty() {
                    continue;
                }
                let rs = gr(row[0].0, row[0].0);
                let tr = push(&mut nodes, NodeKind::TableRow, container, rs);
                for &(cs, ce) in row {
                    let cr = gr(cs, ce);
                    let cell = push(&mut nodes, NodeKind::TableCell, tr, (cr.0, cr.0));
                    for sk in cs..ce {
                        let g = gr(sk, sk + 1);
                        push(&mut nodes, NodeKind::Run, cell, g);
                    }
                }
            }
        } else {
            // 普通块:每个 span → Run 叶。
            for k in b.spans.0..b.spans.1 {
                let g = gr(k, k + 1);
                push(&mut nodes, NodeKind::Run, container, g);
            }
        }
    }

    // 块间分隔等未覆盖 span → 挂 Doc 的 Run。
    for k in 0..covered.len() as u32 {
        if !covered[k as usize] {
            let g = gr(k, k + 1);
            push(&mut nodes, NodeKind::Run, 0, g);
        }
    }

    NodeTree::from_nodes(nodes)
}

/// 节点树不变式校验(0020;测试用)。失败返回 `Err(原因)`。覆盖:子 `range ⊆` 父、兄弟按 start
/// 有序不重叠、根覆盖 `[0,count)`、parent 链合法无环。
#[cfg(test)]
pub(crate) fn check_invariants(tree: &NodeTree) -> Result<(), String> {
    let nodes = tree.nodes();
    if nodes.is_empty() {
        return Ok(());
    }
    // 根:idx 0,parent 指向自身,kind=Doc。
    let root = &nodes[0];
    if root.parent != 0 {
        return Err("根 parent 必须指向自身(0)".into());
    }
    if root.kind != NodeKind::Doc {
        return Err("根 kind 必须 Doc".into());
    }
    for (i, n) in nodes.iter().enumerate() {
        // 区间合法。
        if n.range.0 > n.range.1 {
            return Err(format!("节点 {i} 区间逆序 {:?}", n.range));
        }
        // parent 合法。
        if n.parent as usize >= nodes.len() {
            return Err(format!("节点 {i} parent 越界"));
        }
        if i != 0 {
            let p = &nodes[n.parent as usize];
            // 子 ⊆ 父。
            if n.range.0 < p.range.0 || n.range.1 > p.range.1 {
                return Err(format!(
                    "节点 {i} {:?} 不被父 {} {:?} 包含",
                    n.range, n.parent, p.range
                ));
            }
            // parent 链无环 + 终于根。
            let mut cur = n.parent;
            let mut steps = 0;
            while cur != 0 {
                let pn = &nodes[cur as usize];
                if pn.parent == cur && cur != 0 {
                    return Err(format!("节点 {i} 的祖先 {cur} 自环但非根"));
                }
                cur = pn.parent;
                steps += 1;
                if steps > nodes.len() {
                    return Err(format!("节点 {i} parent 链成环"));
                }
            }
        }
    }
    // 兄弟(同 parent)按 start 有序、不重叠。
    for pi in 0..nodes.len() {
        let mut sibs: Vec<&Node> = nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| *i != pi && n.parent as usize == pi)
            .map(|(_, n)| n)
            .collect();
        sibs.sort_by_key(|n| n.range.0);
        for w in sibs.windows(2) {
            if w[0].range.1 > w[1].range.0 {
                return Err(format!(
                    "父 {pi} 的兄弟区间重叠 {:?}/{:?}",
                    w[0].range, w[1].range
                ));
            }
        }
    }
    Ok(())
}
