import { describe, expect, it } from "vitest";
import { windowsUserAgent } from "./platform";

describe("桌面平台识别", () => {
  it("识别 WebView2 的 Windows 标识且保留 macOS", () => {
    expect(windowsUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")).toBe(true);
    expect(windowsUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")).toBe(false);
  });
});
