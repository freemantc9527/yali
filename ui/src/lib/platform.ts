/** 与桌面运行时平台一致，浏览器预览沿用当前系统。 */
export function windowsUserAgent(userAgent: string): boolean {
  return /Windows/i.test(userAgent);
}
export const isWindows = typeof navigator !== "undefined" && windowsUserAgent(navigator.userAgent);
