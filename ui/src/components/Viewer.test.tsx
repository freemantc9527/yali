import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { clampPos, Viewer } from "./Viewer";
import { TooltipProvider } from "./ui/tooltip";
import type { Job } from "../lib/batches";
const job: Job = {
  jobId: "a",
  batchId: "b",
  path: "/tmp/photo.jpg",
  source: "manual",
  attempt: 1,
  status: "done",
  report: {
    input: "/tmp/photo.jpg",
    output: "/tmp/photo-default.jpg",
    inputBytes: 1000,
    outputBytes: 420,
    savedBytes: 580,
    savedPct: 58,
    engine: "test",
    elapsedMs: 1,
    status: "done",
  },
};
describe("对比查看器", () => {
  it("分割位置限制在范围内", () => {
    expect(clampPos(-5)).toBe(0);
    expect(clampPos(140)).toBe(100);
    expect(clampPos(50)).toBe(50);
  });
  it("大画布提供返回与缩放，底部没有滑杆和说明区", () => {
    const html = renderToStaticMarkup(
      <TooltipProvider>
        <Viewer job={job} jobs={[job]} onSelect={() => {}} onClose={() => {}} />
      </TooltipProvider>,
    );
    expect(html).toContain("返回工作台");
    expect(html).toContain("100%");
    expect(html).not.toContain("compare-footer");
    expect(html).not.toContain("原图与处理后分割位置");
    expect(html).not.toContain("viewer-backdrop");
  });
});
