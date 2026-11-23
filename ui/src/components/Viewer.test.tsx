import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { clampPos, fitImage, Viewer } from "./Viewer";
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
  it("适应窗口同时约束宽高，不放大小图，原始尺寸保留实际像素", () => {
    expect(fitImage({ width: 800, height: 1600 }, { width: 600, height: 300 })).toEqual({ width: 150, height: 300 });
    expect(fitImage({ width: 1600, height: 800 }, { width: 600, height: 300 })).toEqual({ width: 600, height: 300 });
    expect(fitImage({ width: 100, height: 50 }, { width: 600, height: 300 })).toEqual({ width: 100, height: 50 });
    expect(fitImage({ width: 1600, height: 800 }, { width: 600, height: 300 }, true)).toEqual({ width: 1600, height: 800 });
    expect(fitImage({ width: 1600, height: 800 }, { width: 0, height: -32 })).toEqual({ width: 1, height: 1 });
  });
  it("提供明确的显示尺寸与对比方式，底部没有滑杆和说明区", () => {
    const html = renderToStaticMarkup(
      <TooltipProvider>
        <Viewer job={job} jobs={[job]} onSelect={() => {}} onClose={() => {}} />
      </TooltipProvider>,
    );
    expect(html).toContain("返回工作台");
    expect(html).toContain("原始尺寸");
    expect(html).not.toContain("100%");
    expect(html).not.toContain("lucide-zoom-in");
    expect(html).toContain("适应窗口");
    expect(html).toContain("并排");
    expect(html).toContain("叠加");
    expect(html).not.toContain("compare-footer");
    expect(html).not.toContain("原图与处理后分割位置");
    expect(html).not.toContain("viewer-backdrop");
  });
});
