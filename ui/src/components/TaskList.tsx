import { useEffect, useRef, useState } from "react";
import {
  Check,
  CircleAlert,
  Image as ImageIcon,
  LoaderCircle,
  RotateCcw,
  X,
} from "lucide-react";
import type { Job, Status } from "../lib/batches";
import { imagePreview, type ImageInfo } from "../lib/bridge";
import { formatBytes, savings } from "../lib/format";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { IconButton } from "./Controls";

export const statusLabels: Record<Status, string> = {
  pending: "待处理",
  queued: "排队中",
  processing: "处理中",
  done: "已完成",
  larger_output: "未减小",
  failed: "失败",
};
export function fileName(path: string) {
  return path.startsWith("browser:")
    ? path.slice(8).split(":")[0]
    : (path.split("/").pop() ?? path);
}
export function Thumbnail({
  path,
  format,
}: {
  path: string;
  format?: string | null;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [url, setUrl] = useState<string>();
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let alive = true;
    let requested = false;
    setUrl(undefined);
    setFailed(false);
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting) && !requested) {
          requested = true;
          void imagePreview(path, 192)
            .then((url) => {
              if (alive) setUrl(url);
            })
            .catch(() => {
              if (alive) setFailed(true);
            });
          observer.disconnect();
        }
      },
      { rootMargin: "160px" },
    );
    if (host.current) observer.observe(host.current);
    return () => {
      alive = false;
      observer.disconnect();
    };
  }, [path]);
  return (
    <div
      ref={host}
      className="thumbnail"
      title={failed ? "此格式暂不可预览，仍可处理或定位结果" : undefined}
    >
      {url ? (
        <img
          src={url}
          alt=""
          loading="lazy"
          onError={() => {
            setUrl(undefined);
            setFailed(true);
          }}
        />
      ) : (
        <>
          <ImageIcon />
          <span>{format?.toUpperCase() ?? "图片"}</span>
        </>
      )}
    </div>
  );
}
export function TaskRowView({
  job,
  info,
  active,
  selected,
  duplicate,
  onSelect,
  onCompare,
  onRetry,
  onRemove,
}: {
  job: Job;
  info?: ImageInfo;
  active: string | null;
  selected: boolean;
  duplicate: boolean;
  onSelect: () => void;
  onCompare: () => void;
  onRetry: () => void;
  onRemove: () => void;
}) {
  const name = fileName(job.path);
  const dot = name.lastIndexOf(".");
  const canCompare = !!job.report;
  const increased =
    job.report?.outputBytes !== null &&
    job.report?.outputBytes !== undefined &&
    job.report.outputBytes > job.report.inputBytes;
  const status =
    job.status === "done" && increased
      ? "已转换，体积增加"
      : job.status === "larger_output"
        ? "未减小，已保留原图"
        : statusLabels[job.status];
  return (
    <li
      className={`task-row ${selected ? "selected" : ""}`}
      data-testid="task-row"
      data-job-id={job.jobId}
      tabIndex={0}
      aria-label={`${name}，${status}`}
      onFocus={onSelect}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) return;
        if ((e.key === " " || e.key === "Enter") && canCompare) {
          e.preventDefault();
          onCompare();
        }
      }}
    >
      <Thumbnail path={job.path} format={info?.format} />
      <div className="file-info">
        <div className="file-name" title={job.path}>
          <span>{dot > 0 ? name.slice(0, dot) : name}</span>
          {dot > 0 && <span className="file-extension">{name.slice(dot)}</span>}
        </div>
        <p className="hint">
          {info?.width && info.height
            ? `${info.width} × ${info.height} · `
            : ""}
          {info?.format?.toUpperCase() ?? "尺寸未知"}
          {job.attempt > 1 ? ` · 重试 ${job.attempt - 1}` : ""}
        </p>
        {duplicate && (
          <p className="hint path-text">
            {job.path.slice(0, job.path.lastIndexOf("/"))}
          </p>
        )}
        {job.source === "manual" && job.status === "pending" && active && (
          <p className="hint">等待下一批</p>
        )}
        {job.error && (
          <details className="task-error">
            <summary>处理失败：{job.error.split("\n")[0].slice(0, 60)}</summary>
            <p>{job.error}</p>
          </details>
        )}
      </div>
      <div className="file-sizes">
        <span>
          {job.report
            ? formatBytes(job.report.inputBytes)
            : info?.bytes !== undefined && info.bytes !== null
              ? formatBytes(info.bytes)
              : "大小未知"}
        </span>
        <span className="size-arrow">→</span>
        <span>
          {job.status === "larger_output"
            ? "保留原图"
            : job.report?.outputBytes !== undefined &&
                job.report.outputBytes !== null
              ? formatBytes(job.report.outputBytes)
              : job.status === "failed"
                ? "无结果"
                : "待处理"}
        </span>
      </div>
      <div className={`file-saving ${increased ? "warning" : ""}`}>
        {job.status === "larger_output"
          ? "0%"
          : job.report?.outputBytes !== undefined &&
              job.report.outputBytes !== null
            ? savings(job.report.inputBytes, job.report.outputBytes)
            : ""}
      </div>
      <div className={`task-state state-${job.status}`} title={status}>
        {job.status === "done" ? (
          <Check />
        ) : job.status === "processing" || job.status === "queued" ? (
          <LoaderCircle className="animate-spin" />
        ) : job.status === "failed" || job.status === "larger_output" ? (
          <CircleAlert />
        ) : null}
        <Badge variant="outline" className="status-badge">
          {status}
        </Badge>
      </div>
      <div className="row-actions">
        {canCompare && (
          <Button variant="outline" size="sm" onClick={onCompare}>
            {job.report?.output ? "对比" : "原图"}
          </Button>
        )}
        {job.status === "failed" && (
          <IconButton
            label={`重试 ${name}`}
            disabled={!!active}
            onClick={onRetry}
          >
            <RotateCcw />
          </IconButton>
        )}
        {(!active || job.batchId !== active) && job.status !== "queued" && (
          <IconButton label={`移除 ${name}`} onClick={onRemove}>
            <X />
          </IconButton>
        )}
      </div>
    </li>
  );
}
export function TaskList({
  jobs,
  infos,
  active,
  selected,
  onSelect,
  onCompare,
  onRetry,
  onRemove,
}: {
  jobs: Job[];
  infos: Map<string, ImageInfo>;
  active: string | null;
  selected: string | null;
  onSelect: (id: string) => void;
  onCompare: (id: string) => void;
  onRetry: (job: Job) => void;
  onRemove: (job: Job) => void;
}) {
  const counts = new Map<string, number>();
  for (const j of jobs)
    counts.set(fileName(j.path), (counts.get(fileName(j.path)) ?? 0) + 1);
  return (
    <ul className="task-list" aria-label="本批图片">
      {jobs.map((job) => (
        <TaskRowView
          key={job.jobId}
          job={job}
          info={infos.get(job.path)}
          active={active}
          selected={selected === job.jobId}
          duplicate={(counts.get(fileName(job.path)) ?? 0) > 1}
          onSelect={() => onSelect(job.jobId)}
          onCompare={() => onCompare(job.jobId)}
          onRetry={() => onRetry(job)}
          onRemove={() => onRemove(job)}
        />
      ))}
    </ul>
  );
}
