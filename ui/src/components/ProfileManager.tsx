import { useState } from "react";
import { ArrowLeft, Copy, Download, Plus } from "lucide-react";
import { toast } from "sonner";
import { profilesShow, profilesValidate, type Profile } from "../lib/bridge";
import {
  parseProfileJson,
  exportProfileJson,
  renameProfile,
} from "../lib/profile-io";
import { profileLabel } from "../lib/profile-labels";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Textarea } from "./ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
} from "./ui/alert-dialog";

export type PresetMode = "save" | "import" | "export" | null;
export function ProfileManager({
  names,
  onBack,
  onMode,
  onExport,
}: {
  names: string[];
  onBack: () => void;
  onMode: (mode: PresetMode) => void;
  onExport: (p: Profile) => void;
}) {
  return (
    <section className="aux-view">
      <header className="aux-toolbar">
        <Button variant="ghost" onClick={onBack}>
          <ArrowLeft />
          返回工作台
        </Button>
        <h1>管理预设</h1>
      </header>
      <div className="aux-scroll">
        <div className="settings-container">
          <p className="hint">
            已保存预设用于后续处理；本批调整需明确保存才会写入。
          </p>
          <div className="flex gap-2 flex-wrap">
            <Button variant="outline" onClick={() => onMode("save")}>
              <Plus />
              另存当前方案
            </Button>
            <Button variant="outline" onClick={() => onMode("import")}>
              <Download />
              粘贴导入 JSON
            </Button>
          </div>
          <ul className="preset-list">
            {names.map((name) => (
              <li key={name}>
                <div>
                  <strong>{profileLabel(name)}</strong>
                  <p className="hint">
                    {name}
                    {["default", "jpg"].includes(name)
                      ? " · Finder 快速操作使用"
                      : ""}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    void profilesShow(name)
                      .then(onExport)
                      .catch((e) => toast.error(String(e)))
                  }
                >
                  <Copy />
                  导出 JSON
                </Button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}
export function PresetDialog({
  mode,
  profile,
  names,
  onClose,
  onSave,
}: {
  mode: PresetMode;
  profile: Profile | null;
  names: string[];
  onClose: () => void;
  onSave: (p: Profile) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [raw, setRaw] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [replacement, setReplacement] = useState<Profile | null>(null);
  const save = async (p: Profile) => {
    setSaving(true);
    setError("");
    try {
      await onSave(p);
      toast.success("预设已保存");
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
      setReplacement(null);
    }
  };
  const submit = async () => {
    try {
      const p =
        mode === "import"
          ? parseProfileJson(raw)
          : profile
            ? renameProfile(profile, name)
            : null;
      if (!p) return;
      const normalized = await profilesValidate(p);
      if (names.includes(normalized.name)) setReplacement(normalized);
      else await save(normalized);
    } catch (e) {
      setError(String(e));
    }
  };
  const json = profile ? exportProfileJson(profile) : "";
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(json);
      toast.success("JSON 已复制");
    } catch {
      setError("剪贴板不可用，请从下方文本选择并复制，或重试。");
    }
  };
  return (
    <>
      <Dialog
        open={!!mode}
        onOpenChange={(open) => {
          if (!open && !saving) onClose();
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {mode === "save"
                ? "另存为预设"
                : mode === "import"
                  ? "粘贴导入预设"
                  : "导出预设 JSON"}
            </DialogTitle>
            <DialogDescription>
              {mode === "save"
                ? "保存完整草稿，名称也是命令行使用的档名。"
                : mode === "import"
                  ? "先由内核校验，成功保存后才切换当前方案。"
                  : "完整配置可用于备份或导入；复制失败时可手动选择文本。"}
            </DialogDescription>
          </DialogHeader>
          {mode === "save" && (
            <Input
              aria-label="新预设名称"
              placeholder="例如 my-web"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          )}
          {mode === "import" && (
            <Textarea
              aria-label="导入的预设 JSON"
              rows={12}
              value={raw}
              onChange={(e) => setRaw(e.target.value)}
            />
          )}
          {mode === "export" && (
            <Textarea
              aria-label="导出的预设 JSON"
              readOnly
              rows={12}
              value={json}
            />
          )}
          {error && (
            <p className="field-error" role="alert">
              {error}
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" disabled={saving} onClick={onClose}>
              返回
            </Button>
            {mode === "export" ? (
              <Button onClick={() => void copy()}>复制 JSON</Button>
            ) : (
              <Button disabled={saving} onClick={() => void submit()}>
                {saving ? "保存中…" : "保存预设"}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <AlertDialog
        open={!!replacement}
        onOpenChange={(open) => {
          if (!open) setReplacement(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>替换已保存预设？</AlertDialogTitle>
            <AlertDialogDescription>
              已存在「{replacement?.name}」，确认将替换完整配置。
              {replacement && ["default", "jpg"].includes(replacement.name)
                ? "这会影响 Finder 快速操作和后续后台入口。"
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>返回</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => replacement && void save(replacement)}
            >
              确认替换
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
