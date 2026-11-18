/** 字节数 → 人类可读（供文件列表与压缩结果展示复用）。 */
export function formatBytes(bytes: number, digits = 1): string {
  if (!Number.isFinite(bytes) || bytes < 0) throw new Error(`invalid bytes: ${bytes}`);
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(digits)} ${units[unit]}`;
}

/** 压缩率（负数 = 变小），与 docs/BENCHMARKS.md 的百分比口径一致。 */
export function ratio(before: number, after: number): string {
  if (before <= 0) throw new Error("before must be > 0");
  const delta = ((after - before) / before) * 100;
  return `${delta > 0 ? "+" : ""}${delta.toFixed(1)}%`;
}

/** 所有工作台结果从字节计算，正数表示节省。 */
export function savings(before: number, after: number): string {
  if (!Number.isFinite(before) || !Number.isFinite(after) || before <= 0) return '不可计算';
  const pct = (before-after)/before*100;
  return pct < 0 ? `增加 ${Math.abs(pct).toFixed(1)}%` : `${pct.toFixed(1)}%`;
}
