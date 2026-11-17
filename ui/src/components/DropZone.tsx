import { useEffect, useState } from "react";
import { onFileDrop } from "../lib/bridge";

interface Props {
  onPaths: (paths: string[]) => void;
  busy?: boolean;
}

/** 拖入文件入口：Tauri 原生 drag-drop 给真实路径；浏览器降级只拿到文件名（演示用）。 */
export function DropZone({ onPaths, busy = false }: Props) {
  const [hovering, setHovering] = useState(false);

  useEffect(() => onFileDrop(onPaths), [onPaths]);

  return (
    <div
      data-testid="drop-zone"
      className={hovering ? "dropzone hover" : "dropzone"}
      onDragEnter={() => setHovering(true)}
      onDragLeave={() => setHovering(false)}
      onDrop={() => setHovering(false)}
    >
      <strong>{busy ? "压缩中…" : "把图片拖进来"}</strong>
      <span className="muted">右键「压一下」和 GUI 走的是同一份 Profile</span>
    </div>
  );
}
