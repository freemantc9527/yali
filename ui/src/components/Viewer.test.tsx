import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CompressReport } from "../lib/bridge";
import { clampPos, Viewer } from "./Viewer";

const report: CompressReport = {
  input: "/tmp/photo.jpg",
  output: "/tmp/photo-default.jpg",
  inputBytes: 531019,
  outputBytes: 49178,
  savedBytes: 481841,
  savedPct: -90.7,
  engine: "mozjpeg",
  elapsedMs: 120,
  status: "done",
};

describe("clampPos", () => {
  it("限制在 0..100", () => {
    expect(clampPos(-10)).toBe(0);
    expect(clampPos(0)).toBe(0);
    expect(clampPos(50)).toBe(50);
    expect(clampPos(140)).toBe(100);
  });
});

describe("Viewer", () => {
  it("浏览器模式（无 asset protocol）给出说明而非破图", () => {
    const html = renderToStaticMarkup(<Viewer report={report} onClose={() => {}} />);
    expect(html).toContain("浏览器预览模式读不了本地文件路径");
    expect(html).toContain("对比");
    expect(html).toContain("计算中");
  });

  it("没有产出（负优化）时不渲染图，仍可打开", () => {
    const larger: CompressReport = { ...report, output: null, status: "larger_output" };
    const html = renderToStaticMarkup(<Viewer report={larger} onClose={() => {}} />);
    expect(html).toContain("浏览器预览模式");
  });
});
