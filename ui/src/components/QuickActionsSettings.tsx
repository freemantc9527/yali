import { useEffect, useState } from "react";
import { toast } from "sonner";
import { quickActionsStatus, quickActionsInstall, quickActionsUninstall, type QuickActionsStatus } from "../lib/bridge";
import { Button } from "./ui/button";
import { AlertDialog, AlertDialogContent, AlertDialogHeader, AlertDialogTitle, AlertDialogDescription, AlertDialogFooter, AlertDialogCancel, AlertDialogAction } from "./ui/alert-dialog";

export function QuickActionsSettings() {
  const [status, setStatus] = useState<QuickActionsStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [confirming, setConfirming] = useState(false);
  const refresh = async () => {
    setError("");
    try { setStatus(await quickActionsStatus()); }
    catch (e) { setError(String(e)); }
  };
  useEffect(() => { void refresh(); }, []);
  const change = async (remove: boolean) => {
    setBusy(true);
    setError("");
    try {
      setStatus(await (remove ? quickActionsUninstall() : quickActionsInstall()));
      toast.success(remove ? "右键操作已卸载" : "右键操作已安装");
    } catch (e) {
      setError(String(e));
      await refresh();
      // 刷新不能掩盖操作失败。
      setError(String(e));
    } finally { setBusy(false); }
  };
  return (
    <section aria-label="Finder 右键操作" className="settings-fields">
      <div>
        <strong>Finder 右键操作</strong>
        <p className="hint">选中图片后，使用“压一下”或“转 JPG”。</p>
      </div>
      <p className="hint" role="status">{status?.message ?? "正在检查安装状态…"}</p>
      {error && <p className="field-error" role="alert">{error}</p>}
      <div className="quick-action-buttons">
        <Button variant="outline" disabled={busy || !status?.canInstall} onClick={() => void change(false)}>
          {busy ? "处理中…" : status?.installed || status?.needsRepair ? "修复右键操作" : "安装右键操作"}
        </Button>
        <Button variant="ghost" disabled={busy || !(status?.installed || status?.needsRepair)} onClick={() => setConfirming(true)}>卸载</Button>
        <Button variant="ghost" disabled={busy} onClick={() => void refresh()}>刷新状态</Button>
      </div>
      <p className="hint">安装与卸载立即生效，无需保存设置。菜单未显示时，请在系统设置的“服务”或“扩展”中启用鸭梨。</p>
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>卸载鸭梨右键操作？</AlertDialogTitle>
            <AlertDialogDescription>移除“压一下”和“转 JPG”，保留图片、预设和 CLI 入口。</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction onClick={() => void change(true)}>确认卸载</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}
