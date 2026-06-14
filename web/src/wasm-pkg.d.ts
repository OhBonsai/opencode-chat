// wasm-pack(--target web)产物的最小类型声明,供 tsc/编辑器在 pkg 生成前也能通过。
// 实际产物由 `npm run build:wasm` 生成到 web/pkg/。
declare module "*infinite_chat_wasm.js" {
  /** 加载并实例化 wasm 模块。 */
  export default function init(input?: unknown): Promise<unknown>;

  export interface ChatCanvasConfig {
    layout: (runTexts: string[], runRoles: Uint32Array, maxWidth: number) => Float32Array;
    rasterize: (cluster: string, style: number, kind: number) => Uint8Array;
    serverUrl?: string;
    sessionId?: string;
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
    refresh_fonts(): void;
    set_glyph_mode(mode: number): void;
  }
}
