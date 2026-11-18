import { describe, expect, it } from "vitest";
import {
  reducer,
  initialState,
  summary,
  collisionError,
  outputGroups,
  type Job,
  type State,
} from "./batches";
import { profilesShow } from "./bridge";
const job = (path = "/a/a.jpg", jobId = "j1"): Job => ({
  path,
  jobId,
  batchId: null,
  attempt: 1,
  status: "pending",
  source: "manual",
});
const report = {
  input: "/a/a.jpg",
  output: "/out/a.jpg",
  inputBytes: 1000,
  outputBytes: 400,
  savedBytes: 600,
  savedPct: -999,
  engine: "test",
  elapsedMs: 1,
  status: "done" as const,
};
async function submitted() {
  const profile = await profilesShow("default");
  let state = reducer(initialState, { type: "add", jobs: [job()] });
  state = reducer(state, {
    type: "submit",
    batch: { batchId: "b1", profile, jobIds: ["j1"], source: "manual" },
  });
  return { state, profile };
}
describe("批次业务状态", () => {
  it("本批快照与保存档及可编辑草稿分离", async () => {
    const { state, profile } = await submitted();
    profile.quality.level = 9;
    expect(state.batches[0].profile?.quality.level).toBe(4);
    expect((await profilesShow("default")).quality.level).toBe(4);
  });
  it("pending 去重只限相同绝对路径，不吞不同目录同名图", () => {
    const state = reducer(initialState, {
      type: "add",
      jobs: [job(), job("/a/a.jpg", "j2"), job("/b/a.jpg", "j3")],
    });
    expect(state.jobs.map((j) => j.jobId)).toEqual(["j1", "j3"]);
  });
  it("运行中新手动文件保留 pending，系统批次保留 queued", async () => {
    let { state } = await submitted();
    state = reducer(state, { type: "start", batchId: "b1" });
    state = reducer(state, {
      type: "add",
      jobs: [
        job("/b/b.jpg", "j2"),
        { ...job("/c/c.jpg", "j3"), source: "system" },
      ],
    });
    state = reducer(state, {
      type: "submit",
      batch: {
        batchId: "b2",
        profile: await profilesShow("default"),
        jobIds: ["j3"],
        source: "system",
      },
    });
    expect(state.jobs.map((j) => j.status)).toEqual([
      "queued",
      "pending",
      "queued",
    ]);
    expect(state.active).toBe("b1");
  });
  it("迟到 started、重复终态、其他 batch 和旧 attempt 不串行", async () => {
    let { state } = await submitted();
    state = reducer(state, {
      type: "event",
      event: {
        batchId: "b1",
        jobId: "j1",
        path: "/a/a.jpg",
        status: "done",
        report,
      },
    });
    for (const event of [
      {
        batchId: "b1",
        jobId: "j1",
        path: "/a/a.jpg",
        status: "started" as const,
      },
      {
        batchId: "b2",
        jobId: "j1",
        path: "/a/a.jpg",
        status: "failed" as const,
      },
      {
        batchId: "b1",
        jobId: "old",
        path: "/a/a.jpg",
        status: "failed" as const,
      },
    ])
      state = reducer(state, { type: "event", event });
    expect(state.jobs[0].status).toBe("done");
  });
  it("command 返回纠正事件，部分失败不覆盖成功", async () => {
    let { state } = await submitted();
    state = reducer(state, { type: "add", jobs: [job("/b/b.jpg", "j2")] });
    state = reducer(state, {
      type: "submit",
      batch: {
        batchId: "b2",
        profile: await profilesShow("default"),
        jobIds: ["j2"],
        source: "manual",
      },
    });
    state = reducer(state, {
      type: "finish",
      batchId: "b1",
      outcomes: [{ jobId: "j1", path: "/a/a.jpg", status: "done", report }],
    });
    state = reducer(state, {
      type: "finish",
      batchId: "b2",
      outcomes: [
        { jobId: "j2", path: "/b/b.jpg", status: "failed", error: "bad" },
      ],
    });
    expect(state.jobs.map((j) => j.status)).toEqual(["done", "failed"]);
  });
  it("汇总按有效字节加权，未减小为零节省，失败不计入", () => {
    const jobs: Job[] = [
      { ...job(), status: "done", report },
      {
        ...job("/b/b.jpg", "j2"),
        status: "larger_output",
        report: {
          ...report,
          inputBytes: 3000,
          output: null,
          outputBytes: null,
          status: "larger_output",
        },
      },
      { ...job("/c/c.jpg", "j3"), status: "failed" },
    ];
    expect(summary(jobs)).toMatchObject({
      input: 4000,
      output: 3400,
      saved: 600,
      percent: 15,
      success: 1,
      unchanged: 1,
      failed: 1,
      processed: 3,
    });
    expect(summary([]).percent).toBeNull();
    expect(
      summary([{ ...jobs[0], report: { ...report, outputBytes: 1200 } }]).saved,
    ).toBe(-200);
  });
  it("统一目录拒绝同名输出，原目录不误报，size 模板也保守检查", async () => {
    const p = await profilesShow("default");
    const jobs = [job("/a/photo.jpg"), job("/b/photo.jpg", "j2")];
    expect(collisionError(jobs, p)).toBeNull();
    p.output.dir = "downloads";
    p.output.suffixTemplate = "-{size}";
    expect(collisionError(jobs, p)).toContain("冲突");
  });
  it("多个结果目录分组，只收集实际有效产物", () => {
    const groups = outputGroups([
      { ...job(), status: "done", report },
      {
        ...job("/b/b.jpg", "j2"),
        status: "done",
        report: { ...report, output: "/another/b.jpg" },
      },
      {
        ...job("/c/c.jpg", "j3"),
        status: "larger_output",
        report: { ...report, output: null },
      },
    ]);
    expect(groups.map((g) => g.dir)).toEqual(["/out", "/another"]);
  });
  it("系统 default 加载失败保留投递，不会清空其他队列", async () => {
    let { state } = await submitted();
    state = reducer(state, {
      type: "batchError",
      batchId: "b1",
      error: "default 读取失败",
    });
    expect(state.jobs[0].path).toBe("/a/a.jpg");
    expect(state.jobs[0].error).toContain("default");
  });
  it("运行中禁止清空和移除本批，清空不会移除排队系统任务", async () => {
    let { state } = await submitted();
    state = reducer(state, { type: "start", batchId: "b1" });
    expect(reducer(state, { type: "clear" })).toBe(state);
    expect(reducer(state, { type: "remove", jobId: "j1" }).jobs).toHaveLength(
      1,
    );
  });
});

