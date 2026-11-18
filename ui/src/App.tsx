import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
} from "react";
import {
  MoreHorizontal,
  PanelRight,
  Plus,
  Settings2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { ProfileForm } from "./components/ProfileForm";
import {
  ProfileManager,
  PresetDialog,
  type PresetMode,
} from "./components/ProfileManager";
import { TaskList, statusLabels } from "./components/TaskList";
import { BatchSummary } from "./components/BatchSummary";
import { Button } from "./components/ui/button";
import { Choice, IconButton } from "./components/Controls";
import { TooltipProvider } from "./components/ui/tooltip";
import { Toaster } from "./components/ui/sonner";
import { Collapsible, CollapsibleContent } from "./components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./components/ui/dropdown-menu";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
} from "./components/ui/alert-dialog";
import { Alert } from "./components/ui/alert";
import { Progress } from "./components/ui/progress";
import {
  initialState,
  reducer,
  id,
  collisionError,
  summary,
  type Job,
} from "./lib/batches";
import { preflightError } from "./lib/profile-edit";
import { profileLabel, sortProfiles } from "./lib/profile-labels";
import {
  chooseImages,
  compressBatch,
  coreVersion,
  inspectImages,
  isTauri,
  onBatchProgress,
  onFileDrop,
  onOpenFiles,
  profilesList,
  profilesSave,
  profilesShow,
  profilesValidate,
  registerFiles,
  releaseFiles,
  takePendingFiles,
  IMAGE_EXTENSIONS,
  type ImageInfo,
  type Profile,
} from "./lib/bridge";
import { DropZone } from "./components/DropZone";

type View = "workbench" | "profiles" | "settings" | "compare";
const SettingsView = lazy(() =>
  import("./components/SettingsView").then((module) => ({
    default: module.SettingsView,
  })),
);
const Viewer = lazy(() =>
  import("./components/Viewer").then((module) => ({ default: module.Viewer })),
);
type Confirmation = {
  title: string;
  description: string;
  action: () => void | Promise<void>;
};

