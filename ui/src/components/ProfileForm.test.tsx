import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ProfileForm } from "./ProfileForm";
import { profilesShow, type Profile } from "../lib/bridge";
const render = (profile: Profile | null, busy = false) =>
  renderToStaticMarkup(
    <ProfileForm
      profile={profile}
      onChange={() => {}}
      busy={busy}
      dirty={false}
      onSaveAs={() => {}}
      onUpdate={() => {}}
    />,
  );
describe("本批参数表单", () => {
  it("只显示一处预设名称，说明临时调整语义", async () => {
    const html = render(await profilesShow("default"));
    expect(html).toContain("日常压缩");
    expect(html).toContain("仅调整本批");
    expect(html).not.toContain('aria-label="处理预设"');
  });
  it("目标大小与无损模式不显示强度滑杆", async () => {
    const target = await profilesShow("default");
    target.quality = { ...target.quality, kind: "targetBytes", level: null, targetBytes: 500_000 };
    const mail = render(target);
    expect(mail).toContain("目标大小");
    expect(mail).toContain("受画质下限影响");
    expect(mail).not.toContain('aria-label="压缩强度"');
    const archive = render(await profilesShow("archive"));
    expect(archive).not.toContain('aria-label="压缩强度"');
    expect(archive).toContain("无损模式");
  });
  it("运行时参数只读", async () => {
    const html = render(await profilesShow("default"), true);
    expect(html).toContain("<fieldset disabled");
    expect(html).toContain("本批参数已冻结");
  });
  it("推荐压缩默认折叠参数，不要求用户调强度", async () => {
    const html = render(await profilesShow("default"));
    expect(html).toContain("推荐压缩");
    expect(html).toContain("高级设置");
    expect(html).not.toContain('aria-label="质量模式"');
    expect(html).not.toContain('aria-label="压缩强度"');
  });
  it("保留特殊尺寸策略并明确展示", async () => {
    const p = await profilesShow("default");
    p.resize.mode = "always";
    const html = render(p);
    expect(html).toContain("保留导入的尺寸策略");
    expect(html).toContain("always");
  });
  it("加载失败时保留占位", () => {
    expect(render(null)).toContain("读取处理设置中");
  });
});
