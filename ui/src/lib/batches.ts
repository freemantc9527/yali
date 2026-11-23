import type { CompressReport, Profile } from "./bridge";

export type Status =
  | "pending"
  | "queued"
  | "processing"
  | "done"
  | "larger_output"
  | "failed";
export interface Job {
  jobId: string;
  batchId: string | null;
  path: string;
  source: "manual" | "system";
  attempt: number;
  retryOf?: string;
  status: Status;
  report?: CompressReport;
  error?: string;
}
export interface Batch {
  batchId: string;
  profile: Profile | null;
  jobIds: string[];
  source: Job["source"];
  error?: string;
}
export interface BatchEvent {
  batchId: string;
  jobId: string;
  path: string;
  status: "started" | "done" | "larger_output" | "failed";
  report?: CompressReport;
  error?: string;
}
export interface Outcome extends Omit<BatchEvent, "batchId" | "status"> {
  status: "done" | "larger_output" | "failed";
}
export interface BatchRequest {
  batchId: string;
  items: { jobId: string; path: string }[];
  profile: Profile;
}
export interface State {
  jobs: Job[];
  batches: Batch[];
  active: string | null;
  view: string | null;
}
export const initialState: State = {
  jobs: [],
  batches: [],
  active: null,
  view: null,
};
export const terminal = (status: Status) =>
  ["done", "larger_output", "failed"].includes(status);
export const id = () => crypto.randomUUID();

/** 主按钮当前可提交的图片。 */
export function processingJobs(jobs: Job[]): Job[] {
  const pending = jobs.filter((job) => job.status === "pending" && job.source === "manual");
  return pending.length ? pending : repeatJobs(jobs.filter((job) => terminal(job.status)));
}

export function latestJobs(jobs: Job[]): Job[] {
  const latest = new Map<string, Job>();
  for (const job of jobs) latest.set(job.path, job);
  return [...latest.values()];
}

export function repeatJobs(jobs: Job[]): Job[] {
  // 尚未执行的行复用原 ID，让 submit 消费它；已完成的行才创建新 attempt。
  return latestJobs(jobs).map((job) => job.status === "pending" ? {
    ...job,
    source: "manual",
  } : ({
    ...job,
    jobId: id(),
    batchId: null,
    source: "manual",
    status: "pending",
    attempt: job.attempt + 1,
    retryOf: job.jobId,
    report: undefined,
    error: undefined,
  }));
}
export type Action =
  | { type: "add"; jobs: Job[] }
  | { type: "submit"; batch: Batch; jobs?: Job[] }
  | { type: "start"; batchId: string }
  | { type: "event"; event: BatchEvent }
  | { type: "finish"; batchId: string; outcomes: Outcome[] }
  | { type: "batchError"; batchId: string; error: string }
  | { type: "profile"; batchId: string; profile: Profile }
  | { type: "remove"; jobId: string }
  | { type: "clear"; all?: boolean }
  | { type: "removePath"; path: string }
  | { type: "view"; batchId: string };

