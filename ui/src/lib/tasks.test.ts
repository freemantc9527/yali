import { describe, expect, it } from "vitest";
import type { CompressReport } from "./bridge";
import { applyProgress, reconcile, type TaskRow } from "./tasks";

const report = (input: string): CompressReport => ({
  input,
  output: input.replace(".jpg", "-default.jpg"),
  inputBytes: 1000,
  outputBytes: 400,
  savedBytes: 600,
  savedPct: -60,
  engine: "mozjpeg",
  elapsedMs: 20,
  status: "done",
});

const row = (path: string, index: number): TaskRow => ({ path, index, running: true });

describe("applyProgress（乱序安全）", () => {
  it("started → running，done → 报告", () => {
    let rows = [row("/a.jpg", 0), row("/b.jpg", 1)];
    rows = applyProgress(rows, { path: "/a.jpg", index: 0, total: 2, status: "started" });
    expect(rows[0].running).toBe(true);
    rows = applyProgress(rows, { path: "/a.jpg", index: 0, total: 2, status: "done", report: report("/a.jpg") });
    expect(rows[0].running).toBe(false);
    expect(rows[0].report?.output).toBe("/a-default.jpg");
    expect(rows[1].running).toBe(true);
  });

  it("failed → 错误文案，且清掉 running", () => {
    const rows = applyProgress([row("/x.png", 3)], {
      path: "/x.png",
      index: 3,
      total: 1,
      status: "failed",
      error: "引擎缺失",
    });
    expect(rows[0].running).toBe(false);
    expect(rows[0].error).toBe("引擎缺失");
  });

  it("事件比行先到（乱序）也不崩：自动补行", () => {
    const rows = applyProgress([], { path: "/late.jpg", index: 5, total: 6, status: "started" });
    expect(rows).toHaveLength(1);
    expect(rows[0].path).toBe("/late.jpg");
  });

  it("未知状态忽略", () => {
    const input = [row("/a.jpg", 0)];
    const rows = applyProgress(input, {
      path: "/a.jpg",
      index: 0,
      total: 1,
      status: "weird" as never,
    });
    expect(rows[0]).toEqual(input[0]);
  });
});

describe("reconcile（事件丢失兜底）", () => {
  it("用 command 结果补齐缺失报告，且不覆盖已有报告", () => {
    const existing = { ...row("/a.jpg", 0), running: false, report: report("/a.jpg") };
    const rows = reconcile([existing, row("/b.jpg", 1)], [report("/a.jpg"), report("/b.jpg")]);
    expect(rows[1].report?.input).toBe("/b.jpg");
    expect(rows[1].running).toBe(false);
    expect(rows[0].report).toBe(existing.report); // 未被替换
  });

  it("相对路径/绝对路径都能对上", () => {
    const rows = reconcile([row("photo.jpg", 0)], [report("/abs/photo.jpg")]);
    expect(rows[0].report?.input).toBe("/abs/photo.jpg");
  });
});
