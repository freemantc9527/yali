import { afterEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
  vi.resetModules();
});

describe("外部 FFmpeg 检查", () => {
  it("并发检查只启动一次，设置页刷新后对比页使用新结果", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const { ffmpegStatus, ssimAvailable } = await import("./bridge");
    const missing = { available: false, path: null, ssim: false };
    const ready = { available: true, path: "/tools/ffmpeg", ssim: true };
    invoke.mockResolvedValueOnce(missing).mockResolvedValueOnce(ready);

    expect(await Promise.all([ffmpegStatus(), ssimAvailable()])).toEqual([missing, false]);
    expect(invoke.mock.calls).toEqual([["ffmpeg_status", undefined]]);
    expect(await ffmpegStatus(true)).toEqual(ready);
    expect(await ssimAvailable()).toBe(true);
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("检查调用失败后允许重试，无 ssim 滤镜时不启用分数", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const { ffmpegStatus, ssimAvailable } = await import("./bridge");
    invoke.mockRejectedValueOnce(new Error("连接失败"));
    await expect(ffmpegStatus()).rejects.toThrow("连接失败");
    invoke.mockResolvedValue({ available: true, path: "C:\\Tools\\ffmpeg.exe", ssim: false });
    expect(await ssimAvailable()).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("浏览器演示不调用本机探测", async () => {
    const { ffmpegStatus } = await import("./bridge");
    expect(await ffmpegStatus(true)).toEqual({ available: false, path: null, ssim: false });
    expect(invoke).not.toHaveBeenCalled();
  });
});
