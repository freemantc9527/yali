import { describe, expect, it } from "vitest";
import {
  compress,
  coreVersion,
  isTauri,
  onOpenFiles,
  profilesList,
  profilesShow,
  settingsSave,
  settingsShow,
  takePendingFiles,
} from "./bridge";

// node 环境没有 Tauri 注入，全部走浏览器降级分支——这正是 vitest 需要的可测性。
describe("bridge 浏览器降级", () => {
  it("识别非 Tauri 环境", () => {
    expect(isTauri()).toBe(false);
  });

  it("内核版本降级为占位", async () => {
    expect(await coreVersion()).toBe("browser-dev");
  });

  it("仅列出四档出厂 profile（与 fixtures 对齐）", async () => {
    expect(await profilesList()).toEqual([
      "archive",
      "default",
      "jpg",
      "thumbnail",
    ]);
  });

  it("缩小尺寸限制最长边 1024，归档仍为无损 JXL", async () => {
    const thumbnail = await profilesShow("thumbnail");
    expect(thumbnail.resize.maxWidth).toBe(1024);
    expect(thumbnail.resize.maxHeight).toBe(1024);
    expect(thumbnail.resize.keepRatio).toBe(true);
    expect(thumbnail.resize.mode).toBe("shrinkOnly");

    const archive = await profilesShow("archive");
    expect(archive.quality.kind).toBe("lossless");
    expect(archive.format.output).toBe("jxl");
  });

  it("系统投递在浏览器降级为空且可退订（T-013）", async () => {
    const off = onOpenFiles(() => {});
    expect(await takePendingFiles()).toEqual([]);
    off();
  });

  it("字号只接受六个固定档，失败保存不改变已有设置", async () => {
    const original = await settingsShow();
    for (const percent of [80, 100, 120, 150, 180, 200]) {
      await settingsSave({ ...original, fontScalePercent: percent });
      expect((await settingsShow()).fontScalePercent).toBe(percent);
    }
    for (const percent of [79, 81, 110, 175, 201]) {
      await expect(settingsSave({ ...original, fontScalePercent: percent })).rejects.toThrow("80 / 100 / 120 / 150 / 180 / 200%");
      expect((await settingsShow()).fontScalePercent).toBe(200);
    }
    await settingsSave(original);
  });

  it("compress 降级返回完整报告结构（供 UI 演示）", async () => {
    const [r] = await compress(["/tmp/photo.jpg"], "default");
    expect(r.input).toBe("/tmp/photo.jpg");
    expect(r.output).toBe("/tmp/photo-default.jpg");
    expect(r.status).toBe("done");
    expect(r.savedPct).toBeGreaterThan(0);
    expect(r.engine).toBe("mozjpeg");
  });
});
