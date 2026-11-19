import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ChevronLeft,
  ChevronRight,
  Maximize,
  ZoomIn,
} from "lucide-react";
import {
  fileSrc,
  imagePreview,
  inspectImages,
  isTauri,
  ssim,
  type ImageInfo,
} from "../lib/bridge";
import type { Job } from "../lib/batches";
import { fileName } from "./TaskList";
import { Button } from "./ui/button";
import { IconButton } from "./Controls";

export const clampPos = (value: number) => Math.min(100, Math.max(0, value));
const controlTarget = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  !!target.closest(
    'input,textarea,[role="slider"],[role="combobox"],[role="dialog"],[role="alertdialog"]',
  );
export function Viewer({
  job,
  jobs,
  onSelect,
  onClose,
}: {
  job: Job;
  jobs: Job[];
  onSelect: (id: string) => void;
  onClose: () => void;
}) {
  const [pos, setPos] = useState(50);
  const [zoom, setZoom] = useState(false);
  const [urls, setUrls] = useState<string[]>([]);
  const [infos, setInfos] = useState<ImageInfo[]>([]);
  const [error, setError] = useState("");
  const [score, setScore] = useState("计算中…");
  const viewport = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 600, height: 400 });
  const report = job.report!;
  const index = jobs.findIndex((j) => j.jobId === job.jobId);
  const move = (delta: number) => {
    const next = jobs[index + delta];
    if (next) onSelect(next.jobId);
  };
  useEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      }),
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (controlTarget(e.target)) return;
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft" && e.altKey) {
        e.preventDefault();
        move(-1);
      }
      if (e.key === "ArrowRight" && e.altKey) {
        e.preventDefault();
        move(1);
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [job.jobId, jobs]);
  useEffect(() => {
    setPos(50);
    setZoom(false);
  }, [job.jobId]);
  useEffect(() => {
    let alive = true;
    setUrls([]);
    setInfos([]);
    setError("");
    setScore(report.output ? "计算中…" : "仅查看原图");
    const paths = [report.input, ...(report.output ? [report.output] : [])];
    void (async () => {
      const info = await inspectImages(paths);
      if (!alive) return;
      setInfos(info);
      const result = await Promise.all(
        paths.map((path) =>
          zoom && /\.(png|jpe?g|webp|gif|bmp|avif)$/i.test(path) && isTauri()
            ? Promise.resolve(fileSrc(path)!)
            : imagePreview(path, zoom ? 0 : 1600),
        ),
      );
      if (!alive) return;
      setUrls(result);
      const mismatch =
        info.length > 1 &&
        info[0].width &&
        info[1].width &&
        (info[0].width !== info[1].width || info[0].height !== info[1].height);
      if (mismatch) {
        setScore("尺寸不同，SSIM 不适用");
        return;
      }
      if (report.output)
        void ssim(report.input, report.output)
          .then((value) => {
            if (alive)
              setScore(
                `SSIM ${value.toFixed(4)}${isTauri() ? "" : "（演示）"}`,
              );
          })
          .catch((e) => {
            if (alive) setScore(`SSIM 不可用：${String(e)}`);
          });
    })().catch((e) => {
      if (alive) {
        setError(String(e));
        setScore("预览不可用");
      }
    });
    return () => {
      alive = false;
    };
  }, [job.jobId, zoom]);
  const different =
    infos.length > 1 &&
    infos[0].width &&
    infos[1].width &&
    (infos[0].width !== infos[1].width || infos[0].height !== infos[1].height);
  const width = infos[0]?.width ?? 1200,
    height = infos[0]?.height ?? 800;
  const scale = zoom
    ? 1
    : Math.min((size.width - 32) / width, (size.height - 32) / height, 1);
  const pointer = (e: React.PointerEvent<HTMLButtonElement>) => {
    const rect = e.currentTarget.parentElement!.getBoundingClientRect();
    setPos(clampPos(((e.clientX - rect.left) / rect.width) * 100));
  };
  return (
    <section className="aux-view compare-view" data-testid="viewer">
      <header className="aux-toolbar">
        <Button variant="ghost" onClick={onClose}>
          <ArrowLeft />
          返回工作台
        </Button>
        <h1 title={job.path}>{fileName(job.path)}</h1>
        <div className="compare-navigation">
          <IconButton
            label="上一张"
            disabled={index <= 0}
            onClick={() => move(-1)}
          >
            <ChevronLeft />
          </IconButton>
          <span className="hint">
            {index + 1}/{jobs.length}
          </span>
          <IconButton
            label="下一张"
            disabled={index >= jobs.length - 1}
            onClick={() => move(1)}
          >
            <ChevronRight />
          </IconButton>
        </div>
      </header>
      <div className="compare-tools">
        <span className="hint">{score}</span>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant={!zoom ? "secondary" : "ghost"}
            onClick={() => setZoom(false)}
          >
            <Maximize />
            适应窗口
          </Button>
          <Button
            size="sm"
            variant={zoom ? "secondary" : "ghost"}
            onClick={() => setZoom(true)}
          >
            <ZoomIn />
            100%
          </Button>
        </div>
      </div>
      <div className="compare-scroll" ref={viewport}>
        {error ? (
          <div className="preview-message">
            <p>此图片暂不可预览：{error}</p>
            <p className="hint">处理结果不受影响，可返回工作台定位真实产物。</p>
          </div>
        ) : !urls.length ? (
          <p className="loading">正在生成本地预览…</p>
        ) : different ? (
          <div className={`side-by-side ${zoom ? "actual-size" : ""}`}>
            {urls.map((url, i) => (
              <figure key={i}>
                <figcaption>
                  {i === 0 ? "原图" : "处理后"} · {infos[i].width} ×{" "}
                  {infos[i].height}
                </figcaption>
                <img
                  alt={i === 0 ? "原图" : "处理后"}
                  src={url}
                  style={
                    zoom
                      ? { maxWidth: "none", width: infos[i].width ?? undefined }
                      : undefined
                  }
                />
              </figure>
            ))}
          </div>
        ) : (
          <div
            className="compare-stage"
            style={{
              width: Math.max(1, width * scale),
              height: Math.max(1, height * scale),
            }}
          >
            <img src={urls[0]} alt="原图" />
            {urls[1] && (
              <>
                <img
                  src={urls[1]}
                  alt="处理后"
                  style={{ clipPath: `inset(0 0 0 ${pos}%)` }}
                />
                <button
                  className="compare-divider"
                  aria-label="拖动对比分割线"
                  style={{ left: `${pos}%` }}
                  onPointerDown={(e) => {
                    e.currentTarget.setPointerCapture(e.pointerId);
                    pointer(e);
                  }}
                  onPointerMove={(e) => {
                    if (e.currentTarget.hasPointerCapture(e.pointerId))
                      pointer(e);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
                      e.preventDefault();
                      setPos(clampPos(pos + (e.key === "ArrowLeft" ? -1 : 1)));
                    }
                  }}
                >
                  <ArrowLeft />
                  <ArrowRight />
                </button>
                <span className="image-label before-label">原图</span>
                <span className="image-label after-label">处理后</span>
              </>
            )}
          </div>
        )}
      </div>
    </section>
  );
}
