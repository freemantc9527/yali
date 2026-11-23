import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { TaskRowView } from "./TaskList";
import { TooltipProvider } from "./ui/tooltip";
import type { Job } from "../lib/batches";
const job: Job = {
  jobId: "a",
  batchId: "b",
  path: "/tmp/图片.jpg",
  source: "manual",
  attempt: 1,
  status: "done",
  report: {
    input: "/tmp/图片.jpg",
    output: "/tmp/图片-default.jpg",
    inputBytes: 1000,
    outputBytes: 420,
    savedBytes: 580,
    savedPct: -99,
    engine: "test",
    elapsedMs: 1,
    status: "done",
  },
};
const render = (next: Job) =>
  renderToStaticMarkup(
    <TooltipProvider>
      <TaskRowView
        job={next}
        active={null}
        selected={false}
        duplicate={false}
        onSelect={() => {}}
        onCompare={() => {}}
        onRetry={() => {}}
        onRemove={() => {}}
      />
    </TooltipProvider>,
  );
describe("任务结果展示", () => {
  it("从字节而非 savedPct 计算节省", () => {
    const html = render(job);
    expect(html).toContain("58.0%");
    expect(html).not.toContain("-99");
    expect(html).toContain("对比");
  });
  it("转换增大使用明确中性说明", () => {
    const html = render({
      ...job,
      report: { ...job.report!, outputBytes: 1100 },
    });
    expect(html).toContain("增加 10.0%");
    expect(html).toContain("已转换，体积增加");
  });
  it("未减小无虚假结果或对比入口", () => {
    const html = render({
      ...job,
      status: "larger_output",
      report: {
        ...job.report!,
        status: "larger_output",
        output: null,
        outputBytes: null,
      },
    });
    expect(html).toContain("未减小，已保留原图");
    expect(html).not.toContain(">对比<");
    expect(html).toContain(">原图<");
  });
  it("失败保留可读错误与重试入口", () => {
    const html = render({
      ...job,
      status: "failed",
      report: undefined,
      error: "无法读取图片",
    });
    expect(html).toContain("无法读取图片");
    expect(html).toContain("更多操作 图片.jpg");
  });
  it("处理中没有伪造单图百分比", () => {
    const html = render({ ...job, status: "processing", report: undefined });
    expect(html).toContain("处理中");
    expect(html).not.toContain("%");
  });
});
