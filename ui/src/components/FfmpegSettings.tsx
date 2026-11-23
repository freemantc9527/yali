import { useEffect, useState } from "react";
import { Check, CircleAlert, RefreshCw, Terminal } from "lucide-react";
import { ffmpegStatus, isTauri, type FfmpegStatus } from "../lib/bridge";
import { Button } from "./ui/button";

export function FfmpegSettings() {
  const [status, setStatus] = useState<FfmpegStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = async () => {
    if (!isTauri()) return;
    setBusy(true);
    setError("");
    try {
      setStatus(await ffmpegStatus(true));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const state = !isTauri()
    ? "browser"
    : error
      ? "error"
      : busy || status === null
      ? "checking"
      : status.available && status.ssim
        ? "ready"
        : status.available
          ? "partial"
          : status.path
            ? "unusable"
            : "missing";

  return (
    <section className="settings-section" aria-labelledby="ffmpeg-title">
      <div className="settings-section-heading">
        <div className="settings-section-icon" aria-hidden="true">
          <Terminal />
        </div>
        <div>
          <h2 id="ffmpeg-title">质量检测</h2>
          <p className="hint">SSIM 使用系统提供的 FFmpeg，不随 App 打包。</p>
        </div>
      </div>
      <div className="dependency-row">
        <div className="dependency-state" data-state={state}>
          {state === "ready" ? <Check aria-hidden="true" /> : <CircleAlert aria-hidden="true" />}
          <div>
            <strong>FFmpeg</strong>
            <p className="hint" role="status">
              {state === "browser" && "浏览器演示不会检测主机依赖。"}
              {state === "checking" && "正在检查外部依赖…"}
              {state === "ready" && "已找到，SSIM 质量分数可用。"}
              {state === "partial" && "已找到，但没有可用的 ssim 滤镜。"}
              {state === "unusable" && "已找到，但无法运行，SSIM 质量分数会隐藏。"}
              {state === "missing" && "未找到，SSIM 质量分数会隐藏。"}
              {state === "error" && "检查失败，请重试。"}
            </p>
          </div>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={!isTauri() || busy}
          onClick={() => void refresh()}
        >
          <RefreshCw className={busy ? "dependency-spin" : undefined} aria-hidden="true" />
          {busy ? "检查中…" : "重新检查"}
        </Button>
      </div>
      {status?.path && (
        <div className="dependency-path">
          <span>检测到的路径</span>
          <code title={status.path}>{status.path}</code>
        </div>
      )}
      {!isTauri() && <p className="hint">请在桌面 App 中重新检查本机的 FFmpeg。</p>}
      {isTauri() && <p className="hint">未启用 SSIM 时，图片处理和视觉对比仍可使用。</p>}
      {error && <p className="field-error" role="alert">检查 FFmpeg 失败：{error}</p>}
    </section>
  );
}
