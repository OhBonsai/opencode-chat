// chat-input.ts — Plan 13 §5:纯前端调试输入框(画布下方),直接 `POST /session/{id}/message`
// 和本地 opencode serve 实时对话。**零 wasm/core 改动**:只负责"把话发出去",assistant 回包
// (SSE delta/updated)由现有 Rust transport(M1)接收并渲染。
//
// 用法:main.ts 在 serverUrl 就绪时 `mountChatInput({...})`;Enter 发送 / Shift+Enter 换行。

/** opencode 模型标识:`provider/model` 串拆成 {providerID, modelID}(见 scripts/chat.mjs)。 */
export interface ModelRef {
  providerID: string;
  modelID: string;
}

/** `"provider/model"` → {providerID, modelID}。无斜杠则整串作 modelID(provider 空)。 */
export function parseModel(s: string): ModelRef {
  const slash = s.indexOf("/");
  return slash < 0
    ? { providerID: "", modelID: s }
    : { providerID: s.slice(0, slash), modelID: s.slice(slash + 1) };
}

/** 无 `?session=` 时建一个会话:`POST /session` → `{ id }`(prefix 默认无,见 knowledge §2)。 */
export async function ensureSession(serverUrl: string, sessionId?: string): Promise<string> {
  if (sessionId) return sessionId;
  const r = await fetch(`${serverUrl}/session`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: "{}",
  });
  if (!r.ok) throw new Error(`建会话失败 ${r.status} ${await r.text()}`);
  const j = (await r.json()) as { id?: string };
  if (!j.id) throw new Error("建会话响应缺 id");
  return j.id;
}

/** 共享 DOM 构造(真组件真样式):bar + textarea + 发送按钮。真发送(`mountChatInput`)与
 * 剧本模式(`mountScriptedInput`,Plan 25)共用同一份样式/结构 —— 拒绝仿品。 */
function buildInputDom(): {
  bar: HTMLDivElement;
  ta: HTMLTextAreaElement;
  btn: HTMLButtonElement;
} {
  const bar = document.createElement("div");
  bar.style.cssText =
    "position:fixed;left:0;right:0;bottom:0;z-index:9000;display:flex;gap:8px;align-items:flex-end;" +
    "padding:10px 12px;background:rgba(20,22,28,0.82);backdrop-filter:blur(6px);" +
    "border-top:1px solid rgba(255,255,255,0.08)";

  const ta = document.createElement("textarea");
  ta.placeholder = "输入消息,Enter 发送 / Shift+Enter 换行…";
  ta.rows = 1;
  ta.style.cssText =
    "flex:1;resize:none;max-height:30vh;min-height:22px;padding:8px 10px;border-radius:8px;" +
    "border:1px solid rgba(255,255,255,0.12);background:rgba(0,0,0,0.35);color:#e8e8ea;" +
    "font:14px/1.4 system-ui,sans-serif;outline:none";

  const btn = document.createElement("button");
  btn.textContent = "发送";
  btn.style.cssText =
    "padding:8px 16px;border-radius:8px;border:none;cursor:pointer;color:#fff;" +
    "background:#3b6fe0;font:600 14px system-ui,sans-serif";

  bar.appendChild(ta);
  bar.appendChild(btn);
  return { bar, ta, btn };
}

/** 把输入框实测高写入 `--input-h`(画布据此扣高)+ 高度真变时派发 resize(wasm 重配 surface)。
 * 返回清理函数。真发送与剧本模式共用。 */
function attachInsetSync(bar: HTMLDivElement): () => void {
  let lastBarH = -1;
  const syncInset = () => {
    const h = bar.offsetHeight;
    document.documentElement.style.setProperty("--input-h", `${h}px`);
    if (h === lastBarH) return;
    lastBarH = h;
    window.dispatchEvent(new Event("resize"));
  };
  const ro = new ResizeObserver(syncInset);
  ro.observe(bar);
  syncInset();
  // wasm 的 resize 监听在 GPU init 末尾才挂,可能晚于此刻 → 多补几拍确保后备缓冲重配。
  for (const d of [300, 1200]) setTimeout(() => window.dispatchEvent(new Event("resize")), d);
  return () => {
    ro.disconnect();
    document.documentElement.style.setProperty("--input-h", "0px");
    window.dispatchEvent(new Event("resize"));
  };
}

