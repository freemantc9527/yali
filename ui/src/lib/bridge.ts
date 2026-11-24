import { isWindows } from "./platform";
import { preflightError } from "./profile-edit";
import { version as appVersion } from "../../package.json";
import { FONT_SCALE_STEPS } from "./settings";
import { convertFileSrc, invoke as tauriInvoke } from "@tauri-apps/api/core";
/**
 * 内核 command 的唯一调用入口。
 * 浏览器里（纯 vite dev / vitest）没有 Tauri，降级为占位实现，便于脱离壳调试 UI；
 * 组件不得直接 import @tauri-apps/api。
 */

/** 与 schema/profile.schema.json 对齐的最小只读视图（T-005 只做「读」） */
export interface Profile {
  name: string;
  pipeline: { classify: boolean; engine: { jpeg: string } };
  resize: {
    maxWidth: number | null;
    maxHeight: number | null;
    keepRatio: boolean;
    mode: string;
    cropAnchor: string | null;
  };
  quality: {
    kind: string;
    level: number | null;
    targetBytes: number | null;
    minQualityFloor: number;
  };
  format: { output: string; animated: string };
  metadata: { exif: string; icc: string };
  output: {
    dir: string;
    dirPath: string | null;
    suffixTemplate: string;
    largerPolicy: string;
    undoManifest: boolean;
  };
  ui: { notify: boolean; sound: boolean; openGUIOnFail: boolean };
}

export interface ProgressEvent {
  path: string;
  index: number;
  total: number;
  status: "started" | "done" | "failed";
  report?: CompressReport | null;
  error?: string | null;
}

/** 与内核 CompressReport（camelCase）一一对应 */
export interface CompressReport {
  input: string;
  output: string | null;
  inputBytes: number;
  outputBytes: number | null;
  savedBytes: number | null;
  savedPct: number | null;
  engine: string;
  elapsedMs: number;
  status: "done" | "larger_output";
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const FALLBACK_PROFILES = ["default", "jpg", "thumbnail", "archive"];
/** 浏览器降级的内存档案表（页面会话内有效；保存/导入后列表随之变化） */
const fallbackStore = new Map<string, Profile>();

const baseProfile = (): Profile => ({
  name: "default",
  pipeline: { classify: true, engine: { jpeg: "mozjpeg" } },
  resize: {
    maxWidth: null,
    maxHeight: null,
    keepRatio: true,
    mode: "shrinkOnly",
    cropAnchor: null,
  },
  quality: { kind: "recommended", level: 4, targetBytes: null, minQualityFloor: 3 },
  format: { output: "auto", animated: "keep" },
  metadata: { exif: "privacy", icc: "srgb" },
  output: {
    dir: "sibling",
    dirPath: null,
    suffixTemplate: "-{profile}",
    largerPolicy: "keepOriginal",
    undoManifest: true,
  },
  ui: { notify: true, sound: false, openGUIOnFail: true },
});

/** 浏览器降级：与四个出厂 fixtures 字段一致 */
const FALLBACK_PROFILES_DETAIL: Record<string, () => Profile> = {
  default: baseProfile,
  archive: () => ({
    ...baseProfile(),
    quality: {
      kind: "lossless",
      level: null,
      targetBytes: null,
      minQualityFloor: 3,
    },
    format: { output: "jxl", animated: "keep" },
    resize: {
      maxWidth: null,
      maxHeight: null,
      keepRatio: true,
      mode: "none",
      cropAnchor: null,
    },
  }),
  jpg: () => ({
    ...baseProfile(),
    format: { output: "jpeg", animated: "keep" },
  }),
  thumbnail: () => ({
    ...baseProfile(),
    resize: {
      maxWidth: 1024,
      maxHeight: 1024,
      keepRatio: true,
      mode: "shrinkOnly",
      cropAnchor: null,
    },
  }),
};

async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}

export async function coreVersion(): Promise<string> {
  if (!isTauri()) return appVersion;
  return invoke<string>("core_version");
}

