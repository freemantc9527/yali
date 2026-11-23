import { describe, expect, it } from "vitest";
import { imageToolsDescription } from "./settings";

describe("关于页的平台能力", () => {
  it("Windows 隐藏无损归档，macOS 保留", () => {
    expect(imageToolsDescription(true)).not.toContain("无损归档");
    expect(imageToolsDescription(false)).toContain("无损归档");
  });
});
