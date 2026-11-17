import { useEffect, useState } from "react";
import { coreVersion } from "./lib/bridge";
import { formatBytes } from "./lib/format";

/**
 * T-005 最小脚手架：只验证「Vite 构建 → Tauri 壳加载 → 调用内核 command」通路。
 * 真正的参数控制台（DESIGN.md §5）按任务卡逐块填充，此处不做样式迭代。
 */
export default function App() {
  const [version, setVersion] = useState<string>("…");

  useEffect(() => {
    coreVersion().then(setVersion);
  }, []);

  return (
    <main className="shell">
      <h1>鸭梨 Yali</h1>
      <p className="tagline">让图片没有压力</p>
      <dl className="probe">
        <dt>内核版本</dt>
        <dd data-testid="core-version">{version}</dd>
        <dt>体积换算自检</dt>
        <dd>{formatBytes(1_234_567)}</dd>
      </dl>
      <p className="note">UI 脚手架（T-005）：参数控制台尚未接线。</p>
    </main>
  );
}
