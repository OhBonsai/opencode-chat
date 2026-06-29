// wasm-pack(--target web)产物的最小类型声明,供 tsc/编辑器在 pkg 生成前也能通过。
// 实际产物由 `npm run build:wasm` 生成到 web/pkg/。
declare module "*infinite_chat_wasm.js" {
  /** 加载并实例化 wasm 模块。 */
  export default function init(input?: unknown): Promise<unknown>;

  export interface ChatCanvasConfig {
    layout: (
      runTexts: string[],
      runRoles: Uint32Array,
      maxWidth: number,
      // 最小 shim:`tables` 形状由 layout-bridge 的 TableRegionJS 定;facade 用 any 收(回调参数
      // 逆变,宿主传更具体类型才不报错。真实 wasm-bindgen 绑定为 `config: any`)。
      tables?: any,
    ) => Float32Array | { positions: Float32Array; tables: Float32Array };
    /// Plan 13 §4.2 measure 回调(可选):Taffy 叶子量尺寸 → [w, h];缺省时 wasm 退回 layout 派生。
    measure?: (runTexts: string[], runRoles: Uint32Array, availW: number) => Float32Array;
    rasterize: (cluster: string, style: number, kind: number) => Uint8Array;
    serverUrl?: string;
    sessionId?: string;
    /// Plan 5D 重放:预录事件 [{t, raw}](raw = opencode 信封 JSON 串)。
    replay?: { t: number; raw: string }[];
  }

  export interface ChatStats {
    fps: number;
    frameMsAvg: number;
    frameMsMax: number;
    dropped: number;
    glyphsVisible: number;
    glyphsTotal: number;
    blocksVisible: number;
    blocksTotal: number;
    shaderboxActive: number;
    shaderboxPixels: number;
    storeChars: number;
    retainedViews: number;
    retainedGlyphs: number;
    retainedNodes: number;
    phAdvance: number;
    phBfLayout: number;
    phBfGrid: number;
    phBfEmit: number;
    phBfTotal: number;
    phAdvIngest: number;
    phAdvRoles: number;
    phAdvReveal: number;
    phAdvEnsure: number;
    phAdvSchedule: number;
    tierHot: number;
    tierWarm: number;
    rebuilds: number;
    atlasUsed: number;
    atlasCap: number;
    atlasEvict: number;
    camZoom: number;
    paused: number;
    srcBitmap: number;
    srcTinySdf: number;
    srcMsdf: number;
    srcRgba: number;
  }

