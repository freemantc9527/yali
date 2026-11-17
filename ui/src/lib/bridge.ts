/** 内核 command 的唯一调用入口：浏览器里（纯 vite dev）降级为占位，便于脱离 Tauri 调试 UI。 */
export async function coreVersion(): Promise<string> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return "browser-dev";
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("core_version");
}
