import type { CompressReport, ProgressEvent } from "./bridge";

export type { ProgressEvent };

/** 任务行（UI 态；结果以 command 返回为准，事件只做即时反馈） */
export interface TaskRow {
  path: string;
  index: number;
  running: boolean;
  report?: CompressReport;
  error?: string;
}

/** 事件 → 行状态的纯 reducer（乱序/重复事件都要安全） */
export function applyProgress(rows: TaskRow[], event: ProgressEvent): TaskRow[] {
  const key = (row: TaskRow) => row.index === event.index && row.path === event.path;
  const exists = rows.some(key);
  const next: TaskRow[] = exists
    ? rows
    : [...rows, { path: event.path, index: event.index, running: true }];

  return next.map((row) => {
    if (!key(row)) return row;
    switch (event.status) {
      case "started":
        return { ...row, running: true, error: undefined };
      case "done":
        return { ...row, running: false, report: event.report ?? undefined, error: undefined };
      case "failed":
        return { ...row, running: false, error: event.error ?? "处理失败" };
      default:
        return row; // 未知状态不崩，忽略
    }
  });
}

/** 用 command 的最终结果对齐行（补上事件丢失的情况） */
export function reconcile(rows: TaskRow[], reports: CompressReport[]): TaskRow[] {
  const byInput = new Map(reports.map((r) => [r.input, r]));
  return rows.map((row) => {
    const report =
      byInput.get(row.path) ??
      [...byInput.entries()].find(([k]) => k.endsWith(row.path) || row.path.endsWith(k))?.[1];
    if (!report || row.report) return row;
    return { ...row, running: false, report };
  });
}
