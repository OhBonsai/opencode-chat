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

  bar.appendChild(ta);
  bar.appendChild(btn);
  o.parent.appendChild(bar);
  o.parent.appendChild(err);

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
    o.parent.removeChild(bar);
    o.parent.removeChild(err);
  };
}
