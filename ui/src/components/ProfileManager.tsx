import { useState } from "react";
import type { Profile } from "../lib/bridge";
import { exportProfileJson, parseProfileJson, renameProfile, validateProfileName } from "../lib/profile-io";

interface Props {
  profile: Profile | null;
  onSave: (profile: Profile) => Promise<void>;
}

/** Profile 管理器（T-011）：另存 / 导出（剪贴板）/ 导入（粘贴 JSON）。 */
export function ProfileManager({ profile, onSave }: Props) {
  const [newName, setNewName] = useState("");
  const [importText, setImportText] = useState("");
  const [status, setStatus] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const saveAs = async () => {
    if (!profile) return;
    const error = validateProfileName(newName);
    if (error) {
      setStatus({ kind: "error", text: error });
      return;
    }
    setBusy(true);
    try {
      await onSave(renameProfile(profile, newName));
      setStatus({ kind: "ok", text: `已另存为 ${newName.trim()}` });
      setNewName("");
    } catch (e) {
      setStatus({ kind: "error", text: e instanceof Error ? e.message : String(e) });
    } finally {
      setBusy(false);
    }
  };

  const exportJson = async () => {
    if (!profile) return;
    const json = exportProfileJson(profile);
    try {
      await navigator.clipboard.writeText(json);
      setStatus({ kind: "ok", text: "已复制到剪贴板" });
    } catch {
      setStatus({ kind: "error", text: "剪贴板不可用（可在控制台查看 JSON）" });
      console.log(json);
    }
  };

  const importJson = async () => {
    try {
      const imported = parseProfileJson(importText);
      await onSave(imported); // 内核 validate 兜底，失败会抛错
      setStatus({ kind: "ok", text: `已导入 ${imported.name}` });
      setImportText("");
    } catch (e) {
      setStatus({ kind: "error", text: e instanceof Error ? e.message : String(e) });
    }
  };

  return (
    <section className="card" data-testid="profile-manager">
      <header className="card-head">
        <h2>Profile 管理</h2>
      </header>

      <div className="row" style={{ marginTop: 12 }}>
        <input
          aria-label="save-as-name"
          type="text"
          placeholder="新档名，如 my-web"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <button type="button" onClick={saveAs} disabled={!profile || busy || !newName.trim()}>
          另存为
        </button>
        <button type="button" onClick={exportJson} disabled={!profile}>
          导出到剪贴板
        </button>
      </div>

      <textarea
        aria-label="import-json"
        placeholder="粘贴 Profile JSON 后导入"
        rows={4}
        value={importText}
        onChange={(e) => setImportText(e.target.value)}
        style={{ width: "100%", marginTop: 12, fontFamily: "var(--font-mono, monospace)", fontSize: 12 }}
      />
      <div className="row" style={{ marginTop: 8 }}>
        <button type="button" onClick={importJson} disabled={!importText.trim()}>
          导入
        </button>
        {status && (
          <span className={status.kind === "ok" ? "status ok" : "status error"} data-testid="manager-status">
            {status.kind === "ok" ? "✓ " : "✗ "}
            {status.text}
          </span>
        )}
      </div>
    </section>
  );
}
