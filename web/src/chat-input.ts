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
export function mountChatInput(o: {
  serverUrl: string;
  sessionId?: string;
  model: ModelRef;
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
    "position:fixed;left:12px;bottom:60px;z-index:9001;max-width:60vw;color:#ff8a8a;" +
    "font:12px/1.4 system-ui,sans-serif;white-space:pre-wrap;display:none";

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

  let inFlight = false;
  const send = async () => {
    const text = ta.value.trim();
    if (!text || inFlight) return;
    inFlight = true;
    ta.disabled = true;
    btn.disabled = true;
    clearError();
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
      showError(`网络错误: ${String(e)}`);
    } finally {
      inFlight = false;
      ta.disabled = false;
      btn.disabled = false;
      ta.focus();
    }
  };

  ta.addEventListener("input", autosize);
  ta.addEventListener("keydown", (e) => {
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

  return () => {
    o.parent.removeChild(bar);
    o.parent.removeChild(err);
  };
}
