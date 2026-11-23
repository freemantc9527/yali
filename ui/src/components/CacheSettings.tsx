import { useEffect, useState } from "react";
import { cacheStatus, cacheClear, isTauri, type CacheStatus } from "../lib/bridge";
import { formatBytes } from "../lib/format";
import { Button } from "./ui/button";
import { AlertDialog, AlertDialogContent, AlertDialogHeader, AlertDialogTitle, AlertDialogDescription, AlertDialogFooter, AlertDialogCancel, AlertDialogAction } from "./ui/alert-dialog";

export function CacheSettings() {
  const [status, setStatus] = useState<CacheStatus>();
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    let alive = true;
    const refresh = () => void cacheStatus().then((value) => {
      if (alive) setStatus(value);
    }).catch((e) => { if (alive) setError(String(e)); });
    refresh();
    const interval = window.setInterval(refresh, 5000);
    return () => { alive = false; window.clearInterval(interval); };
  }, []);
  const clear = async () => {
    setBusy(true);
    setError("");
    try {
      const result = await cacheClear();
      setStatus(result);
      setMessage(result.bytes > 0 ? "已清理闲置缓存，使用中的文件保留，释放后按自动规则回收。" : "缓存已清理。");
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  return <section className="settings-section" aria-label="图片缓存">
    <div className="setting-row">
      <div>
        <strong>图片缓存</strong>
        <p className="hint" role="status">{status ? `已用 ${formatBytes(status.bytes)} · 上限 256 MiB` : "正在统计…"}</p>
        {status && status.protectedBytes > 0 && <p className="hint">使用中 {formatBytes(status.protectedBytes)}</p>}
      </div>
      <Button variant="outline" disabled={!isTauri() || busy || !status?.bytes} onClick={() => setConfirming(true)}>{busy ? "清理中…" : "清理缓存"}</Button>
    </div>
    <p className="hint">闲置超过 24 小时自动清理，超出容量时优先清理较久未使用的文件。使用中的文件受保护，释放后继续回收。</p>
    {message && <p className="hint" role="status">{message}</p>}
    {error && <p className="field-error" role="alert">{error}</p>}
    <AlertDialog open={confirming} onOpenChange={setConfirming}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>清理图片缓存？</AlertDialogTitle>
          <AlertDialogDescription>删除闲置缩略图、显示副本和临时文件，需要时会重新生成。保留原图、处理结果、预设和撤销记录；使用中的文件暂不清理。</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction onClick={() => void clear()}>确认清理</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </section>;
}