/** 挂载输入框到 `parent`。返回卸载函数(移除 DOM)。`sessionId` 可空 → 首次发送时惰性建会话
 * (`ensureSession`),故输入框**立即可见**,不依赖服务端/会话先就绪(无服务端则发送时友好报错)。 */
/** 跨重载暂存待发消息的 key(画布未连服务端时,重连后自动续发)。 */
const PENDING_KEY = "ic_pending_send";

export function mountChatInput(o: {
  serverUrl: string;
  sessionId?: string;
  model: ModelRef;
  /** 画布是否已连同一服务端的 SSE(= 页面带 `?server=`)。false → 发送前先重连(否则回包无处渲染)。 */
  canvasLive: boolean;
  parent: HTMLElement;
}): () => void {
  let session = o.sessionId; // 惰性:首次发送时建
  const { bar, ta, btn } = buildInputDom();

  const err = document.createElement("div");
  err.style.cssText =
    "position:fixed;left:12px;right:12px;bottom:64px;z-index:9001;color:#ffb4b4;" +
    "background:rgba(60,16,16,0.92);border:1px solid #7a2a2a;border-radius:8px;padding:8px 12px;" +
    "font:13px/1.45 system-ui,sans-serif;white-space:pre-wrap;display:none";

  const showError = (msg: string) => {
    err.textContent = msg;
    err.style.display = "block";
  };
  const clearError = () => {
    err.style.display = "none";
  };

  // textarea 高度随内容自适应(单行起,最多 30vh)。
  const autosize = () => {
    ta.style.height = "auto";
    ta.style.height = `${ta.scrollHeight}px`;
  };

  const fetchHint = (e: unknown) =>
    e instanceof TypeError
      ? `连不上 opencode (${o.serverUrl})。先起服务端:node scripts/serve.mjs,或 ?server= 指定地址。`
      : String(e);

  let inFlight = false;
  const send = async () => {
    const text = ta.value.trim();
    if (!text || inFlight) return;
    inFlight = true;
    ta.disabled = true;
    btn.disabled = true;
    const btnLabel = btn.textContent;
    clearError();

    // 画布未连服务端(页面无 ?server=)→ 回包无处渲染。先建会话、暂存本条,重载到 ?server=&session=
    // 让画布连上同一 SSE,重连后自动续发(见下方 PENDING_KEY 回放)。
    if (!o.canvasLive) {
      btn.textContent = "连接中…";
      console.info("[chat-input] 画布未连服务端,重连后续发", { serverUrl: o.serverUrl });
      try {
        const sid = session ?? (await ensureSession(o.serverUrl));
        sessionStorage.setItem(PENDING_KEY, text);
        const u = new URL(location.href);
        u.searchParams.set("server", o.serverUrl);
        u.searchParams.set("session", sid);
        location.assign(u.toString()); // 重载 → 画布连 SSE,本条自动续发
      } catch (e) {
        showError(`无法连接:${fetchHint(e)}`);
        console.error("[chat-input] 连接失败", e);
        inFlight = false;
        ta.disabled = false;
        btn.disabled = false;
        btn.textContent = btnLabel;
      }
      return;
    }

    btn.textContent = "发送中…";
    console.info("[chat-input] 发送", { serverUrl: o.serverUrl, session, text });
    try {
      if (!session) session = await ensureSession(o.serverUrl); // 惰性建会话(首发)
      const r = await fetch(`${o.serverUrl}/session/${session}/message`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ parts: [{ type: "text", text }], model: o.model }),
      });
      if (!r.ok) {
        showError(`发送失败 ${r.status}: ${(await r.text()).slice(0, 300)}`);
      } else {
        ta.value = "";
        autosize();
      }
    } catch (e) {
      showError(`发送失败:${fetchHint(e)}`);
      console.error("[chat-input] 发送失败", e);
    } finally {
      inFlight = false;
      ta.disabled = false;
      btn.disabled = false;
      btn.textContent = btnLabel;
      ta.focus();
    }
  };

  ta.addEventListener("input", autosize);
  ta.addEventListener("keydown", (e) => {
    // IME 组字中(中文/日文…):Enter 仅确认候选,不发送。keyCode 229 / isComposing 都判,稳健。
    if (e.isComposing || e.keyCode === 229) return;
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  });
  btn.addEventListener("click", () => void send());

  o.parent.appendChild(bar);
  o.parent.appendChild(err);

  const detachInset = attachInsetSync(bar);

  // 重连续发:上一步因画布未连而重载到 ?server= 后,画布已连 SSE → 取出暂存消息自动发出。
  if (o.canvasLive) {
    const pending = sessionStorage.getItem(PENDING_KEY);
    if (pending) {
      sessionStorage.removeItem(PENDING_KEY);
      ta.value = pending;
      autosize();
      setTimeout(() => void send(), 150); // 让 SSE/快照先就绪一拍
    }
  }

  return () => {
    detachInset();
    o.parent.removeChild(bar);
    o.parent.removeChild(err);
  };
}

