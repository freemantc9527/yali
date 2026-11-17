import { describe, expect, it } from "vitest";
import { compress, coreVersion, isTauri, profilesList, profilesShow } from "./bridge";

// node 环境没有 Tauri 注入，全部走浏览器降级分支——这正是 vitest 需要的可测性。
describe("bridge 浏览器降级", () => {
  it("识别非 Tauri 环境", () => {
    expect(isTauri()).toBe(false);
  });

  it("内核版本降级为占位", async () => {
    expect(await coreVersion()).toBe("browser-dev");
  });

  it("列出五档出厂 profile（与 fixtures 对齐）", async () => {
    expect(await profilesList()).toEqual([
      "archive",
      "default",
      "mail",
      "screenshot",
      "web",
    ]);
  });

  it("各档降级字段与 fixtures 一致（web=level 5+webp，mail=targetBytes）", async () => {
    const web = await profilesShow("web");
    expect(web.name).toBe("web");
    expect(web.quality.level).toBe(5);
    expect(web.format.output).toBe("webp");

    const mail = await profilesShow("mail");
    expect(mail.quality.kind).toBe("targetBytes");
    expect(mail.quality.targetBytes).toBe(500_000);
    expect(mail.resize.maxWidth).toBe(1920);

    const archive = await profilesShow("archive");
    expect(archive.quality.kind).toBe("lossless");
    expect(archive.format.output).toBe("jxl");
  });

  it("compress 降级返回完整报告结构（供 UI 演示）", async () => {
    const [r] = await compress(["/tmp/photo.jpg"], "default");
    expect(r.input).toBe("/tmp/photo.jpg");
    expect(r.output).toBe("/tmp/photo-default.jpg");
    expect(r.status).toBe("done");
    expect(r.savedPct).toBeLessThan(0);
    expect(r.engine).toBe("mozjpeg");
  });
});
