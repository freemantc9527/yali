import { describe, expect, it } from "vitest";
import type { Profile } from "./bridge";
import {
  parseOptionalNumber,
  preflightError,
  withFormat,
  withOutput,
  withQuality,
} from "./profile-edit";

const base: Profile = {
  name: "default",
  pipeline: { classify: true, engine: { jpeg: "mozjpeg" } },
  resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "shrinkOnly", cropAnchor: null },
  quality: { kind: "level", level: 4, targetBytes: null, minQualityFloor: 3 },
  format: { output: "auto", animated: "keep" },
  metadata: { exif: "privacy", icc: "srgb" },
  output: { dir: "sibling", suffixTemplate: "-{profile}", largerPolicy: "keepOriginal", undoManifest: true },
  ui: { notify: true, sound: false, openGUIOnFail: true },
};

describe("profile-edit 不可变更新", () => {
  it("改质量不动其他字段、不改原对象", () => {
    const next = withQuality(base, { level: 7 });
    expect(next.quality.level).toBe(7);
    expect(next.quality.kind).toBe("level");
    expect(base.quality.level).toBe(4);
    expect(next.format).toEqual(base.format);
  });

  it("改格式/后缀同样保持不可变", () => {
    const a = withFormat(base, { output: "webp" });
    const b = withOutput(a, { suffixTemplate: "-web" });
    expect(a.format.output).toBe("webp");
    expect(b.output.suffixTemplate).toBe("-web");
    expect(base.output.suffixTemplate).toBe("-{profile}");
  });
});

describe("parseOptionalNumber", () => {
  it("空串→null，正整数→数值，非法→NaN（交给校验报错）", () => {
    expect(parseOptionalNumber("")).toBeNull();
    expect(parseOptionalNumber(" 12 ")).toBe(12);
    expect(parseOptionalNumber("0")).toBeNaN();
    expect(parseOptionalNumber("-3")).toBeNaN();
    expect(parseOptionalNumber("abc")).toBeNaN();
  });
});

describe("preflightError", () => {
  it("合法档无提示", () => {
    expect(preflightError(base)).toBeNull();
  });

  it("level 越界", () => {
    expect(preflightError(withQuality(base, { level: 0 }))).toContain("1–10");
    expect(preflightError(withQuality(base, { level: 11 }))).toContain("1–10");
  });

  it("targetBytes 模式缺目标值", () => {
    const p = withQuality(base, { kind: "targetBytes", targetBytes: null });
    expect(preflightError(p)).toContain("目标大小");
  });

  it("custom 目录必须填路径", () => {
    expect(preflightError(withOutput(base, { dir: "custom", dirPath: "" }))).toContain("自定义目录");
    expect(preflightError(withOutput(base, { dir: "custom", dirPath: "~/out" }))).toBeNull();
  });

  it("后缀为空会覆盖原图，必须拦下", () => {
    expect(preflightError(withOutput(base, { suffixTemplate: "" }))).toContain("后缀");
  });

  it("NaN 参与校验时报数字错误", () => {
    const p = withQuality(base, { level: Number.NaN });
    expect(preflightError(p)).not.toBeNull();
  });
});
