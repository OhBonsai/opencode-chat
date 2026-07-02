// 供 vitest/tsc 导入 scripts/record-chat.mjs 的纯转换函数(Plan 25 PR-D)。
declare module "*record-chat.mjs" {
  export interface RawRecord {
    t: number;
    raw: string;
  }
  export interface FilteredItem {
    t: number;
    env: { type: string; properties?: Record<string, unknown> };
  }
  export function eventSession(env: unknown): string | null;
  export function filterRecords(records: RawRecord[], sessionId: string): FilteredItem[];
  export function userTextOf(items: FilteredItem[], messageId: string): string;
  export function toScript(
    items: FilteredItem[],
    opts?: { title?: string },
  ): { meta: { title: string; version: number }; track: unknown[] };
}
