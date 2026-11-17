import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { TaskList, TaskRowView, type TaskRow } from "./TaskList";

const doneRow: TaskRow = {
  path: "/tmp/photo.jpg",
  index: 0,
  running: false,
  report: {
    input: "/tmp/photo.jpg",
    output: "/tmp/photo-default.jpg",
    inputBytes: 766_000,
    outputBytes: 334_000,
    savedBytes: 432_000,
    savedPct: -56.4,
    engine: "mozjpeg",
    elapsedMs: 96,
    status: "done",
  },
};

describe("TaskRowView", () => {
  it("完成行显示 输入 → 输出 与压缩率", () => {
    const html = renderToStaticMarkup(<TaskRowView row={doneRow} />);
    expect(html).toContain("photo.jpg");
    expect(html).toContain("748.0 KB → 326.2 KB");
    expect(html).toContain("-56.4%");
    expect(html).toContain("mozjpeg");
    expect(html).not.toContain("⚠");
  });

  it("负优化件显示 ⚠ 与「未落盘」", () => {
    const row: TaskRow = {
      path: "/tmp/tiny.jpg",
      index: 1,
      running: false,
      report: {
        input: "/tmp/tiny.jpg",
        output: null,
        inputBytes: 52_379,
        outputBytes: null,
        savedBytes: null,
        savedPct: null,
        engine: "mozjpeg",
        elapsedMs: 79,
        status: "larger_output",
      },
    };
    const html = renderToStaticMarkup(<TaskRowView row={row} />);
    expect(html).toContain("⚠ 负优化");
    expect(html).toContain("未落盘");
  });

  it("失败行显示错误信息", () => {
    const html = renderToStaticMarkup(
      <TaskRowView row={{ path: "/tmp/x.png", index: 2, running: false, error: "引擎缺失" }} />,
    );
    expect(html).toContain("✗ 失败");
    expect(html).toContain("引擎缺失");
  });

  it("处理中行显示占位", () => {
    const html = renderToStaticMarkup(<TaskRowView row={{ path: "/tmp/a.jpg", index: 3, running: true }} />);
    expect(html).toContain("处理中");
  });
});

describe("TaskList", () => {
  it("空列表给出引导文案", () => {
    const html = renderToStaticMarkup(<TaskList rows={[]} />);
    expect(html).toContain("拖入图片开始");
  });

  it("多行渲染", () => {
    const html = renderToStaticMarkup(<TaskList rows={[doneRow, doneRow]} />);
    expect(html.match(/data-testid="task-row"/g)?.length).toBe(2);
  });
});
