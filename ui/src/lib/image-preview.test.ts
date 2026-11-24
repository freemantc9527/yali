import { afterEach, describe, expect, it, vi } from "vitest";
const platform = vi.hoisted(() => ({ windows: false }));
vi.mock("./platform", () => ({ get isWindows() { return platform.windows; } }));
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (path: string) => `asset:${path}` }));
import { displayImage, imagePreview } from "./bridge";

afterEach(() => { platform.windows = false; vi.unstubAllGlobals(); vi.resetAllMocks(); });
describe("真实文件预览", () => {
  const native = () => vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  it("可解码图片直接读原件，不调用预览转换", async () => {
    native();
    invoke.mockResolvedValue("/original.png");
    vi.stubGlobal("Image", class { src = ""; decode = () => Promise.resolve(); });
    const result = await displayImage("/original.png");
    expect(result.src).toMatch(/^asset:\/original.png\?v=/);
    expect(invoke.mock.calls).toEqual([["image_source", { path: "/original.png" }]]);
    result.release();
    expect(invoke).toHaveBeenCalledTimes(1);
  });
  it("解码失败才生成原尺寸 PNG，释放租约幂等", async () => {
    native();
    invoke.mockImplementation(async (cmd: string) => cmd === "image_source" ? "/photo.jxl" : { path: "/.yali/preview.png", leaseId: "42" });
    vi.stubGlobal("Image", class { src = ""; decode = () => Promise.reject(new Error("decode")); });
    const result = await displayImage("/photo.jxl");
    expect(invoke).toHaveBeenCalledWith("image_preview", { path: "/photo.jxl", maxEdge: 0 });
    expect(result.src).toBe("asset:/.yali/preview.png");
    result.release(); result.release();
    expect(invoke.mock.calls.filter(([cmd]) => cmd === "release_preview")).toEqual([["release_preview", { leaseId: "42" }]]);
  });
  it("列表仍使用 192px 缩略图并持有可释放的租约", async () => {
    native();
    invoke.mockResolvedValue({ path: "/thumb.png", leaseId: "thumb" });
    const result = await imagePreview("/photo.png", 192);
    expect(invoke).toHaveBeenCalledWith("image_preview", { path: "/photo.png", maxEdge: 192 });
    expect(invoke).not.toHaveBeenCalledWith("release_preview", expect.anything());
    result.release();
    expect(invoke).toHaveBeenCalledWith("release_preview", { leaseId: "thumb" });
  });
  it("Windows 的 TIFF/BMP 使用 PNG 显示副本并保留中文、UNC 和扩展路径", async () => {
    native();
    platform.windows = true;
    invoke.mockResolvedValue({ path: "C:\\用户\\缓存 [图].png", leaseId: "win" });
    for (const path of ["C:\\用户\\图片 & [1].TIFF", "\\\\server\\share\\图片.bmp", "\\\\?\\C:\\用户\\图.tif"]) {
      const result = await displayImage(path);
      expect(invoke).toHaveBeenCalledWith("image_preview", { path, maxEdge: 0 });
      expect(result.src).toBe("asset:C:\\用户\\缓存 [图].png");
      result.release();
    }
    expect(invoke).not.toHaveBeenCalledWith("image_source", expect.anything());
  });

});
