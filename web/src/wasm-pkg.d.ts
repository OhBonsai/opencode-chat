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
      tables?: unknown,
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
  }
}
