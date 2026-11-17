import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { Profile } from "../lib/bridge";
import { ProfileForm } from "./ProfileForm";

const profile: Profile = {
  name: "default",
  pipeline: { classify: true, engine: { jpeg: "mozjpeg" } },
  resize: { maxWidth: null, maxHeight: null, keepRatio: true, mode: "shrinkOnly", cropAnchor: null },
  quality: { kind: "level", level: 4, targetBytes: null, minQualityFloor: 3 },
  format: { output: "auto", animated: "keep" },
  metadata: { exif: "privacy", icc: "srgb" },
  output: { dir: "sibling", suffixTemplate: "-{profile}", largerPolicy: "keepOriginal", undoManifest: true },
  ui: { notify: true, sound: false, openGUIOnFail: true },
};

describe("ProfileForm", () => {
  it("渲染下拉与档位字段", () => {
    const html = renderToStaticMarkup(
      <ProfileForm profile={profile} names={["default", "web"]} selected="default" onSelect={() => {}} />,
    );
    expect(html).toContain("level 4");
    expect(html).toContain("auto · 动图 keep");
    expect(html).toContain("EXIF privacy");
    expect(html).toContain("负优化拦截开");
    expect(html).toContain("撤销清单开");
    // 下拉选项来自磁盘档列表
    expect(html).toContain('value="web"');
  });

  it("targetBytes 档显示目标与 floor", () => {
    const mail: Profile = {
      ...profile,
      name: "mail",
      quality: { kind: "targetBytes", level: null, targetBytes: 500_000, minQualityFloor: 3 },
    };
    const html = renderToStaticMarkup(
      <ProfileForm profile={mail} names={["mail"]} selected="mail" onSelect={() => {}} />,
    );
    expect(html).toContain("目标 500000 字节");
    expect(html).toContain("floor 3");
  });

  it("无损档显示「无损」", () => {
    const archive: Profile = {
      ...profile,
      name: "archive",
      quality: { kind: "lossless", level: null, targetBytes: null, minQualityFloor: 3 },
    };
    const html = renderToStaticMarkup(
      <ProfileForm profile={archive} names={["archive"]} selected="archive" onSelect={() => {}} />,
    );
    expect(html).toContain("无损");
  });

  it("profile 未加载时显示占位", () => {
    const html = renderToStaticMarkup(
      <ProfileForm profile={null} names={[]} selected="" onSelect={() => {}} />,
    );
    expect(html).toContain("读取中");
  });
});
