import { isWindows } from "../lib/platform";
import { useEffect, useState } from "react";
import {
  ArrowLeft,
  ChevronRight,
  FolderPlus,
  Info,
  SlidersHorizontal,
  Workflow,
  X,
} from "lucide-react";
import { toast } from "sonner";
import {
  chooseDirectory,
  settingsShow,
  settingsSave,
  type Settings,
} from "../lib/bridge";
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
import { Slider } from "./ui/slider";
import { FONT_SCALE_STEPS, imageToolsDescription } from "../lib/settings";
import { QuickActionsSettings } from "./QuickActionsSettings";
import { CacheSettings } from "./CacheSettings";
import { FfmpegSettings } from "./FfmpegSettings";
import appIcon from "../../../src-tauri/icons/128x128@2x.png";

export function SettingsView({
  version,
  onBack,
  onFontScaleChange,
}: {
  version: string;
  onBack: () => void;
  onFontScaleChange: (percent: number) => void;
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
      onFontScaleChange(draft.fontScalePercent);
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
    <section className="aux-view settings-view">
      <header className="aux-toolbar">
        <Button variant="ghost" onClick={back}>
          <ArrowLeft />
          返回工作台
        </Button>
        <div className="settings-toolbar-title">
          <span>偏好与运行环境</span>
          <h1>应用设置</h1>
        </div>
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
              <Tabs
                defaultValue="general"
                orientation="vertical"
                className="settings-tabs"
              >
                <TabsList
                  variant="line"
                  className="settings-nav"
                  aria-label="设置分区"
                >
                  <p className="settings-nav-label">设置</p>
                  <TabsTrigger value="general">
                    <SlidersHorizontal aria-hidden="true" />
                    <span>
                      <strong>通用</strong>
                      <small>外观与运行环境</small>
                    </span>
                  </TabsTrigger>
                  {!isWindows && <TabsTrigger value="automation">
                    <Workflow aria-hidden="true" />
                    <span>
                      <strong>自动化</strong>
                      <small>系统入口与监听</small>
                    </span>
                  </TabsTrigger>}
                  <TabsTrigger value="about">
                    <Info aria-hidden="true" />
                    <span>
                      <strong>关于</strong>
                      <small>鸭梨与版本信息</small>
                    </span>
                  </TabsTrigger>
                </TabsList>
                <div className="settings-panel">
                  <TabsContent value="general">
                    <div className="settings-panel-heading">
                      <div>
                        <h2>通用</h2>
                        <p className="hint">{isWindows ? "调整 Yali 的外观和本地运行环境。" : "调整 Yali 的外观、通知和本地运行环境。"}</p>
                      </div>
                    </div>
                    <fieldset disabled={busy} className="settings-fields">
                      <section className="settings-section" aria-labelledby="appearance-title">
                        <div className="settings-section-heading">
                          <div>
                            <h2 id="appearance-title">外观与行为</h2>
                            <p className="hint">保存后生效。</p>
                          </div>
                        </div>
                        <div className="setting-row font-scale-row">
                          <div>
                            <strong id="font-scale-label">字体大小</strong>
                            <p className="hint">默认 100%。</p>
                          </div>
                          <div className="font-scale-control">
                            <Slider
                              aria-label={`字体大小，${draft.fontScalePercent}%`}
                              aria-valuetext={`${draft.fontScalePercent}%`}
                              min={0}
                              max={FONT_SCALE_STEPS.length - 1}
                              step={1}
                              value={[FONT_SCALE_STEPS.indexOf(draft.fontScalePercent)]}
                              disabled={busy}
                              onValueChange={([index]) => setDraft({
                                ...draft,
                                fontScalePercent: FONT_SCALE_STEPS[index],
                              })}
                            />
                            <div className="font-scale-ticks" aria-hidden="true">
                              {FONT_SCALE_STEPS.map((percent) => (
                                <span key={percent}>{percent}%</span>
                              ))}
                            </div>
                          </div>
                        </div>
                        {!isWindows && <>
                        <label className="setting-row">
                          <div>
                            <strong>菜单栏常驻</strong>
                            <p className="hint">关闭后，关闭窗口会退出应用。</p>
                          </div>
                          <Switch
                            checked={draft.tray.enabled}
                            onCheckedChange={(enabled) => setDraft({
                              ...draft,
                              tray: { ...draft.tray, enabled },
                            })}
                          />
                        </label>
                        <label className="setting-row">
                          <div>
                            <strong>处理通知</strong>
                            <p className="hint">沿用应用当前通知行为。</p>
                          </div>
                          <Switch
                            checked={draft.notify.enabled}
                            onCheckedChange={(enabled) => setDraft({
                              ...draft,
                              notify: { ...draft.notify, enabled },
                            })}
                          />
                        </label>
                        </>}
                      </section>
                      <FfmpegSettings />
                      <CacheSettings />
                    </fieldset>
                  </TabsContent>
                  {!isWindows && <TabsContent value="automation">
                    <div className="settings-panel-heading">
                      <div>
                        <h2>自动化</h2>
                        <p className="hint">把常用处理放到系统入口，减少重复操作。</p>
                      </div>
                    </div>
                    <div className="settings-fields">
                      <section className="settings-section">
                        <QuickActionsSettings />
                      </section>
                      <fieldset disabled={busy} className="settings-section settings-fields">
                        <div className="settings-section-heading">
                          <div>
                            <h2>后台监听</h2>
                            <p className="hint">自动化使用已保存的 default 预设，与工作台本批草稿独立。</p>
                          </div>
                        </div>
                        <label className="setting-row">
                          <div>
                            <strong>剪贴板监听</strong>
                            <p className="hint">复制图片后处理，结果保存到下载文件夹。</p>
                          </div>
                          <Switch
                            checked={draft.clipboard.enabled}
                            onCheckedChange={(enabled) => setDraft({
                              ...draft,
                              clipboard: { ...draft.clipboard, enabled },
                            })}
                          />
                        </label>
                        <label className="setting-row">
                          <div>
                            <strong>监视文件夹</strong>
                            <p className="hint">自动处理所选文件夹里的新图片。</p>
                          </div>
                          <Switch
                            checked={draft.watch.enabled}
                            onCheckedChange={(enabled) => setDraft({
                              ...draft,
                              watch: { ...draft.watch, enabled },
                            })}
                          />
                        </label>
                        <ul className="directory-list">
                          {draft.watch.dirs.map((dir) => (
                            <li key={dir}>
                              <span>{dir}</span>
                              <IconButton
                                label={`移出监视清单 ${dir}`}
                                onClick={() => setDraft({
                                  ...draft,
                                  watch: {
                                    ...draft.watch,
                                    dirs: draft.watch.dirs.filter((p) => p !== dir),
                                  },
                                })}
                              >
                                <X />
                              </IconButton>
                            </li>
                          ))}
                        </ul>
                        <div className="settings-inline-actions">
                          <Button variant="outline" onClick={() => void addDirectory()}>
                            <FolderPlus />添加监视目录
                          </Button>
                          <span className="hint">移除只移出清单，不删除目录。</span>
                        </div>
                        <Collapsible>
                          <CollapsibleTrigger asChild>
                            <Button variant="ghost" className="settings-disclosure">
                              <ChevronRight aria-hidden="true" />
                              高级设置
                            </Button>
                          </CollapsibleTrigger>
                          <CollapsibleContent className="advanced-content">
                            <Field label="剪贴板最小体积（bytes）">
                              <NumberField
                                label="剪贴板最小体积"
                                allowZero
                                value={draft.clipboard.minBytes}
                                onChange={(minBytes) => setDraft({
                                  ...draft,
                                  clipboard: { ...draft.clipboard, minBytes: minBytes ?? NaN },
                                })}
                              />
                            </Field>
                            <Field label="文件变化防抖（ms）">
                              <NumberField
                                label="防抖时间"
                                allowZero
                                value={draft.watch.settleMs}
                                onChange={(settleMs) => setDraft({
                                  ...draft,
                                  watch: { ...draft.watch, settleMs: settleMs ?? NaN },
                                })}
                              />
                            </Field>
                          </CollapsibleContent>
                        </Collapsible>
                      </fieldset>
                    </div>
                  </TabsContent>}
                  <TabsContent value="about">
                    <div className="settings-panel-heading">
                      <h2>关于</h2>
                    </div>
                    <section className="about-panel" aria-labelledby="about-name">
                      <div className="about-brand">
                        <img src={appIcon} alt="鸭梨 Yali 应用图标" width={112} height={112} />
                        <div className="about-identity">
                          <h3 id="about-name">鸭梨 <span>Yali</span></h3>
                          <p className="about-slogan">让图片没有压力</p>
                        </div>
                      </div>
                      <p className="about-description">本地图片压缩与格式转换，让每张图片轻一点。</p>
                      <dl className="about-details">
                        <div>
                          <dt>版本</dt>
                          <dd className="about-version">{version}</dd>
                        </div>
                        <div>
                          <dt>图片工具</dt>
                          <dd>{imageToolsDescription(isWindows)}</dd>
                        </div>
                      </dl>
                    </section>
                  </TabsContent>
                </div>
              </Tabs>
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
