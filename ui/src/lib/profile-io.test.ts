import { describe, expect, it } from "vitest";
import type { Profile } from "./bridge";
import { exportProfileJson, parseProfileJson, renameProfile, validateProfileName } from "./profile-io";

const base: Profile = {
  name: "default",
  pipeline: { classify: true, engine: { jpeg: "mozjpeg" } },
  resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "shrinkOnly", cropAnchor: null },
  quality: { kind: "level", level: 4, targetBytes: null, minQualityFloor: 3 },
  format: { output: "auto", animated: "keep" },
  metadata: { exif: "privacy", icc: "srgb" },
  output: { dir: "sibling", dirPath: null, suffixTemplate: "-{profile}", largerPolicy: "keepOriginal", undoManifest: true },
  ui: { notify: true, sound: false, openGUIOnFail: true },
};

describe("validateProfileName", () => {
  it("拒绝空名、分隔符、点开头", () => {
    expect(validateProfileName("")).toContain("不能为空");
    expect(validateProfileName("  ")).toContain("不能为空");
    expect(validateProfileName("a/b")).toContain("分隔符");
    expect(validateProfileName("a\\b")).toContain("分隔符");
    expect(validateProfileName(".hidden")).toContain("点开头");
    expect(validateProfileName("x".repeat(65))).toContain("太长");
  });

  it("合法名通过", () => {
    expect(validateProfileName("my-web")).toBeNull();
    expect(validateProfileName("朋友圈")).toBeNull();
  });
});

describe("renameProfile", () => {
  it("返回新对象且只改档名", () => {
    const next = renameProfile(base, " my-web ");
    expect(next.name).toBe("my-web");
    expect(next.quality).toEqual(base.quality);
    expect(base.name).toBe("default");
  });

  it("非法档名抛错", () => {
    expect(() => renameProfile(base, "a/b")).toThrow("分隔符");
  });
});

describe("parseProfileJson", () => {
  it("解析合法 JSON", () => {
    const p = parseProfileJson(JSON.stringify(base));
    expect(p.name).toBe("default");
  });

  it("语法错误带解析信息", () => {
    expect(() => parseProfileJson('{"name": "x",')).toThrow(/JSON 解析失败/);
  });

  it("拒绝非对象（数组/null/字符串）", () => {
    expect(() => parseProfileJson("[]")).toThrow("JSON 对象");
    expect(() => parseProfileJson("null")).toThrow("JSON 对象");
    expect(() => parseProfileJson('"x"')).toThrow("JSON 对象");
  });
});

describe("exportProfileJson", () => {
  it("可被 parseProfileJson 回读（往返一致）", () => {
    const json = exportProfileJson(base);
    expect(parseProfileJson(json)).toEqual(base);
  });
});
