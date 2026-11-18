import { CheckCircle2, FolderOpen, Sparkles } from "lucide-react";
import type { Job } from "../lib/batches";
import { summary, outputGroups } from "../lib/batches";
import { formatBytes, savings } from "../lib/format";
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
      <div className="batch-status">
        <CheckCircle2 />
        <div>
          <strong>
            {busy
              ? `已处理 ${s.processed} / ${jobs.length} 张`
              : s.processed
                ? `成功 ${s.success} 张${s.unchanged ? ` · 未减小 ${s.unchanged}` : ""}${s.failed ? ` · 失败 ${s.failed}` : ""}`
                : pending
                  ? `${pending} 张图片待处理`
                  : "让图片没有压力"}
          </strong>
          <p className="hint">
            {busy ? "本批参数已固定，可继续添加图片" : outputHint}
          </p>
        </div>
      </div>
      <div className="batch-numbers">
        {s.input > 0 && (
          <>
            <strong>
              {s.saved >= 0 ? "共省 " : "净增加 "}
              {formatBytes(Math.abs(s.saved))}
            </strong>
            <p className="hint">
              {formatBytes(s.input)} → {formatBytes(s.output)} ·{" "}
              {savings(s.input, s.output)}
              {s.failed ? " · 已处理结果" : ""}
            </p>
          </>
        )}
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
              ? `压一下 · ${pending} 张`
              : jobs.length
                ? "再压一批"
                : "压一下"}
        </Button>
      </div>
    </footer>
  );
}
