import { useEffect, useState } from "react";
import { ArrowLeft, FolderPlus, X } from "lucide-react";
import { toast } from "sonner";
import {
  chooseDirectory,
  settingsShow,
  settingsSave,
  type Settings,
} from "../lib/bridge";
import { Separator } from "./ui/separator";
import { Button } from "./ui/button";
import { Switch } from "./ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "./ui/collapsible";
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
import { Field, IconButton } from "./Controls";
import { NumberField } from "./ProfileForm";

export function SettingsView({
  version,
  onBack,
}: {
  version: string;
  onBack: () => void;
}) {
  const [saved, setSaved] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const dirty = JSON.stringify(saved) !== JSON.stringify(draft);
  const load = () => {
    setError("");
    void settingsShow()
      .then((s) => {
        setSaved(s);
        setDraft(s);
      })
      .catch((e) => setError(String(e)));
  };
  useEffect(load, []);
  const back = () => {
    if (dirty) setLeaving(true);
    else onBack();
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (
        e.key === "Escape" &&
        !(
          e.target instanceof HTMLElement &&
          e.target.closest("[role=alertdialog],[role=dialog],[role=combobox]")
        )
      )
        back();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [dirty]);
  const save = async () => {
    if (!draft) return;
    setBusy(true);
    setError("");
    try {
      await settingsSave(draft);
      setSaved(structuredClone(draft));
      toast.success("设置已保存");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const addDirectory = async () => {
    if (!draft) return;
    try {
      const dir = await chooseDirectory();
      if (dir)
        setDraft({
          ...draft,
          watch: {
            ...draft.watch,
            dirs: [...new Set([...draft.watch.dirs, dir])],
          },
        });
    } catch (e) {
      toast.error(String(e));
    }
  };
  return (
    <section className="aux-view">
      <header className="aux-toolbar">
        <Button variant="ghost" onClick={back}>
          <ArrowLeft />
          返回工作台
        </Button>
        <h1>应用设置</h1>
      </header>
      <div className="aux-scroll">
        <div className="settings-container">
          {error && (
            <p role="alert" className="field-error">
              {error}
            </p>
          )}
          {!draft ? (
            <Button variant="outline" onClick={load}>
              重新读取设置
            </Button>
          ) : (
            <>
              <Tabs defaultValue="general">
                <TabsList>
                  <TabsTrigger value="general">通用</TabsTrigger>
                  <TabsTrigger value="automation">自动化</TabsTrigger>
                </TabsList>
                <TabsContent value="general">
                  <fieldset disabled={busy} className="settings-fields">
                    <label className="setting-row">
                      <div>
                        <strong>菜单栏常驻</strong>
                        <p className="hint">关闭后，关闭窗口会退出应用。</p>
                      </div>
                      <Switch
                        checked={draft.tray.enabled}
                        onCheckedChange={(enabled) =>
                          setDraft({
                            ...draft,
                            tray: { ...draft.tray, enabled },
                          })
                        }
                      />
                    </label>
                    <label className="setting-row">
                      <div>
                        <strong>处理通知</strong>
                        <p className="hint">沿用应用当前通知行为。</p>
                      </div>
                      <Switch
                        checked={draft.notify.enabled}
                        onCheckedChange={(enabled) =>
                          setDraft({
                            ...draft,
                            notify: { ...draft.notify, enabled },
                          })
                        }
                      />
                    </label>
                    <Separator />
                    <div className="setting-row">
                      <div>
                        <strong>鸭梨 Yali</strong>
                        <p className="hint">让图片没有压力 · 内核 {version}</p>
                      </div>
                    </div>
                  </fieldset>
                </TabsContent>
                <TabsContent value="automation">
                  <fieldset disabled={busy} className="settings-fields">
                    <p className="hint">
                      自动化使用已保存的 default 预设，与工作台本批草稿独立。
                    </p>
                    <label className="setting-row">
                      <div>
                        <strong>剪贴板监听</strong>
                        <p className="hint">
                          复制图片后处理，结果保存到下载文件夹。
                        </p>
                      </div>
                      <Switch
                        checked={draft.clipboard.enabled}
                        onCheckedChange={(enabled) =>
                          setDraft({
                            ...draft,
                            clipboard: { ...draft.clipboard, enabled },
                          })
                        }
                      />
                    </label>
                    <label className="setting-row">
                      <div>
                        <strong>监视文件夹</strong>
                        <p className="hint">自动处理所选文件夹里的新图片。</p>
                      </div>
                      <Switch
                        checked={draft.watch.enabled}
                        onCheckedChange={(enabled) =>
                          setDraft({
                            ...draft,
                            watch: { ...draft.watch, enabled },
                          })
                        }
                      />
                    </label>
                    <ul className="directory-list">
                      {draft.watch.dirs.map((dir) => (
                        <li key={dir}>
                          <span>{dir}</span>
                          <IconButton
                            label={`移出监视清单 ${dir}`}
                            onClick={() =>
                              setDraft({
                                ...draft,
                                watch: {
                                  ...draft.watch,
                                  dirs: draft.watch.dirs.filter(
                                    (p) => p !== dir,
                                  ),
                                },
                              })
                            }
                          >
                            <X />
                          </IconButton>
                        </li>
                      ))}
                    </ul>
                    <Button
                      variant="outline"
                      onClick={() => void addDirectory()}
                    >
                      <FolderPlus />
                      添加监视目录
                    </Button>
                    <p className="hint">移除只移出清单，不删除目录。</p>
                    <Collapsible>
                      <CollapsibleTrigger asChild>
                        <Button variant="ghost">高级设置</Button>
                      </CollapsibleTrigger>
                      <CollapsibleContent className="advanced-content">
                        <Field label="剪贴板最小体积（bytes）">
                          <NumberField
                            label="剪贴板最小体积"
                            allowZero
                            value={draft.clipboard.minBytes}
                            onChange={(minBytes) =>
                              setDraft({
                                ...draft,
                                clipboard: {
                                  ...draft.clipboard,
                                  minBytes: minBytes ?? NaN,
                                },
                              })
                            }
                          />
                        </Field>
                        <Field label="文件变化防抖（ms）">
                          <NumberField
                            label="防抖时间"
                            allowZero
                            value={draft.watch.settleMs}
                            onChange={(settleMs) =>
                              setDraft({
                                ...draft,
                                watch: {
                                  ...draft.watch,
                                  settleMs: settleMs ?? NaN,
                                },
                              })
                            }
                          />
                        </Field>
                      </CollapsibleContent>
                    </Collapsible>
                  </fieldset>
                </TabsContent>
              </Tabs>
              <p className="hint">修改仅在点击保存后生效。</p>
            </>
          )}
        </div>
      </div>
      <footer className="aux-footer">
        <Button disabled={!draft || !dirty || busy} onClick={() => void save()}>
          {busy ? "保存中…" : "保存设置"}
        </Button>
      </footer>
      <AlertDialog open={leaving} onOpenChange={setLeaving}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>放弃设置调整？</AlertDialogTitle>
            <AlertDialogDescription>
              尚未保存的设置将被放弃。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>返回</AlertDialogCancel>
            <AlertDialogAction onClick={onBack}>放弃调整</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}