export async function profilesList(): Promise<string[]> {
  if (!isTauri()) {
    return [...new Set([...FALLBACK_PROFILES, ...fallbackStore.keys()])].sort();
  }
  return invoke<string[]>("profiles_list");
}

export async function profilesShow(name: string): Promise<Profile> {
  if (!isTauri()) {
    const saved = fallbackStore.get(name);
    if (saved) return structuredClone(saved);
    const factory = FALLBACK_PROFILES_DETAIL[name] ?? baseProfile;
    return { ...factory(), name };
  }
  return invoke<Profile>("profiles_show", { name });
}

export async function profilesSave(profile: Profile): Promise<void> {
  if (!isTauri()) {
    // 浏览器降级：只写内存表（不落盘）
    fallbackStore.set(profile.name, structuredClone(profile));
    return;
  }
  return invoke<void>("profiles_save", { profile });
}

// 浏览器降级的进度广播（模拟 Tauri 事件，便于脱离壳调试逐行更新）
const progressHandlers = new Set<(e: ProgressEvent) => void>();
const demoReport = (p: string, profile: string): CompressReport => ({
  input: p,
  output: `${p.replace(/\.[^.]+$/, "")}-${profile}.jpg`,
  inputBytes: 766_000,
  outputBytes: 334_000,
  savedBytes: 432_000,
  savedPct: 56.4,
  engine: "mozjpeg",
  elapsedMs: 96,
  status: "done",
});

/** 两图 SSIM（与 benchkit 同口径：ffmpeg ssim，1=完全一致） */
export async function ssim(input: string, output: string): Promise<number> {
  if (!isTauri()) return 0.9812; // 浏览器降级：演示值
  return invoke<number>("ssim", { input, output });
}

/** SSIM 是可选增强：需要系统提供带 ssim 滤镜的 ffmpeg。 */
export async function ssimAvailable(): Promise<boolean> {
  if (!isTauri()) return true;
  return (await ffmpegStatus()).ssim;
}

export interface FfmpegStatus {
  available: boolean;
  path: string | null;
  ssim: boolean;
}

let ffmpegCheck: Promise<FfmpegStatus> | undefined;

/** 会话内共享检查结果，设置页可强制刷新；不将机器状态写入配置。 */
export async function ffmpegStatus(refresh = false): Promise<FfmpegStatus> {
  if (!isTauri()) {
    return { available: false, path: null, ssim: false };
  }
  if (!ffmpegCheck || refresh) {
    const check = invoke<FfmpegStatus>("ffmpeg_status");
    ffmpegCheck = check;
    void check.catch(() => {
      if (ffmpegCheck === check) ffmpegCheck = undefined;
    });
  }
  return ffmpegCheck;
}

/** 系统投递信号（Tauri 事件 `yali://open-files`）的浏览器降级订阅表 */
const openFileHandlers = new Set<() => void>();

/** 订阅「系统投递了图片」（打开方式 / Dock 拖入）。路径本身用 takePendingFiles 取。 */
export function onOpenFiles(handler: () => void): () => void {
  if (!isTauri()) {
    openFileHandlers.add(handler);
    return () => openFileHandlers.delete(handler);
  }
  return subscribe("yali://open-files", handler, handler);
}

/** 取走系统投递的图片路径（冷启动缓冲 + 事件唤醒共用）；取走即清空 */
export async function takePendingFiles(): Promise<string[]> {
  if (!isTauri()) return [];
  return invoke<string[]>("take_pending_files");
}

export async function compress(
  paths: string[],
  profile: string,
): Promise<CompressReport[]> {
  if (!isTauri()) {
    const total = paths.length;
    const reports: CompressReport[] = [];
    for (const [index, path] of paths.entries()) {
      progressHandlers.forEach((h) =>
        h({ path, index, total, status: "started" }),
      );
      await new Promise((r) => setTimeout(r, 120));
      const report = demoReport(path, profile);
      reports.push(report);
      progressHandlers.forEach((h) =>
        h({ path, index, total, status: "done", report }),
      );
    }
    return reports;
  }
  return invoke<CompressReport[]>("compress", { paths, profile });
}