describe("批次调度边界", () => {
  it("重试与同路径待处理项并存时，提交任务必须实际入队", async () => {
    const profile = await profilesShow("default");
    const failed = { ...job(), batchId: "old", status: "failed" as const };
    const pending = job(failed.path, "pending");
    const retry = {
      ...failed,
      jobId: "retry",
      batchId: null,
      status: "pending" as const,
      attempt: 2,
      retryOf: failed.jobId,
    };
    const state = reducer(
      { ...initialState, jobs: [failed, pending] },
      {
        type: "submit",
        batch: { batchId: "retry-batch", profile, jobIds: [retry.jobId], source: "manual" },
        jobs: [retry],
      },
    );
    expect(state.jobs.find((j) => j.jobId === "retry")).toMatchObject({
      batchId: "retry-batch", status: "queued", attempt: 2, retryOf: failed.jobId,
    });
    expect(state.jobs.find((j) => j.jobId === "pending")).toEqual(pending);
    expect(state.jobs.find((j) => j.jobId === failed.jobId)).toEqual(failed);
  });
  it("TIFF 自动转 JPEG 的输出族与原 JPEG 也会冲突", async () => {
    const p = await profilesShow("default");
    p.output.dir = "downloads";
    const jobs = [job("/a/photo.tiff"), job("/b/photo.jpg", "j2")];
    const infos = new Map([
      [jobs[0].path, { format: "tiff" }],
      [jobs[1].path, { format: "jpeg" }],
    ]);
    expect(collisionError(jobs, p, infos)).toContain("冲突");
    p.output.dir = "sibling";
    expect(collisionError(jobs, p, infos)).toBeNull();
  });
  it("别的系统批次读取失败不释放当前运行批次", async () => {
    let { state } = await submitted();
    state = reducer(state, { type: "start", batchId: "b1" });
    state = reducer(state, {
      type: "batchError",
      batchId: "b2",
      error: "default 不可读",
    });
    expect(state.active).toBe("b1");
  });
  it("原目录下不同格式转成同格式也会冲突，GIF 转 WebP 按实际输出族检查", async () => {
    const p = await profilesShow("default");
    p.format.output = "jpeg";
    expect(
      collisionError([job("/a/photo.png"), job("/a/photo.webp", "j2")], p),
    ).toContain("冲突");
    p.output.dir = "downloads";
    p.format.output = "auto";
    p.format.animated = "webp";
    expect(
      collisionError([job("/a/photo.gif"), job("/b/photo.webp", "j2")], p),
    ).toContain("冲突");
  });
});

it("清空不丢失尚在加载 default 的系统投递批次", () => {
  const system = { ...job("/system/a.jpg"), source: "system" as const };
  let state = reducer(initialState, { type: "add", jobs: [system] });
  state = reducer(state, {
    type: "submit",
    batch: {
      batchId: "waiting",
      profile: null,
      jobIds: [system.jobId],
      source: "system",
    },
  });
  state = reducer(state, { type: "clear" });
  expect(state.jobs).toHaveLength(1);
  expect(state.batches[0].batchId).toBe("waiting");
});