  export class ChatCanvas {
    constructor(canvas: HTMLCanvasElement, config: ChatCanvasConfig);
    start(): void;
    stats(): ChatStats;
    set_paused(paused: boolean): void;
    step(): void;
    set_debug_geometry(on: boolean): void;
    /** ShaderBox 画廊(Plan 16 `?gallery`):视口格栅逐格一个内置 shader,验全盘上屏。 */
    set_shaderbox_gallery(on: boolean): void;
    /** 到达整流基线吐字速率(Plan 18 `?bench`:调极大值即时载满长会话)。 */
    set_stream_rate(cps: number): void;
    /** Plan 19 P1 A/B(`?sizefold`):sizes 退回每帧 fold(P1 前),对照缓存 fps 收益。 */
    set_bench_fold_width(on: boolean): void;
    refresh_fonts(): void;
    set_glyph_mode(mode: number): void;
    /** 平移画布(屏幕/设备像素;Plan 6 web 层输入)。dy>0 看更新内容,dx>0 看右侧。 */
    pan_by(dx: number, dy: number): void;
    /** 围绕屏幕点(设备像素)缩放;factor>1 放大。 */
    zoom_at(factor: number, sx: number, sy: number): void;
    /** Plan 14 ③:领取待解码图片,返回 JSON `[{key,url}]`(并转 Loading)。 */
    take_pending_images(): string;
    /** Plan 14 ③:上传解码后的 RGBA 首帧(w×h×4 sRGB)→ GPU 纹理 + 推进该 key 到 Ready。 */
    upload_image_rgba(
      key: string,
      rgba: Uint8Array,
      w: number,
      h: number,
      animated: boolean,
    ): void;
    /** Plan 14 ③:图片解码/网络失败 → Failed(显 alt 兜底)。 */
    image_failed(key: string): void;
    /** Plan 14 ⑥:动图嵌入屏幕矩形 JSON `[{key,url,x,y,w,h}]`(设备像素)供 DOM overlay 定位。 */
    frame_embeds(): string;
    /** Plan 15 ④:屏幕点(设备像素)命中哪个代码块行窗 → key 串(空 = 未命中)。 */
    code_block_at_screen(sx: number, sy: number): string;
    /** Plan 15 ④:块内滚动(dx px 横、dyLines 行纵)。 */
    scroll_code_block(key: string, dx: number, dyLines: number): void;
    /** 设表格面板渲染样式(实时,无需重排/reload)。颜色分量 0..1。 */
    set_table_style(cfg: {
      lineColor?: [number, number, number, number];
      headerFill?: [number, number, number, number];
      aoColor?: [number, number, number];
      lineW?: number;
      ao?: number;
      aoWidth?: number;
      radius?: number;
    }): void;
    load_msdf(meta: {
      atlasW: number;
      atlasH: number;
      fontSize: number;
      ids: Uint32Array;
      cells: Float32Array;
      pixels: Uint8Array[];
    }): void;
    // ── Plan 21 P2/P3:选区 / 复制 / 文本层 / 查找(0030 DOM overlay 宿主接口)。
    /** Plan 21 P2:设选区,`flat` = 扁平三元组 `[block,start,end,...]`(end 不含);空 = 清。 */
    set_selection(flat: Uint32Array): void;
    /** Plan 21 P1:可见消息复制数据 JSON `[{id,turn,role,x,y,w,h,text}]`(设备像素)。 */
    visible_turns(): string;
    /** Plan 21 P2:可见文本 run JSON `[{block,char0,x,y,w,h,text}]`(虚拟透明文本层/原生选区)。 */
    visible_text_runs(): string;
    /** Plan 21 P3:跨全历史全文查找 → JSON `[{view,char}]`(含屏外/Warm 块)。 */
    find(query: string): string;
    /** Plan 21 P3:跳到某 view(Cmd+F 命中后),相机平移到块顶。 */
    scroll_to(view: number): void;
    // ── Plan 22:事件注入 + 会话 FSM + Dock 应答(0031 边界)。
    /** Plan 22 P0:注入一条原始 SSE 事件(TS→Rust 事件边界;也供 E2E 注入非文本 part)。 */
    push_event(raw: string): void;
    /** Plan 22 P2/P4:会话生命周期态标签(idle/streaming/blocked:permission/…)。 */
    session_status(): string;
    /** Plan 22 P2:用户发送 → FSM AwaitingAck(起 no-reply 计时)。 */
    note_send(): void;
    /** Plan 22 P4/F11:用户停止 → FSM Stopped + 冻结当前流式消息 + epoch+1。 */
    stop_turn(): void;
    /** Plan 22 P4:Dock 应答权限请求 → 解阻 FSM。 */
    reply_permission(): void;
    /** Plan 22 P4:Dock 应答反问 → 解阻 FSM。 */
    reply_question(): void;
    // ── 揭示调度 / 调试播放器(Plan 8C/12/19/0019)。
    /** Plan 12:数学每 em 的 world px = 正文字号(含 DPR)。 */
    set_math_em(px: number): void;
    /** Plan 8C/0019:揭示速率上限(glyph/秒);≤0 = 不限速。 */
    set_reveal_cps(cps: number): void;
    /** 0019:揭示放慢因子 `[0.01,1.0]`(越小越慢)。 */
    set_reveal_slow(slow: number): void;
    /** Plan 19 P2 虚拟化开关(false = 全程 Hot,P2 对照/兜底)。 */
    set_virtualize(on: boolean): void;
    /** Plan 8B/0019:表格揭示风格(0=逐字 / 1=行框 / 2=整表骨架先行)。 */
    set_table_reveal_style(style: number): void;
    /** 调试播放器:揭示动画跳到时间轴 `targetMs`(确定性重跑到该时刻)。 */
    seek_reveal(targetMs: number): void;
    /** 调试:清 spawn,按当前风格/速度从头重揭一遍(所见即所设)。 */
    restart_reveal(): void;
    /** 调试播放器:按显式 `dtMs` 推进一帧(不走墙钟;配 set_paused 由 JS 掌钟)。 */
    tick(dtMs: number): void;
  }
}