function apply(jobs: Job[], event: BatchEvent, authoritative = false): Job[] {
  return jobs.map((job) => {
    if (job.batchId !== event.batchId || job.jobId !== event.jobId) return job;
    if (!authoritative && terminal(job.status)) return job;
    return {
      ...job,
      status: event.status === "started" ? "processing" : event.status,
      report: event.report,
      error: event.error,
    };
  });
}
export function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "add": {
      const pending = new Set(
        state.jobs
          .filter((j) => j.status === "pending" && j.source === "manual")
          .map((j) => j.path),
      );
      return {
        ...state,
        jobs: [
          ...state.jobs,
          ...action.jobs.filter(
            (j) =>
              j.source !== "manual" ||
              (!pending.has(j.path) && !!pending.add(j.path)),
          ),
        ],
      };
    }
    case "submit": {
      const existing = new Set(state.jobs.map((job) => job.jobId));
      // 新 attempt 与批次原子入队，不经过 pending 的路径去重。
      const jobs = [
        ...state.jobs,
        ...(action.jobs ?? []).filter((job) => !existing.has(job.jobId)),
      ];
      return {
        ...state,
        batches: [...state.batches, structuredClone(action.batch)],
        view: action.batch.batchId,
        jobs: jobs.map((j) =>
          action.batch.jobIds.includes(j.jobId)
            ? { ...j, batchId: action.batch.batchId, status: "queued" }
            : j,
        ),
      };
    }
    case "start":
      return { ...state, active: action.batchId };
    case "event":
      return { ...state, jobs: apply(state.jobs, action.event) };
    case "finish":
      return {
        ...state,
        active: state.active === action.batchId ? null : state.active,
        jobs: action.outcomes.reduce(
          (jobs, o) => apply(jobs, { ...o, batchId: action.batchId }, true),
          state.jobs,
        ),
      };
    case "batchError":
      return {
        ...state,
        active: state.active === action.batchId ? null : state.active,
        batches: state.batches.map((b) =>
          b.batchId === action.batchId ? { ...b, error: action.error } : b,
        ),
        jobs: state.jobs.map((j) =>
          j.batchId === action.batchId && !terminal(j.status)
            ? { ...j, status: "failed", error: action.error }
            : j,
        ),
      };
    case "profile":
      return {
        ...state,
        batches: state.batches.map((b) =>
          b.batchId === action.batchId
            ? {
                ...b,
                profile: structuredClone(action.profile),
                error: undefined,
              }
            : b,
        ),
      };
    case "removePath":
      if (state.jobs.some((j) => j.path === action.path && ["queued", "processing"].includes(j.status))) return state;
      return { ...state, jobs: state.jobs.filter((j) => j.path !== action.path) };
    case "remove":
      return {
        ...state,
        jobs: state.jobs.filter(
          (j) =>
            j.jobId !== action.jobId ||
            (j.batchId === state.active && state.active !== null),
        ),
      };
    case "clear": {
      if (state.active) return state;
      const jobs = state.jobs.filter(
        (job) =>
          (job.source === "system" && job.status === "queued") ||
          (!action.all && job.batchId !== state.view && job.status !== "pending"),
      );
      return {
        ...state,
        view: null,
        jobs,
        batches: state.batches.filter((batch) =>
          jobs.some((job) => job.batchId === batch.batchId),
        ),
      };
    }
    case "view":
      return { ...state, view: action.batchId };
  }
}

export function summary(jobs: Job[]) {
  let input = 0,
    output = 0,
    success = 0,
    failed = 0,
    unchanged = 0;
  for (const j of jobs) {
    if (j.status === "failed") failed++;
    if (j.report && j.status === "done" && j.report.outputBytes !== null) {
      input += j.report.inputBytes;
      output += j.report.outputBytes;
      success++;
    }
    if (j.report && j.status === "larger_output") {
      input += j.report.inputBytes;
      output += j.report.inputBytes;
      unchanged++;
    }
  }
  return {
    input,
    output,
    success,
    failed,
    unchanged,
    saved: input - output,
    percent: input > 0 ? ((input - output) / input) * 100 : null,
    processed: jobs.filter((j) => terminal(j.status)).length,
  };
}

/** 统一目录只接受可确定不冲突的命名族；{size} 也保守阻止同 stem。 */
export function collisionError(
  jobs: Job[],
  profile: Profile,
  infos?: ReadonlyMap<string, { format?: string | null }>,
): string | null {
  const keys = new Set<string>();
  for (const job of jobs) {
    const name = job.path.split("/").pop() ?? job.path;
    const stem = name.replace(/\.[^.]+$/, "").toLocaleLowerCase();
    const inputFormat = (
      infos?.get(job.path)?.format ?? name.split(".").pop()
    )?.toLowerCase();
    // 与 Registry::auto_of 一致：TIFF → JPEG，BMP → PNG。
    const autoFormat = ["tiff", "tif"].includes(inputFormat ?? "")
      ? "jpeg"
      : inputFormat === "bmp" ? "png" : inputFormat;
    const format =
      profile.format.output === "auto"
        ? inputFormat === "gif" && profile.format.animated === "webp"
          ? "webp"
          : autoFormat
        : profile.format.output;
    const directory =
      profile.output.dir === "sibling"
        ? job.path.slice(0, job.path.lastIndexOf("/"))
        : profile.output.dir;
    const key = `${directory.toLocaleLowerCase()}|${stem}|${format?.replace("jpeg", "jpg")}|${profile.output.suffixTemplate}`;
    if (keys.has(key))
      return "存在同名输出冲突，请改为原图文件夹或调整后缀、分批处理。";
    keys.add(key);
  }
  return null;
}

export function outputGroups(jobs: Job[]) {
  const groups = new Map<string, string[]>();
  for (const j of jobs)
    if (j.status === "done" && j.report?.output) {
      const dir = j.report.output.slice(0, j.report.output.lastIndexOf("/"));
      groups.set(dir, [...(groups.get(dir) ?? []), j.report.output]);
    }
  return [...groups].map(([dir, paths]) => ({ dir, paths }));
}
