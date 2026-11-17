import { formatBytes } from "../lib/format";
import type { TaskRow } from "../lib/tasks";

export type { TaskRow };

const basename = (path: string) => path.split("/").pop() ?? path;

export function TaskRowView({ row, onOpen }: { row: TaskRow; onOpen?: (row: TaskRow) => void }) {
  const { report } = row;
  const name = basename(report?.input ?? row.path);

  if (row.error) {
    return (
      <li className="task failed" data-testid="task-row">
        <span className="task-name">{name}</span>
        <span className="badge">✗ 失败</span>
        <span className="muted small">{row.error}</span>
      </li>
    );
  }
  if (row.running || !report) {
    return (
      <li className="task running" data-testid="task-row">
        <span className="task-name">{name}</span>
        <span className="badge">处理中…</span>
      </li>
    );
  }

  const larger = report.status === "larger_output";
  const ratio =
    report.savedPct === null
      ? "—"
      : `${report.savedPct > 0 ? "+" : ""}${report.savedPct.toFixed(1)}%`;

  return (
    <li
      className={larger ? "task larger" : "task done"}
      data-testid="task-row"
      onClick={() => row.report && onOpen?.(row)}
      style={{ cursor: row.report ? "pointer" : "default" }}
    >
      <span className="task-name">{name}</span>
      <span className="sizes">
        {formatBytes(report.inputBytes)} →{" "}
        {report.outputBytes === null ? "—" : formatBytes(report.outputBytes)}
      </span>
      <span className="ratio">{larger ? "未落盘" : ratio}</span>
      {larger && (
        <span className="badge warn" title="输出不小于输入，已按负优化策略拦截">
          ⚠ 负优化
        </span>
      )}
      <span className="muted small">
        {report.engine} · {report.elapsedMs}ms
      </span>
    </li>
  );
}

export function TaskList({ rows, onOpen }: { rows: TaskRow[]; onOpen?: (row: TaskRow) => void }) {
  return (
    <section className="card" data-testid="task-list">
      <header className="card-head">
        <h2>任务</h2>
        <span className="muted small">{rows.length} 个</span>
      </header>
      {rows.length === 0 ? (
        <p className="muted">还没有任务。拖入图片开始。</p>
      ) : (
        <ul className="tasks">
          {rows.map((row) => (
            <TaskRowView key={`${row.index}-${row.path}`} row={row} onOpen={onOpen} />
          ))}
        </ul>
      )}
    </section>
  );
}
