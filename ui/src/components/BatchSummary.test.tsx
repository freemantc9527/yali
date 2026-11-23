import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { BatchSummary } from "./BatchSummary";
import type { Job } from "../lib/batches";

describe("底栏状态", () => {
  it("结果集中显示完成数和节省量，提供复用原图的处理入口", () => {
    const job: Job = { path: "/a.png", jobId: "j", batchId: "b", attempt: 1, source: "manual", status: "done", report: {
      input: "/a.png", output: "/a-default.png", inputBytes: 1000, outputBytes: 400, savedBytes: 600, savedPct: 60, elapsedMs: 1, engine: "test", status: "done",
    } };
    const html = renderToStaticMarkup(<BatchSummary jobs={[job]} busy={false} pending={0} disabled={false} onStart={() => {}} outputHint="原图保留 · 结果在原图文件夹" />);
    expect(html).toContain("完成 1 张");
    expect(html).toContain("节省");
    expect(html).toContain("60.0%");
    expect(html).toContain("重新处理 1 张");
    expect(html).not.toContain("共省");
    expect(html).not.toContain("→");
  });
});
