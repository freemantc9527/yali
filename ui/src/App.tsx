import { useCallback, useEffect, useRef, useState } from "react";
import { DropZone } from "./components/DropZone";
import { ProfileForm } from "./components/ProfileForm";
import { ProfileManager } from "./components/ProfileManager";
import { Viewer } from "./components/Viewer";
import { TaskList } from "./components/TaskList";
import { applyProgress, reconcile, type TaskRow } from "./lib/tasks";
import {
  compress,
  coreVersion,
  profilesList,
  onOpenFiles,
  onProgress,
  profilesSave,
  profilesShow,
  takePendingFiles,
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
  const [viewing, setViewing] = useState<TaskRow | null>(null);
  const [busy, setBusy] = useState(false);

  // 逐文件进度事件（结果权威仍是 command 返回值，事件丢失由 reconcile 兜底）
  useEffect(() => onProgress((event) => setRows((prev) => applyProgress(prev, event))), []);

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

  const handleSave = useCallback(async (next: Profile) => {
    await profilesSave(next);
    const list = await profilesList();
    setNames(list);
    setProfile(next); // 保存后以草稿为准；切档时会从磁盘重读
  }, []);

  const handlePaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0 || busy) return;
      setBusy(true);
      const base = rows.length; // 新批次排在已有任务之前，index 全局唯一
      setRows((prev) => [
        ...paths.map((path, i) => ({ path, index: base + i, running: true })),
        ...prev,
      ]);
      try {
        const reports = await compress(paths, selected);
        setRows((prev) => reconcile(prev, reports));
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
    [busy, rows.length, selected],
  );

  // 系统投递（打开方式 / Dock 拖入）走同一个入队口；用 ref 避免每次任务变化都重订阅
  const handlePathsRef = useRef(handlePaths);
  useEffect(() => {
    handlePathsRef.current = handlePaths;
  }, [handlePaths]);

  useEffect(() => {
    const drain = () => {
      void takePendingFiles().then((paths) => {
        if (paths.length > 0) void handlePathsRef.current(paths);
      });
    };
    // 先订阅再取缓冲：两个动作之间投递的文件也不会丢
    const unlisten = onOpenFiles(drain);
    drain();
    return unlisten;
  }, []);

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
        onSave={handleSave}
      />
      <ProfileManager profile={profile} onSave={handleSave} />
      <TaskList rows={rows} onOpen={(row) => setViewing(row)} />
      {viewing?.report && <Viewer report={viewing.report} onClose={() => setViewing(null)} />}
    </main>
  );
}
