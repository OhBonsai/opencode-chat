# spec/ · 文档总导航

opencode-chat 的全部设计文档。新读者按下面顺序读。入口见仓库根 [AGENTS.md](../AGENTS.md)。

## 阅读顺序

1. **[decision/0000-overview.md](./decision/0000-overview.md)** — 为什么做、做什么(公众号文章版,先读这篇建立全局)
2. **[architecture.md](./architecture.md)** — 整体链路 + 13 模块设计(动态视图)
3. **[decision/0001…0008](./decision/)** — 逐项技术决策(ADR,append-only)
4. **[research/](./research/)** — 业界调研(现状 + 我们的差异化)
5. **[plan/](./plan/)** — 分阶段开发方案
6. **[dev-practices.md](./dev-practices.md)** — 铁律 + 技能 + 文档规范
7. **[testing-and-benchmark.md](./testing-and-benchmark.md)** — 测试/benchmark/可观测性

## 决策索引(L2 · append-only)

| 编号 | 主题 |
|---|---|
| [0000](./decision/0000-overview.md) | 项目总览(文章版) |
| [0001](./decision/0001-canvas-architecture.md) | 架构选型:wgpu/pretext,否决方案 |
| [0002](./decision/0002-event-driven-pipeline.md) | 事件管线 + 状态机 + 效果开关 |
| [0003](./decision/0003-fault-tolerance.md) | 容错 + 状态同步 + 降级 |
| [0004](./decision/0004-markdown-and-embeds.md) | markdown 语义层 + 图片/mermaid |
| [0005](./decision/0005-turn-aggregation-and-settlement.md) | Turn 聚合 + 收尾判定 |
| [0006](./decision/0006-inline-tags-and-extensibility.md) | 内嵌标签 + 插件扩展 |
| [0007](./decision/0007-rich-media-embeds.md) | 富媒体三层 + 像素对齐相机 |
| [0008](./decision/0008-multi-instance-sync.md) | 多标签/多实例同步 |

新决策从 `0009` 续编;append-only,覆盖旧决策时旧文件头加 `Superseded by: NNNN`。

## 目录结构

```
spec/
├── README.md                  # 本文(L0 导航)
├── decision/                  # L2 ADR(0000 总览 + 0001-0008 决策)
├── research/                  # L3 业界调研
├── knowledge/                 # 外部接口知识(opencode API,渐进式加载真相)
├── plan/                      # 分阶段开发方案(plan1…)
├── diagnose/                  # 诊断经验沉淀 + ISSUE_INDEX
├── architecture.md            # L4 模块动态视图
├── dev-practices.md           # L1 铁律 + 技能 + 文档规范
└── testing-and-benchmark.md   # 测试/benchmark/可观测性
```

## 文档形态判据(写文档前先定形态)

| 形态 | 选它 | 位置 |
|---|---|---|
| 已拍板决策,留证据 | decision(ADR) | `decision/NNNN-slug.md` |
| 多阶段/多 PR 方案 | plan | `plan/planN-slug.md` |
| 单 PR 方案 | spec | `plan/`(或随 PR) |
| bug 根因 + 踩坑 | diagnose | `diagnose/NNN-slug.md` + 更新 ISSUE_INDEX |
| 调研/参考 | research | `research/slug.md` |
