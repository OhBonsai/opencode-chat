function i(t, r = 1) {
  const o = t.sessionID ?? "replay", s = t.messageID ?? "m", a = t.partID ?? "p1", e = r > 0 ? 1 / r : 1;
  return t.steps.map((n) => ({ t: Math.round(n.t * e), raw: JSON.stringify({ type: "message.part.delta", properties: { sessionID: o, messageID: s, partID: a, field: "text", delta: n.delta } }) }));
}
async function p(t, r = 1, o = "/infinite-chat/cases") {
  const s = await fetch(`${o}/${encodeURIComponent(t)}.json`);
  if (!s.ok) throw new Error(`replay case \u4E0D\u5B58\u5728: ${t} (${s.status})`);
  const a = await s.text();
  if (a.trimStart().startsWith("<")) throw new Error(`replay case \u4E0D\u5B58\u5728(\u8FD4\u56DE HTML,\u975E JSON): ${t}`);
  const e = JSON.parse(a);
  if (!Array.isArray(e.steps) || e.steps.length === 0) throw new Error(`replay case ${t} \u65E0 steps`);
  return i(e, r);
}
export {
  i as buildReplay,
  p as loadCase
};
