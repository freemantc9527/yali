import { useEffect, useState } from "react";
import type { CompressReport } from "../lib/bridge";
import { fileSrc, ssim } from "../lib/bridge";

interface Props {
  report: CompressReport;
  onClose: () => void;
}

export const clampPos = (value: number) => Math.min(100, Math.max(0, value));

/** 对比查看器（T-010）：before/after 滑杆 + SSIM 标注（口径与 benchkit 一致）。 */
export function Viewer({ report, onClose }: Props) {
  const [pos, setPos] = useState(50);
  const [score, setScore] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const before = fileSrc(report.input);
  const after = report.output ? fileSrc(report.output) : null;

  useEffect(() => {
    let cancelled = false;
    if (!report.output) return;
    ssim(report.input, report.output)
      .then((v) => !cancelled && setScore(v))
      .catch((e) => !cancelled && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, [report.input, report.output]);

  return (
    <div className="viewer-backdrop" onClick={onClose} data-testid="viewer">
      <div className="viewer" onClick={(e) => e.stopPropagation()}>
        <header className="card-head">
          <h2>对比</h2>
          <span className="muted small">
            {score !== null ? `SSIM ${score.toFixed(4)}` : error ? "SSIM 不可用" : "计算中…"}
          </span>
          <button type="button" onClick={onClose}>
            关闭
          </button>
        </header>

        {!before || !after ? (
          <p className="muted" style={{ padding: 24 }}>
            浏览器预览模式读不了本地文件路径——在 Tauri 应用里打开本页可看对比图。
          </p>
        ) : (
          <>
            <div className="compare">
              <img src={before} alt="原图" />
              <div className="after-clip" style={{ width: `${clampPos(pos)}%` }}>
                <img src={after} alt="压缩后" />
              </div>
              <div className="divider" style={{ left: `${clampPos(pos)}%` }} />
            </div>
            <input
              aria-label="compare-slider"
              type="range"
              min={0}
              max={100}
              value={pos}
              onChange={(e) => setPos(Number(e.target.value))}
              style={{ width: "100%", marginTop: 12 }}
            />
            <p className="muted small">
              左：原图 {report.inputBytes.toLocaleString()} B · 右：{report.outputBytes?.toLocaleString()} B ·{" "}
              {report.engine}
            </p>
          </>
        )}
      </div>
    </div>
  );
}
