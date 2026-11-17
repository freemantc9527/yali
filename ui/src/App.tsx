import { useCallback, useEffect, useState } from "react";
import { DropZone } from "./components/DropZone";
import { ProfileForm } from "./components/ProfileForm";
import { TaskList, type TaskRow } from "./components/TaskList";
import {
  compress,
  coreVersion,
  profilesList,
  profilesShow,
  type Profile,
} from "./lib/bridge";

/**
 * 参数控制台（T-005 最小可用版）：Profile 读取 + 拖入 + 调内核 + 任务列表。
 * 视觉迭代不在本卡范围（禁止无卡样式迭代）；压缩逻辑全部在内核，经 bridge 走 invoke。
 */
export default function App() {
  const [version, setVersion] = useState("…");
  const [names, setNames] = useState<string[]>([]);
  const [selected, setSelected] = useState("default");
  const [profile, setProfile] = useState<Profile | null>(null);
  const [rows, setRows] = useState<TaskRow[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    coreVersion().then(setVersion);
    profilesList()
      .then((list) => {
        setNames(list);
        if (list.length > 0 && !list.includes(selected)) setSelected(list[0]);
      })
      .catch(() => setNames([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selected) return;
    profilesShow(selected)
      .then(setProfile)
      .catch(() => setProfile(null));
  }, [selected]);

  const handlePaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0 || busy) return;
      setBusy(true);
      setRows((prev) => [
        ...paths.map((path) => ({ path, running: true })),
        ...prev,
      ]);
      try {
        const reports = await compress(paths, selected);
        const byInput = new Map(reports.map((r) => [r.input, r]));
        setRows((prev) =>
          prev.map((row) => {
            const report = byInput.get(row.path);
            if (!report) return row;
            // 相对路径与绝对路径都要能对上
            const key = [...byInput.keys()].find((k) => k.endsWith(row.path) || row.path.endsWith(k));
            return { ...row, running: false, report: key ? byInput.get(key) : report };
          }),
        );
      } catch (e) {
        setRows((prev) =>
          prev.map((row) =>
            row.running ? { ...row, running: false, error: String(e) } : row,
          ),
        );
      } finally {
        setBusy(false);
      }
    },
    [busy, selected],
  );

  return (
    <main className="shell">
      <header className="hero">
        <div>
          <h1>鸭梨 Yali</h1>
          <p className="tagline">让图片没有压力</p>
        </div>
        <span className="version" data-testid="core-version">
          v{version}
        </span>
      </header>

      <DropZone onPaths={handlePaths} busy={busy} />
      <ProfileForm
        profile={profile}
        names={names}
        selected={selected}
        onSelect={setSelected}
      />
      <TaskList rows={rows} />
    </main>
  );
}
