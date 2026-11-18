import { ImagePlus } from "lucide-react";
import { Button } from "./ui/button";
import mascot from "../../../src-tauri/icons/mascot.png";

export function DropZone({
  empty,
  dragging,
  reading,
  onAdd,
}: {
  empty: boolean;
  dragging: boolean;
  reading: boolean;
  onAdd: () => void;
}) {
  if (!empty)
    return (
      <button
        className={`drop-continue ${dragging ? "dragging" : ""}`}
        onClick={onAdd}
      >
        <ImagePlus />
        <span>{reading ? "正在读取图片…" : "继续拖入图片，或点击添加"}</span>
      </button>
    );
  return (
    <div className={`empty-state ${dragging ? "dragging" : ""}`}>
      <img src={mascot} alt="鸭梨抱梨鸭" />
      <h2>图片放进来，轻一点带走</h2>
      <p>拖入图片，或选择一批开始处理</p>
      <Button variant="outline" onClick={onAdd}>
        <ImagePlus />
        添加图片
      </Button>
      <p className="hint">
        {reading ? "正在读取图片…" : "原图保留，处理前可以调整参数"}
      </p>
    </div>
  );
}
