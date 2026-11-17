import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { Profile } from "../lib/bridge";
import { ProfileForm } from "./ProfileForm";

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

const render = (p: Profile | null, names = ["default"]) =>
  renderToStaticMarkup(
    <ProfileForm profile={p} names={names} selected={p?.name ?? ""} onSelect={() => {}} onSave={async () => {}} />,
  );

describe("ProfileForm（T-007 可编辑）", () => {
  it("level 档渲染等级下拉，选中当前值", () => {
    const html = render(base);
    expect(html).toContain('aria-label="quality-kind"');
    expect(html).toContain('aria-label="quality-level"');
    expect(html).toMatch(/value="4"[^>]*selected|selected[^>]*value="4"/);
  });

  it("targetBytes 档渲染目标与 floor 输入", () => {
    const mail: Profile = {
      ...base,
      quality: { kind: "targetBytes", level: null, targetBytes: 500_000, minQualityFloor: 3 },
    };
    const html = render(mail, ["mail"]);
    expect(html).toContain('aria-label="quality-targetBytes"');
    expect(html).toContain('value="500000"');
    expect(html).toContain('aria-label="quality-floor"');
    expect(html).not.toContain('aria-label="quality-level"');
  });

  it("无损档不渲染等级下拉", () => {
    const archive: Profile = {
      ...base,
      quality: { kind: "lossless", level: null, targetBytes: null, minQualityFloor: 3 },
    };
    const html = render(archive, ["archive"]);
    expect(html).not.toContain('aria-label="quality-level"');
    expect(html).not.toContain('aria-label="quality-targetBytes"');
  });

  it("输出与元数据字段可编辑（含后缀模板现值）", () => {
    const html = render(base);
    expect(html).toContain('aria-label="output-dir"');
    expect(html).toContain('aria-label="output-suffix"');
    expect(html).toContain('value="-{profile}"');
    expect(html).toContain('aria-label="metadata-exif"');
    expect(html).toContain('aria-label="format-output"');
    expect(html).toContain('aria-label="resize-mode"');
  });

  it("保存按钮初始不可点（无改动）且无状态提示", () => {
    const html = render(base);
    expect(html).toContain("保存");
    expect(html).toContain("disabled");
    expect(html).not.toContain('data-testid="save-status"');
  });

  it("custom 目录时出现路径输入", () => {
    const custom: Profile = {
      ...base,
      output: { ...base.output, dir: "custom", dirPath: "~/压缩输出" },
    };
    const html = render(custom);
    expect(html).toContain('aria-label="output-dir-path"');
    expect(html).toContain("~/压缩输出");
  });

  it("未加载时显示占位", () => {
    const html = render(null, []);
    expect(html).toContain("读取中");
  });
});