export default function App() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [version, setVersion] = useState("读取中");
  const [names, setNames] = useState<string[]>([]);
  const [selectedProfileName, setSelectedProfileName] = useState("default");
  const [savedProfile, setSavedProfile] = useState<Profile | null>(null);
  const [draftProfile, setDraftProfile] = useState<Profile | null>(null);
  const [view, setView] = useState<View>("workbench");
  const [filter, setFilter] = useState("all");
  const [inspector, setInspector] = useState(() => window.innerWidth >= 960);
  const [selectedJob, setSelectedJob] = useState<string | null>(null);
  const [comparing, setComparing] = useState<string | null>(null);
  const [infos, setInfos] = useState(new Map<string, ImageInfo>());
  const [adding, setAdding] = useState(0);
  const [error, setError] = useState("");
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
  const [presetMode, setPresetMode] = useState<PresetMode>(null);
  const [exportProfile, setExportProfile] = useState<Profile | null>(null);
  const [dragging, setDragging] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const running = useRef(false);
  const submitting = useRef(false);
  const [validating, setValidating] = useState(false);
  const attemptedPaths = useRef(new Set<string>());
  const inflight = useRef(new Set<string>());
  const focusReturn = useRef<HTMLElement | null>(null);
  const busy = !!state.active || validating;
  const dirty = JSON.stringify(draftProfile) !== JSON.stringify(savedProfile);
  const pending = state.jobs.filter(
    (j) => j.status === "pending" && j.source === "manual",
  );
  const visibleBatch = state.batches.find((b) => b.batchId === state.view);
  const batchJobs = state.jobs.filter(
    (j) => j.batchId === state.view && state.view !== null,
  );
  const workspaceJobs = [...batchJobs, ...pending];
  const filtered = workspaceJobs.filter(
    (j) =>
      filter === "all" ||
      (filter === "processing"
        ? j.status === "processing" || j.status === "queued"
        : j.status === filter),
  );
  const count = (value: string) =>
    workspaceJobs.filter(
      (j) =>
        value === "all" ||
        (value === "processing"
          ? ["queued", "processing"].includes(j.status)
          : j.status === value),
    ).length;
  const loadProfile = async (name: string) => {
    setError("");
    try {
      const p = await profilesShow(name);
      setSelectedProfileName(name);
      setSavedProfile(structuredClone(p));
      setDraftProfile(structuredClone(p));
    } catch (e) {
      setError(`读取预设失败：${String(e)}`);
    }
  };
  const loadInitial = useCallback(async () => {
    try {
      const [v, list] = await Promise.all([coreVersion(), profilesList()]);
      setVersion(v);
      setNames(sortProfiles(list));
      await loadProfile(list.includes("default") ? "default" : list[0]);
    } catch (e) {
      setError(String(e));
    }
  }, []);
  useEffect(() => {
    void loadInitial();
  }, [loadInitial]);
  useEffect(
    () => onBatchProgress((event) => dispatch({ type: "event", event })),
    [],
  );
  const addPaths = async (
    paths: string[],
    source: Job["source"] = "manual",
  ) => {
    if (!paths.length) return;
    paths.forEach((p) => inflight.current.add(p));
    setAdding((n) => n + 1);
    try {
      const inspected = await inspectImages(paths);
      const infoMap = new Map<string, ImageInfo>();
      const seen = new Set<string>();
      const jobs: Job[] = inspected.flatMap((info) => {
        const path = info.canonicalPath ?? info.path;
        infoMap.set(path, info);
        if (seen.has(path)) return [];
        seen.add(path);
        return [
          {
            jobId: id(),
            batchId: null,
            path,
            source,
            attempt: 1,
            status: "pending" as const,
            ...(info.error ? { error: info.error } : {}),
          },
        ];
      });
      setInfos((previous) => new Map([...previous, ...infoMap]));
      dispatch({ type: "add", jobs });
      setFilter("all");
      if (source === "system") {
        const batchId = id();
        dispatch({
          type: "submit",
          batch: {
            batchId,
            jobIds: jobs.map((j) => j.jobId),
            profile: null,
            source,
          },
        });
        try {
          const profile = await profilesShow("default");
          dispatch({ type: "profile", batchId, profile });
        } catch (e) {
          dispatch({
            type: "batchError",
            batchId,
            error: `default 预设读取失败，投递文件已保留：${String(e)}`,
          });
        }
      }
    } catch (e) {
      const jobs: Job[] = paths.map((path) => ({
        jobId: id(),
        batchId: null,
        path,
        source,
        attempt: 1,
        status: "pending",
        error: String(e),
      }));
      dispatch({ type: "add", jobs });
      if (source === "system") {
        const batchId = id();
        dispatch({
          type: "submit",
          batch: {
            batchId,
            jobIds: jobs.map((j) => j.jobId),
            profile: null,
            source,
          },
        });
        dispatch({
          type: "batchError",
          batchId,
          error: `投递文件已保留，可重试 default：${String(e)}`,
        });
      }
      setError(`添加图片失败，路径已保留：${String(e)}`);
    } finally {
      paths.forEach((p) => inflight.current.delete(p));
      setAdding((n) => n - 1);
    }
  };
  const addRef = useRef(addPaths);
  addRef.current = addPaths;
  useEffect(() => {
    let alive = true,
      draining = false,
      again = false;
    const drain = async () => {
      if (draining) {
        again = true;
        return;
      }
      draining = true;
      do {
        again = false;
        try {
          const paths = await takePendingFiles();
          // take 是消费操作：StrictMode 清理发生后，也必须交付已取走的文件。
          if (paths.length) await addRef.current(paths, "system");
        } catch (e) {
          if (alive) setError(`读取系统投递失败：${String(e)}`);
        }
      } while (again && alive);
      draining = false;
    };
    const off = onOpenFiles(() => void drain());
    void drain();
    const drop = isTauri()
      ? onFileDrop((paths) => void addRef.current(paths))
      : () => {};
    return () => {
      alive = false;
      off();
      drop();
    };
  }, []);
  useEffect(() => {
    releaseFiles(
      new Set([...state.jobs.map((j) => j.path), ...inflight.current]),
    );
  }, [state.jobs]);
  useEffect(() => () => releaseFiles(new Set()), []);
  useEffect(() => {
    if (running.current || state.active) return;
    const batch = state.batches.find(
      (b) =>
        b.profile &&
        state.jobs.some(
          (j) => j.batchId === b.batchId && j.status === "queued",
        ),
    );
    if (!batch?.profile) return;
    const jobs = state.jobs.filter(
      (j) => j.batchId === batch.batchId && j.status === "queued",
    );
    const conflict = collisionError(jobs, batch.profile, infos);
    if (conflict) {
      dispatch({ type: "batchError", batchId: batch.batchId, error: conflict });
      return;
    }
    running.current = true;
    jobs.forEach((job) => attemptedPaths.current.add(job.path));
    dispatch({ type: "start", batchId: batch.batchId });
    void compressBatch({
      batchId: batch.batchId,
      items: jobs.map(({ jobId, path }) => ({ jobId, path })),
      profile: structuredClone(batch.profile),
    })
      .then((result) =>
        dispatch({
          type: "finish",
          batchId: batch.batchId,
          outcomes: result.outcomes,
        }),
      )
      .catch((e) =>
        dispatch({
          type: "batchError",
          batchId: batch.batchId,
          error: String(e),
        }),
      )
      .finally(() => {
        running.current = false;
      });
  }, [state]);
  const add = async () => {
    try {
      if (isTauri()) await addPaths(await chooseImages());
      else input.current?.click();
    } catch (e) {
      setError(String(e));
    }
  };
  const startJobs = async (jobs: Job[], profile: Profile) => {
    if (submitting.current) return;
    const problem =
      preflightError(profile) ?? collisionError(jobs, profile, infos);
    if (problem) {
      setError(problem);
      setInspector(true);
      return;
    }
    submitting.current = true;
    setValidating(true);
    try {
      await profilesValidate(profile);
    } catch (e) {
      setError(String(e));
      submitting.current = false;
      setValidating(false);
      return;
    }
    dispatch({
      type: "submit",
      jobs,
      batch: {
        batchId: id(),
        jobIds: jobs.map((j) => j.jobId),
        profile: structuredClone(profile),
        source: "manual",
      },
    });
    setFilter("all");
    setError("");
    submitting.current = false;
    setValidating(false);
  };
  const start = () => {
    if (!draftProfile || busy) return;
    if (!pending.length) {
      dispatch({ type: "clear" });
      setFilter("all");
      void add();
      return;
    }
    const run = () => startJobs(pending, structuredClone(draftProfile));
    const repeated = pending.some((j) => attemptedPaths.current.has(j.path));
    if (repeated)
      setConfirm({
        title: "处理已添加过的图片？",
        description: "同名处理结果可能被替换，原图保留。仅处理当前待处理项。",
        action: run,
      });
    else void run();
  };
  const retry = (jobs: Job[]) => {
    if (!draftProfile || busy) return;
    setConfirm({
      title: `重试 ${jobs.length} 张失败图片？`,
      description:
        "使用当前本批草稿建立新任务；旧结果保留。同名处理结果可能被替换，原图保留。",
      action: async () => {
        const next = jobs.map((j) => ({
          ...j,
          jobId: id(),
          batchId: null,
          status: "pending" as const,
          attempt: j.attempt + 1,
          retryOf: j.jobId,
          report: undefined,
          error: undefined,
          source: "manual" as const,
        }));
        const problem =
          preflightError(draftProfile) ??
          collisionError(next, draftProfile, infos);
        if (problem) {
          setError(problem);
          return;
        }
        await startJobs(next, draftProfile);
      },
    });
  };
  const changeProfile = (name: string) => {
    if (dirty)
      setConfirm({
        title: "放弃本批调整？",
        description: "切换预设会放弃尚未保存的本批参数。已提交批次不受影响。",
        action: () => loadProfile(name),
      });
    else void loadProfile(name);
  };
  const saveProfile = async (profile: Profile) => {
    const normalized = await profilesValidate(profile);
    await profilesSave(normalized);
    setNames(sortProfiles(await profilesList()));
    setSelectedProfileName(normalized.name);
    setSavedProfile(structuredClone(normalized));
    setDraftProfile(structuredClone(normalized));
  };
  const update = () => {
    if (!draftProfile) return;
    setConfirm({
      title: `更新「${profileLabel(draftProfile.name)}」？`,
      description: ["default", "jpg"].includes(draftProfile.name)
        ? "这会写入已保存预设，影响 Finder 快速操作和后续后台入口。"
        : "这会更新磁盘上的完整预设，供后续处理使用。",
      action: async () => {
        try {
          await saveProfile(draftProfile);
          toast.success("当前预设已更新");
        } catch (e) {
          setError(String(e));
        }
      },
    });
  };
  const enter = (next: View) => {
    focusReturn.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    setView(next);
  };
  const back = () => {
    setView("workbench");
    requestAnimationFrame(() => focusReturn.current?.focus());
  };
  const compare = (jobId: string) => {
    setComparing(jobId);
    enter("compare");
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        if (view === "workbench") enter("settings");
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [view]);
  const compareJobs = state.jobs.filter(
    (j) => !!j.report && j.batchId === state.view,
  );
  const viewed = state.jobs.find((j) => j.jobId === comparing);
  const filters = [
    "all",
    "pending",
    "processing",
    "done",
    "larger_output",
    "failed",
  ];
  const outputProfile = pending.length
    ? draftProfile
    : (visibleBatch?.profile ?? draftProfile);
  const outputHint =
    outputProfile?.output.dir === "sibling"
      ? "结果存到原图所在文件夹，原图保留"
      : outputProfile?.output.dir === "downloads"
        ? "结果存到下载文件夹，原图保留"
        : outputProfile?.output.dir === "custom"
          ? `保存到 ${outputProfile.output.dirPath}`
          : "保存位置跟随调用环境";
  return (
    <TooltipProvider>
      <a className="skip-link" href="#main-content">
        跳到图片列表
      </a>
      <div
        className="app-root"
        onDragOver={(e) => {
          if (!isTauri()) {
            e.preventDefault();
            setDragging(true);
          }
        }}
        onDragLeave={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node))
            setDragging(false);
        }}
        onDrop={(e) => {
          e.preventDefault();
          setDragging(false);
          if (!isTauri())
            void addPaths(registerFiles(Array.from(e.dataTransfer.files)));
        }}
      >
        <input
          hidden
          ref={input}
          type="file"
          multiple
          accept={IMAGE_EXTENSIONS.map((e) => `.${e}`).join(",")}
          onChange={(e) => {
            void addPaths(registerFiles(Array.from(e.target.files ?? [])));
            e.target.value = "";
          }}
        />
        {!isTauri() && (
          <div className="demo-banner">界面演示，未执行本地压缩</div>
        )}
        <div className="app-surface">
          <div className="workbench" hidden={view !== "workbench"}>
            <header className="toolbar">
              <Button variant="outline" onClick={() => void add()}>
                <Plus />
                <span>添加图片</span>
              </Button>
              <div className="toolbar-profile">
                <Choice
                  label="处理预设"
                  value={selectedProfileName}
                  options={names.map((n) => [n, profileLabel(n)])}
                  disabled={busy}
                  onChange={changeProfile}
                />
              </div>
              <div className="toolbar-spacer" />
              <Button
                variant="ghost"
                className="manage-presets"
                disabled={busy}
                onClick={() => enter("profiles")}
              >
                管理预设
              </Button>
              <IconButton
                label="处理设置"
                aria-expanded={inspector}
                aria-controls="parameter-inspector"
                onClick={() => setInspector((v) => !v)}
              >
                <PanelRight />
              </IconButton>
              <IconButton label="应用设置" onClick={() => enter("settings")}>
                <Settings2 />
              </IconButton>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="toolbar-overflow"
                    aria-label="更多操作"
                  >
                    <MoreHorizontal />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                  <DropdownMenuItem
                    disabled={busy}
                    onSelect={() => enter("profiles")}
                  >
                    管理预设
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => enter("settings")}>
                    应用设置
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </header>
            <Collapsible
              open={inspector}
              onOpenChange={setInspector}
              className={`workspace ${inspector ? "inspector-open" : ""}`}
            >
              <main id="main-content" tabIndex={-1} className="image-workspace">
                <div className="list-heading">
                  <div>
                    <h1>
                      图片 <span>{workspaceJobs.length} 张</span>
                    </h1>
                    {state.batches.length > 1 && (
                      <Choice
                        label="会话批次"
                        value={state.view ?? state.batches.at(-1)!.batchId}
                        onChange={(batchId) => {
                          dispatch({ type: "view", batchId });
                          setFilter("all");
                        }}
                        options={state.batches.map((b, i) => [
                          b.batchId,
                          `本次会话 · 第 ${i + 1} 批${b.source === "system" ? "（系统投递）" : ""}`,
                        ])}
                      />
                    )}
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={busy || !workspaceJobs.length}
                    onClick={() => dispatch({ type: "clear" })}
                  >
                    <Trash2 />
                    <span>清空列表</span>
                  </Button>
                </div>
                {error && (
                  <Alert className="error-banner">
                    {error}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setError("")}
                    >
                      收起
                    </Button>
                    {!draftProfile && (
                      <Button onClick={() => void loadInitial()}>
                        重试读取
                      </Button>
                    )}
                  </Alert>
                )}
                {visibleBatch?.error && (
                  <Alert className="error-banner">
                    {visibleBatch.error}
                    {!visibleBatch.profile && (
                      <Button
                        size="sm"
                        disabled={busy}
                        onClick={() =>
                          void profilesShow("default")
                            .then((profile) => {
                              const jobs = batchJobs.map((j) => ({
                                ...j,
                                jobId: id(),
                                batchId: null,
                                status: "pending" as const,
                                error: undefined,
                                attempt: j.attempt + 1,
                              }));
                              dispatch({ type: "add", jobs });
                              dispatch({
                                type: "submit",
                                batch: {
                                  batchId: id(),
                                  profile,
                                  jobIds: jobs.map((j) => j.jobId),
                                  source: "system",
                                },
                              });
                            })
                            .catch((e) => setError(String(e)))
                        }
                      >
                        重试 default 投递
                      </Button>
                    )}
                  </Alert>
                )}
                {workspaceJobs.length > 0 && (
                  <>
                    <div
                      className="filter-row"
                      role="group"
                      aria-label="任务状态筛选"
                    >
                      {filters
                        .filter((f) => f === "all" || count(f) > 0)
                        .map((f) => (
                          <Button
                            key={f}
                            variant={filter === f ? "secondary" : "ghost"}
                            size="sm"
                            aria-pressed={filter === f}
                            onClick={() => setFilter(f)}
                          >
                            {f === "all"
                              ? "全部"
                              : statusLabels[
                                  f as keyof typeof statusLabels
                                ]}{" "}
                            {count(f)}
                          </Button>
                        ))}
                      {workspaceJobs.some((j) => j.status === "failed") && (
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={busy}
                          onClick={() =>
                            retry(
                              workspaceJobs.filter(
                                (j) => j.status === "failed",
                              ),
                            )
                          }
                        >
                          重试失败项
                        </Button>
                      )}
                    </div>
                    <div className="list-column-labels">
                      <span>文件</span>
                      <span>原始大小 → 处理后</span>
                      <span>节省</span>
                      <span>状态</span>
                    </div>
                  </>
                )}
                {state.active === state.view && (
                  <Progress
                    aria-label="本批处理进度"
                    value={
                      batchJobs.length
                        ? (summary(batchJobs).processed / batchJobs.length) *
                          100
                        : 0
                    }
                    className="batch-progress"
                  />
                )}
                <div className="list-scroll">
                  {workspaceJobs.length ? (
                    <>
                      <TaskList
                        jobs={filtered}
                        infos={infos}
                        active={state.active}
                        selected={selectedJob}
                        onSelect={setSelectedJob}
                        onCompare={compare}
                        onRetry={(j) => retry([j])}
                        onRemove={(j) =>
                          dispatch({ type: "remove", jobId: j.jobId })
                        }
                      />
                      {!filtered.length && (
                        <p className="loading">没有此状态的图片</p>
                      )}
                      <DropZone
                        empty={false}
                        dragging={dragging}
                        reading={adding > 0}
                        onAdd={() => void add()}
                      />
                    </>
                  ) : (
                    <DropZone
                      empty
                      dragging={dragging}
                      reading={adding > 0}
                      onAdd={() => void add()}
                    />
                  )}
                </div>
              </main>
              <CollapsibleContent
                forceMount
                id="parameter-inspector"
                className="inspector"
              >
                <ProfileForm
                  key={selectedProfileName}
                  profile={draftProfile}
                  onChange={setDraftProfile}
                  busy={busy}
                  dirty={dirty}
                  onSaveAs={() => {
                    setExportProfile(null);
                    setPresetMode("save");
                  }}
                  onUpdate={update}
                />
              </CollapsibleContent>
            </Collapsible>
            <BatchSummary
              jobs={batchJobs}
              busy={busy && state.view === state.active}
              pending={pending.length}
              disabled={
                busy ||
                adding > 0 ||
                !draftProfile ||
                !!(draftProfile && preflightError(draftProfile)) ||
                (!pending.length && !batchJobs.length)
              }
              onStart={start}
              outputHint={outputHint}
            />
          </div>
          <Suspense fallback={<p className="loading">正在打开视图…</p>}>
            {view === "profiles" && (
              <ProfileManager
                names={names}
                onBack={back}
                onMode={(mode) => {
                  setExportProfile(null);
                  setPresetMode(mode);
                }}
                onExport={(p) => {
                  setExportProfile(p);
                  setPresetMode("export");
                }}
              />
            )}
            {view === "settings" && (
              <SettingsView version={version} onBack={back} />
            )}
            {view === "compare" && viewed?.report && (
              <Viewer
                job={viewed}
                jobs={compareJobs}
                onSelect={setComparing}
                onClose={back}
              />
            )}
          </Suspense>
        </div>
        <PresetDialog
          key={presetMode ?? "closed"}
          mode={presetMode}
          profile={presetMode === "export" ? exportProfile : draftProfile}
          names={names}
          onClose={() => setPresetMode(null)}
          onSave={saveProfile}
        />
        <AlertDialog
          open={!!confirm}
          onOpenChange={(open) => {
            if (!open) setConfirm(null);
          }}
        >
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>{confirm?.title}</AlertDialogTitle>
              <AlertDialogDescription>
                {confirm?.description}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>返回</AlertDialogCancel>
              <AlertDialogAction
                onClick={() => {
                  const action = confirm?.action;
                  setConfirm(null);
                  if (action) void action();
                }}
              >
                确认
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
        <Toaster richColors closeButton />
      </div>
    </TooltipProvider>
  );
}
