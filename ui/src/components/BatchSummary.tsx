import { FolderOpen, Sparkles } from "lucide-react";
import type { Job } from "../lib/batches";
import { summary, outputGroups } from "../lib/batches";
import { formatBytes } from "../lib/format";
import { reveal } from "../lib/bridge";
import { toast } from "sonner";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";

export function BatchSummary({
  jobs,
  busy,
  pending,
  disabled,
  onStart,
  outputHint,
}: {
  jobs: Job[];
  busy: boolean;
  pending: number;
  disabled: boolean;
  onStart: () => void;
  outputHint: string;
}) {
  const s = summary(jobs);
  const groups = outputGroups(jobs);
  const open = (path: string) => {
    void reveal(path).catch((e) => toast.error(String(e)));
  };
  const resultButton = (
    <Button variant="outline" disabled={!groups.length}>
      <FolderOpen />
      查看结果
    </Button>
  );
  return (
    <footer className="batch-footer">
      <div className="batch-copy" role="status" aria-live="polite">
        <div className="batch-headline">
          <span>{busy
            ? `已处理 ${s.processed} / ${jobs.length} 张`
            : pending ? `${pending} 张待处理`
            : s.processed ? `完成 ${s.success + s.unchanged} 张` : "添加图片，开始处理"}</span>
          {!busy && !pending && s.failed > 0 && <span className="warning">失败 {s.failed} 张</span>}
          {!busy && !pending && s.unchanged > 0 && <span>未减小 {s.unchanged} 张</span>}
          {!busy && !pending && s.input > 0 && (
            <span className={s.saved < 0 ? "warning" : "batch-saving"}>
              {s.saved >= 0 ? "节省 " : "增加 "}{formatBytes(Math.abs(s.saved))}
              {`（${Math.abs(s.percent ?? 0).toFixed(1)}%）`}
            </span>
          )}
        </div>
        <p className="hint">{busy ? "正在处理原图，可继续添加图片" : outputHint}</p>
      </div>
      <div className="footer-actions">
        {groups.length > 1 ? (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>{resultButton}</DropdownMenuTrigger>
            <DropdownMenuContent>
              {groups.map((g) => (
                <DropdownMenuItem key={g.dir} onSelect={() => open(g.paths[0])}>
                  {g.dir} · {g.paths.length} 个产物
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        ) : (
          <Button
            variant="outline"
            disabled={!groups.length}
            onClick={() => open(groups[0].paths[0])}
          >
            <FolderOpen />
            查看结果
          </Button>
        )}
        <Button
          className="start-button"
          disabled={disabled || busy}
          onClick={onStart}
        >
          <Sparkles />
          {busy
            ? `处理中 · ${s.processed}/${jobs.length}`
            : pending
              ? `开始处理 ${pending} 张`
              : jobs.length
                ? `重新处理 ${jobs.length} 张`
                : "开始处理"}
        </Button>
      </div>
    </footer>
  );
}
