import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// 与 src-tauri/tauri.conf.json 的 devUrl 对齐：端口固定，避免 Tauri 连不上 dev server。
// 构建产物固定输出到 ui/dist（tauri.conf.json 的 frontendDist 指向这里）。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "safari17",
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