// ───────────────────────── 剧本模式(Plan 25 PR-B) ─────────────────────────

/** 剧本模式输入框句柄:player 的 driver 用它演"用户在打字并发送"。 */
export interface ScriptedInput {
  /** 逐字打出 `text`(`cps` 字/秒);`instant` = 秒填(seek 快放)。返回打完的 Promise。 */
  typeText(text: string, cps: number, instant?: boolean): Promise<void>;
  /** 按发送态样式闪一下按钮 → 清空输入框(发送本身不产画布内容;气泡走剧本事件)。 */
  flashSend(): void;
  unmount(): void;
}

/** 挂载**剧本模式**输入框(Plan 25):同一份真组件真样式,但**禁网络**(不 POST、不建会话)、
 * 观看者不可编辑(readOnly)。真发送路径(`mountChatInput`)零改动、零影响。 */
export function mountScriptedInput(parent: HTMLElement): ScriptedInput {
  const { bar, ta, btn } = buildInputDom();
  ta.readOnly = true; // 纯自动播片:观看者不打字
  ta.placeholder = "";
  parent.appendChild(bar);
  const detachInset = attachInsetSync(bar);

  const autosize = () => {
    ta.style.height = "auto";
    ta.style.height = `${ta.scrollHeight}px`;
  };

  let typeTimer = 0;
  const typeText = (text: string, cps: number, instant = false): Promise<void> => {
    window.clearInterval(typeTimer);
    if (instant || cps <= 0) {
      ta.value = text;
      autosize();
      return Promise.resolve();
    }
    // 逐字(码点)填入;光标 = textarea 原生 caret(focus 即见)。
    const chars = [...text];
    ta.value = "";
    ta.focus();
    let i = 0;
    return new Promise((resolve) => {
      typeTimer = window.setInterval(() => {
        i += 1;
        ta.value = chars.slice(0, i).join("");
        autosize();
        if (i >= chars.length) {
          window.clearInterval(typeTimer);
          resolve();
        }
      }, 1000 / cps);
    });
  };

  const flashSend = () => {
    // 按下态:短暂提亮 + 微缩,再复原并清空 —— 与真发送同一颗按钮同一套样式。
    const prev = btn.style.background;
    btn.style.background = "#5a8bff";
    btn.style.transform = "scale(0.94)";
    window.setTimeout(() => {
      btn.style.background = prev;
      btn.style.transform = "";
      ta.value = "";
      autosize();
    }, 160);
  };

  return {
    typeText,
    flashSend,
    unmount: () => {
      window.clearInterval(typeTimer);
      detachInset();
      parent.removeChild(bar);
    },
  };
}