/** 订阅逐文件进度（Tauri 事件 `yali://progress`；浏览器降级走本地广播） */
export function onProgress(handler: (e: ProgressEvent) => void): () => void {
  if (!isTauri()) {
    progressHandlers.add(handler);
    return () => progressHandlers.delete(handler);
  }
  return subscribe("yali://progress", handler);
}

/**
 * 文件拖入：Tauri 走原生 drag-drop 事件（拿到真实路径）；浏览器降级用 HTML5 drop。
 * 返回取消订阅函数。
 */
export function onFileDrop(handler: (paths: string[]) => void): () => void {
  if (!isTauri()) {
    const onDrop = (event: DragEvent) => {
      event.preventDefault();
      const files = Array.from(event.dataTransfer?.files ?? []);
      handler(files.map((f) => f.name));
    };
    const onDragOver = (event: DragEvent) => event.preventDefault();
    window.addEventListener("drop", onDrop);
    window.addEventListener("dragover", onDragOver);
    return () => {
      window.removeEventListener("drop", onDrop);
      window.removeEventListener("dragover", onDragOver);
    };
  }

  let disposed = false;
  let unlisten: (() => void) | undefined;
  void import("@tauri-apps/api/webview")
    .then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "drop") handler(event.payload.paths);
      }),
    )
    .then((off) => {
      if (disposed) off();
      else unlisten = off;
    })
    .catch(console.error);
  return () => {
    disposed = true;
    unlisten?.();
  };
}
export interface ImageInfo {
  path: string;
  canonicalPath?: string | null;
  bytes?: number | null;
  format?: string | null;
  width?: number | null;
  height?: number | null;
  error?: string | null;
}
export interface Settings {
  schemaVersion: number;
  fontScalePercent: number;
  clipboard: { enabled: boolean; minBytes: number };
  watch: { enabled: boolean; dirs: string[]; settleMs: number };
  tray: { enabled: boolean };
  notify: { enabled: boolean };
}
let demoSettings: Settings = {
  schemaVersion: 2,
  fontScalePercent: 100,
  clipboard: { enabled: false, minBytes: 65536 },
  watch: { enabled: false, dirs: [], settleMs: 800 },
  tray: { enabled: true },
  notify: { enabled: true },
};
export const IMAGE_EXTENSIONS = [
  "png",
  "jpg",
  "jpeg",
  "heic",
  "heif",
  "gif",
  "webp",
  "avif",
  "jxl",
  "tif",
  "tiff",
  "bmp",
].filter((extension) => !isWindows || !["heic", "heif", "avif", "jxl"].includes(extension));
export const browserFiles = new Map<
  string,
  { url: string; file: File; width?: number; height?: number }
>();
export function registerFiles(files: File[]): string[] {
  return files.map((file) => {
    const key = `browser:${file.name}:${file.size}:${file.lastModified}`;
    if (!browserFiles.has(key))
      browserFiles.set(key, { file, url: URL.createObjectURL(file) });
    return key;
  });
}
export function releaseFiles(keep: Set<string>) {
  for (const [key, value] of browserFiles)
    if (!keep.has(key)) {
      URL.revokeObjectURL(value.url);
      browserFiles.delete(key);
    }
}
export async function inspectImages(paths: string[]): Promise<ImageInfo[]> {
  if (isTauri()) return invoke("inspect_images", { paths });
  return Promise.all(
    paths.map(async (path) => {
      const entry = browserFiles.get(path);
      if (!entry) return { path, error: "演示中请选择真实图片文件" };
      if (!entry.width) {
        await new Promise<void>((resolve) => {
          const img = new Image();
          img.onload = () => {
            entry.width = img.naturalWidth;
            entry.height = img.naturalHeight;
            resolve();
          };
          img.onerror = () => resolve();
          img.src = entry.url;
        });
      }
      return {
        path,
        canonicalPath: path,
        bytes: entry.file.size,
        format: entry.file.name.split(".").pop(),
        width: entry.width,
        height: entry.height,
      };
    }),
  );
}
export async function imagePreview(
  path: string,
  maxEdge: 0 | 192,
): Promise<PreviewImage> {
  if (!isTauri()) {
    await inspectImages([path]);
    const entry = browserFiles.get(path);
    if (!entry || !entry.width)
      throw new Error("此格式暂不可预览，仍可处理或定位结果");
    return { src: entry.url, release() {} };
  }
  const cached = await invoke<{ path: string; leaseId: string }>("image_preview", { path, maxEdge });
  let released = false;
  return {
    src: convertFileSrc(cached.path),
    release() {
      if (released) return;
      released = true;
      void invoke("release_preview", { leaseId: cached.leaseId }).catch(console.error);
    },
  };
}

