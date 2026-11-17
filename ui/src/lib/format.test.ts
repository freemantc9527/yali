import { describe, expect, it } from "vitest";
import { formatBytes, ratio } from "./format";

describe("formatBytes", () => {
  it("保持 B 级原值", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("逐级进位", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1_234_567)).toBe("1.2 MB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
  });

  it("拒绝非法输入", () => {
    expect(() => formatBytes(-1)).toThrow();
    expect(() => formatBytes(Number.NaN)).toThrow();
  });
});

describe("ratio", () => {
  it("压缩为负、膨胀为正", () => {
    expect(ratio(1000, 420)).toBe("-58.0%");
    expect(ratio(1000, 1100)).toBe("+10.0%");
    expect(ratio(1000, 1000)).toBe("0.0%");
  });
});
