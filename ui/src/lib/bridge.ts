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

const FALLBACK_PROFILES = ["archive", "default", "jpg", "mail", "screenshot", "web"];
/** 浏览器降级的内存档案表（页面会话内有效；保存/导入后列表随之变化） */
const fallbackStore = new Map<string, Profile>();

const baseProfile = (): Profile => ({
  name: "default",
  pipeline: { classify: true, engine: { jpeg: "mozjpeg" } },
  resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "shrinkOnly", cropAnchor: null },
  quality: { kind: "level", level: 4, targetBytes: null, minQualityFloor: 3 },
  format: { output: "auto", animated: "keep" },
  metadata: { exif: "privacy", icc: "srgb" },
  output: { dir: "sibling", dirPath: null, suffixTemplate: "-{profile}", largerPolicy: "keepOriginal", undoManifest: true },
  ui: { notify: true, sound: false, openGUIOnFail: true },
});

/** 浏览器降级：与 fixtures/ 五档字段一致，便于脱离 Tauri 验证表单联动 */
const FALLBACK_PROFILES_DETAIL: Record<string, () => Profile> = {
  default: baseProfile,
  web: () => ({
    ...baseProfile(),
    format: { output: "webp", animated: "webp" },
    quality: { kind: "level", level: 5, targetBytes: null, minQualityFloor: 3 },
  }),
  archive: () => ({
    ...baseProfile(),
    quality: { kind: "lossless", level: null, targetBytes: null, minQualityFloor: 3 },
    format: { output: "jxl", animated: "keep" },
    resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "none", cropAnchor: null },
  }),
  jpg: () => ({
    ...baseProfile(),
    format: { output: "jpeg", animated: "keep" },
  }),
  mail: () => ({
    ...baseProfile(),
    resize: { maxWidth: 1920, maxHeight: null, keepRatio: true, mode: "shrinkOnly", cropAnchor: null },
    quality: { kind: "targetBytes", level: null, targetBytes: 500_000, minQualityFloor: 3 },
  }),
  screenshot: () => ({
    ...baseProfile(),
    quality: { kind: "lossless", level: null, targetBytes: null, minQualityFloor: 3 },
    resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "none", cropAnchor: null },
  }),
};

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  return tauriInvoke<T>(cmd, args);
}

export async function coreVersion(): Promise<string> {
  if (!isTauri()) return "browser-dev";
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
    if (saved) return saved;
    const factory = FALLBACK_PROFILES_DETAIL[name] ?? baseProfile;
    return { ...factory(), name };
  }
  return invoke<Profile>("profiles_show", { name });
}

export async function profilesSave(profile: Profile): Promise<void> {
  if (!isTauri()) {
    // 浏览器降级：只写内存表（不落盘）
    fallbackStore.set(profile.name, profile);
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
  savedPct: -56.4,
  engine: "mozjpeg",
  elapsedMs: 96,
  status: "done",
});

/** 两图 SSIM（与 benchkit 同口径：ffmpeg ssim，1=完全一致） */
export async function ssim(input: string, output: string): Promise<number> {
  if (!isTauri()) return 0.9812; // 浏览器降级：演示值
  return invoke<number>("ssim", { input, output });
}

/** 本地文件 → 可放进 <img src> 的地址（Tauri asset protocol）；浏览器返回 null */
export function fileSrc(path: string | null): string | null {
  if (!path || !isTauri()) return null;
  const prefix = "asset://localhost/";
  return prefix + encodeURIComponent(path);
}

export async function compress(paths: string[], profile: string): Promise<CompressReport[]> {
  if (!isTauri()) {
    const total = paths.length;
    const reports: CompressReport[] = [];
    for (const [index, path] of paths.entries()) {
      progressHandlers.forEach((h) => h({ path, index, total, status: "started" }));
      await new Promise((r) => setTimeout(r, 120));
      const report = demoReport(path, profile);
      reports.push(report);
      progressHandlers.forEach((h) => h({ path, index, total, status: "done", report }));
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
  let unlisten: (() => void) | undefined;
  void (async () => {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<ProgressEvent>("yali://progress", (ev) => handler(ev.payload));
  })();
  return () => unlisten?.();
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

  let unlisten: (() => void) | undefined;
  void (async () => {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") handler(event.payload.paths);
    });
  })();
  return () => unlisten?.();
}
