// style-config(Plan 6)— web 层样式配置(Figma 式属性面板的数据源)。
//
// 渲染样式里"非内容、可调观感"的部分放这里(表格垂直对齐…),由排版(layout-bridge)读取、
// 样式面板(style-panel)写入。改值后宿主调 `chat.refresh_fonts()` 作废排版缓存 → 下一帧重排。
// 结构按元素分组(table / list / div …),便于面板分节渲染,也便于后续扩展。

export type VAlign = "top" | "center" | "bottom";
/// 水平对齐;`auto` = 跟随 markdown 列对齐(`:--` / `:-:` / `--:`),其余为全表覆盖。
export type HAlign = "auto" | "left" | "center" | "right";
export type RGBA = [number, number, number, number]; // 分量 0..1
export type RGB = [number, number, number];

/// 表格面板渲染样式(0..1 颜色;字段名与 wasm `set_table_style` 一致 → 可直接透传)。
export interface TableRender {
  lineColor: RGBA; // 网格线 / 外框
  headerFill: RGBA; // 表头底
  aoColor: RGB; // AO 辉光色(暗色主题取白)
  lineW: number; // 线宽 px
  ao: number; // AO 强度 0..~0.5
  aoWidth: number; // AO 向内淡出 px
  radius: number; // 圆角 px
}

/// 主题 token **局部覆盖**(Plan 26①):键 = core `Theme` 字段名(snake_case),值 = RGBA/RGB
/// 数组。空对象 = 默认主题。整包 JSON 化后经 wasm `set_theme` 灌入(缺字段 core 用默认补)。
export type ThemeOverrides = Record<string, number[]>;

/// 面板可调的主题 token(名, 标签, 默认值)。默认值镜像 core `Theme::default()`(跨语言令牌表,
/// 改 core 默认需同步;alert 三元组走 JSON/URL,不进面板)。
export const THEME_TOKENS: [string, string, RGBA][] = [
  ["code_bg", "code bg", [0.1, 0.11, 0.16, 0.75]],
  ["code_chip", "code chip", [0.18, 0.19, 0.26, 0.7]],
  ["code_border", "code border", [0.32, 0.36, 0.46, 0.85]],
  ["quote_bar", "quote bar", [0.42, 0.46, 0.56, 0.9]],
  ["head_rule", "head rule", [0.24, 0.27, 0.33, 0.9]],
  ["hr_rule", "hr rule", [0.82, 0.86, 0.94, 1.0]],
  ["selection", "selection", [0.26, 0.45, 0.92, 0.4]],
  ["card_bg", "card bg", [0.14, 0.16, 0.21, 0.55]],
  ["card_border", "card border", [0.3, 0.34, 0.44, 0.7]],
  ["diff_add_bg", "diff add", [0.22, 0.45, 0.27, 0.35]],
  ["diff_del_bg", "diff del", [0.5, 0.22, 0.24, 0.35]],
];

export interface StyleConfig {
  table: {
    /// 单元格文字在行内的垂直对齐(多行/不等高行显著;单行行内即整体上/中/下)。**布局**(走重排)。
    vAlign: VAlign;
    /// 水平对齐覆盖(auto = 用列对齐)。**布局**(走重排)。
    hAlign: HAlign;
  };
  /// 表格面板**渲染**样式(颜色/AO/线宽/圆角)。走 wasm `set_table_style`,实时、不重排。
  tableRender: TableRender;
  /// 主题 token 覆盖(Plan 26①)。走 wasm `set_theme`,实时、不重排。
  theme: ThemeOverrides;
  // 占位:后续元素分组(list 标记/缩进、div 容器内边距…)接到这里,面板自动出节。
}

const DEFAULT: StyleConfig = {
  table: { vAlign: "center", hAlign: "auto" },
  // 默认 = core `TableStyle::default()`(theme table_rule / table_header_bg)。
  tableRender: {
    lineColor: [0.26, 0.29, 0.36, 0.9],
    headerFill: [0.16, 0.18, 0.24, 0.6],
    aoColor: [1, 1, 1],
    lineW: 1,
    ao: 0.12,
    aoWidth: 10,
    radius: 4,
  },
  theme: {},
};

const KEY = "infinite-chat.styleConfig";

function clone(c: StyleConfig): StyleConfig {
  return JSON.parse(JSON.stringify(c)) as StyleConfig;
}

let config: StyleConfig = load();

function load(): StyleConfig {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return clone(DEFAULT);
    const c = JSON.parse(raw) as Partial<StyleConfig>;
    return {
      table: { ...DEFAULT.table, ...(c.table ?? {}) },
      tableRender: { ...DEFAULT.tableRender, ...(c.tableRender ?? {}) },
      theme: { ...(c.theme ?? {}) },
    };
  } catch {
    return clone(DEFAULT);
  }
}

/// 当前配置(排版热路径直接读;返回引用,勿原地改——改用 `setStyleConfig`)。
export function getStyleConfig(): StyleConfig {
  return config;
}

/// 整体替换并持久化(面板写入用)。
export function setStyleConfig(next: StyleConfig): void {
  config = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(config));
  } catch {
    /* localStorage 不可用 → 忽略,仅本会话生效 */
  }
}