export interface PreviewImage { src: string; release: () => void }

/** TIFF/BMP 在 Windows 使用 PNG 显示副本；其他格式按需转换，真实源路径保持不变。 */
export async function displayImage(path: string): Promise<PreviewImage> {
  if (!isTauri()) return imagePreview(path, 0);
  if (isWindows && /\.(?:tiff?|bmp)$/i.test(path)) return imagePreview(path, 0);
  const source = await invoke<string>("image_source", { path });
  const src = `${convertFileSrc(source)}?v=${Date.now()}`;
  const image = new Image();
  image.src = src;
  try {
    await image.decode();
    return { src, release() {} };
  } catch {
    return imagePreview(path, 0);
  }
}

export interface CacheStatus { bytes: number; protectedBytes: number; limitBytes: number }
export async function cacheStatus(): Promise<CacheStatus> {
  return isTauri() ? invoke("cache_status") : { bytes: 0, protectedBytes: 0, limitBytes: 256 * 1024 * 1024 };
}
export async function cacheClear(): Promise<CacheStatus> {
  return isTauri() ? invoke("cache_clear") : cacheStatus();
}
export async function chooseImages(): Promise<string[]> {
  if (!isTauri()) return [];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "图片", extensions: IMAGE_EXTENSIONS }],
  });
  return result === null ? [] : Array.isArray(result) ? result : [result];
}
export async function chooseDirectory(): Promise<string | null> {
  if (!isTauri()) throw new Error("目录选择仅在鸭梨桌面应用中可用");
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({ multiple: false, directory: true });
  return typeof result === "string" ? result : null;
}
export async function reveal(path: string) {
  if (!isTauri()) throw new Error("Finder 定位仅在鸭梨桌面应用中可用");
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}
export async function settingsShow(): Promise<Settings> {
  return isTauri() ? invoke("settings_show") : structuredClone(demoSettings);
}

export interface QuickActionsStatus {
  installed: boolean;
  needsRepair: boolean;
  canInstall: boolean;
  cliPath: string;
  message: string;
}

export async function quickActionsStatus(): Promise<QuickActionsStatus> {
  if (!isTauri()) return {
    installed: false, needsRepair: false, canInstall: false, cliPath: "",
    message: "请在鸭梨桌面应用中安装右键操作。",
  };
  return invoke("quick_actions_status");
}

export async function quickActionsInstall(): Promise<QuickActionsStatus> {
  if (!isTauri()) throw new Error("右键操作安装仅在鸭梨桌面应用中可用");
  return invoke("quick_actions_install");
}

