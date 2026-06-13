// wasm-pack(--target web)产物的最小类型声明,供 tsc/编辑器在 pkg 生成前也能通过。
// 实际产物由 `npm run build:wasm` 生成到 web/pkg/。
declare module "*opencode_chat_wasm.js" {
  /** 加载并实例化 wasm 模块。 */
  export default function init(input?: unknown): Promise<unknown>;

  export interface ChatCanvasConfig {
    layout: (text: string, maxWidth: number) => Float32Array;
    rasterize: (cluster: string) => { data: Uint8Array; width: number; height: number };
    serverUrl?: string;
    sessionId?: string;
  }

  export class ChatCanvas {
    constructor(canvas: HTMLCanvasElement, config: ChatCanvasConfig);
    start(): void;
  }
}