export async function quickActionsUninstall(): Promise<QuickActionsStatus> {
  if (!isTauri()) throw new Error("右键操作卸载仅在鸭梨桌面应用中可用");
  return invoke("quick_actions_uninstall");
}
export async function settingsSave(settings: Settings): Promise<void> {
  if (isTauri()) return invoke("settings_save", { settings });
  if (!FONT_SCALE_STEPS.includes(settings.fontScalePercent))
    throw new Error("字体大小须为 80 / 100 / 120 / 150 / 180 / 200%");
  if (
    !Number.isInteger(settings.clipboard.minBytes) ||
    settings.clipboard.minBytes < 0 ||
    settings.clipboard.minBytes > 2 ** 30 ||
    !Number.isInteger(settings.watch.settleMs) ||
    settings.watch.settleMs < 0 ||
    settings.watch.settleMs > 10000
  )
    throw new Error("请检查最小体积（0 至 1GiB）与防抖时间（0 至 10000ms）");
  demoSettings = structuredClone(settings);
}
export async function profilesValidate(profile: Profile): Promise<Profile> {
  if (isTauri()) return invoke("profiles_validate", { profile });
  const base = baseProfile();
  for (const key of Object.keys(base) as (keyof Profile)[]) {
    if (!(key in profile)) throw new Error(`缺少 ${key}`);
    if (typeof base[key] === "object")
      for (const field of Object.keys(base[key]))
        if (!(field in (profile[key] as object)))
          throw new Error(`缺少 ${key}.${field}`);
  }
  if (
    !["auto", "jpeg", "png", "webp", "avif", "heic", "jxl"].includes(
      profile.format.output,
    )
  )
    throw new Error("输出格式无效");
  const error = preflightError(profile);
  if (error) throw new Error(error);
  return structuredClone(profile);
}
function subscribe<T>(
  name: string,
  handler: (event: T) => void,
  ready?: () => void,
): () => void {
  let disposed = false;
  let off: (() => void) | undefined;
  void import("@tauri-apps/api/event")
    .then(({ listen }) => listen<T>(name, (e) => handler(e.payload)))
    .then((unlisten) => {
      if (disposed) unlisten();
      else {
        off = unlisten;
        ready?.();
      }
    })
    .catch((e) => console.error(`事件订阅失败 ${name}`, e));
  return () => {
    disposed = true;
    off?.();
  };
}
const batchHandlers = new Set<
  (event: import("./batches").BatchEvent) => void
>();
export function onBatchProgress(
  handler: (event: import("./batches").BatchEvent) => void,
) {
  if (isTauri()) return subscribe("yali://batch-progress", handler);
  batchHandlers.add(handler);
  return () => {
    batchHandlers.delete(handler);
  };
}
export async function compressBatch(
  request: import("./batches").BatchRequest,
): Promise<{ batchId: string; outcomes: import("./batches").Outcome[] }> {
  if (isTauri()) return invoke("compress_batch", { request });
  await profilesValidate(request.profile);
  const outcomes: import("./batches").Outcome[] = [];
  for (const item of request.items) {
    batchHandlers.forEach((h) =>
      h({ batchId: request.batchId, ...item, status: "started" }),
    );
    await new Promise((resolve) => setTimeout(resolve, 350));
    const bytes = browserFiles.get(item.path)?.file.size ?? 0;
    const failure = /fail|失败/i.test(item.path);
    const unchanged = /larger|未减小/i.test(item.path);
    const grow = /grow|增加/i.test(item.path);
    const outputBytes = Math.round(bytes * (grow ? 1.15 : 0.42));
    const report: CompressReport = {
      input: item.path,
      output: unchanged ? null : item.path,
      inputBytes: bytes,
      outputBytes: unchanged ? null : outputBytes,
      savedBytes: unchanged ? null : bytes - outputBytes,
      savedPct:
        unchanged || !bytes ? null : ((bytes - outputBytes) / bytes) * 100,
      engine: "界面演示",
      elapsedMs: 350,
      status: unchanged ? "larger_output" : "done",
    };
    const outcome: import("./batches").Outcome = failure
      ? {
          ...item,
          status: "failed",
          error: "演示失败样例：无法读取图片，可重试",
        }
      : { ...item, status: report.status, report };
    outcomes.push(outcome);
    batchHandlers.forEach((h) => h({ batchId: request.batchId, ...outcome }));
  }
  return { batchId: request.batchId, outcomes };
}
