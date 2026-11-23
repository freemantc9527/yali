import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ChevronLeft,
  ChevronRight,
  Maximize,
  Square,
  Columns2,
  SquareSplitHorizontal,
} from "lucide-react";
import {
  displayImage,
  inspectImages,
  isTauri,
  ssim,
  ssimAvailable,
  type ImageInfo,
  type PreviewImage,
} from "../lib/bridge";
import type { Job } from "../lib/batches";
import { fileName } from "./TaskList";
import { Button } from "./ui/button";
import { IconButton } from "./Controls";

export const clampPos = (value: number) => Math.min(100, Math.max(0, value));
type Dimensions = { width: number; height: number };
export function fitImage(
  image: Dimensions,
  viewport: Dimensions,
  originalSize = false,
) {
  const scale = originalSize
    ? 1
    : Math.max(0, Math.min(viewport.width / image.width, viewport.height / image.height, 1));
  return {
    width: Math.max(1, image.width * scale),
    height: Math.max(1, image.height * scale),
  };
}
function useViewport() {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) =>
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height }),
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return { ref, size };
}
const dimensions = (info?: ImageInfo) => ({
  width: info?.width || 1200,
  height: info?.height || 800,
});
const imageLabel = (info: ImageInfo | undefined, index: number) =>
  `${index === 0 ? "原图" : "处理后"}${info?.width && info?.height ? ` · ${info.width} × ${info.height}` : ""}`;
function ImagePane({ url, info, index, originalSize, onError }: {
  url: string;
  info: ImageInfo;
  index: number;
  originalSize: boolean;
  onError: () => void;
}) {
  const { ref, size } = useViewport();
  return (
    <figure>
      <figcaption>{imageLabel(info, index)}</figcaption>
      <div className="compare-pane" ref={ref}>
        <img
          src={url}
          alt={index === 0 ? "原图" : "处理后"}
          onError={onError}
          style={fitImage(dimensions(info), size, originalSize)}
        />
      </div>
    </figure>
  );
}
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
  const [layout, setLayout] = useState<"split" | "side" | null>(null);
  const [urls, setUrls] = useState<string[]>([]);
  const [infos, setInfos] = useState<ImageInfo[]>([]);
  const [error, setError] = useState("");
  const [score, setScore] = useState<string | null>("计算中…");
  const { ref: viewport, size } = useViewport();
  const report = job.report!;
  const index = jobs.findIndex((j) => j.jobId === job.jobId);
  const move = (delta: number) => {
    const next = jobs[index + delta];
    if (next) onSelect(next.jobId);
  };
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
    setLayout(null);
    setInfos([]);
  }, [job.jobId]);
  useEffect(() => {
    let alive = true;
    let acquired: PreviewImage[] = [];
    setUrls([]);
    setError("");
    setScore(report.output ? "计算中…" : "仅查看原图");
    const paths = [report.input, ...(report.output ? [report.output] : [])];
    void (async () => {
      const info = await inspectImages(paths);
      if (!alive) return;
      setInfos(info);
      const result = await Promise.allSettled(paths.map(displayImage));
      acquired = result.flatMap((item) => item.status === "fulfilled" ? [item.value] : []);
      const failed = result.find((item) => item.status === "rejected");
      if (!alive || failed) {
        acquired.forEach((image) => image.release());
        if (failed?.status === "rejected") throw failed.reason;
        return;
      }
      setUrls(acquired.map((image) => image.src));
      const mismatch =
        info.length > 1 &&
        info[0].width &&
        info[1].width &&
        (info[0].width !== info[1].width || info[0].height !== info[1].height);
      if (mismatch) {
        setScore("尺寸不同");
        return;
      }
      if (report.output) {
        const available = await ssimAvailable();
        if (!alive) return;
        if (!available) {
          setScore(null);
          return;
        }
        void ssim(report.input, report.output)
          .then((value) => {
            if (alive)
              setScore(
                `SSIM ${value.toFixed(4)}${isTauri() ? "" : "（演示）"}`,
              );
          })
          .catch(() => {
            if (alive) setScore(null);
          });
      }
    })().catch((e) => {
      if (alive) {
        setError(String(e));
        setScore("预览不可用");
      }
    });
    return () => {
      alive = false;
      acquired.forEach((image) => image.release());
    };
  }, [job.jobId]);
  const different =
    infos.length > 1 &&
    infos[0].width &&
    infos[1].width &&
    (infos[0].width !== infos[1].width || infos[0].height !== infos[1].height);
  const mode = layout ?? (different ? "side" : "split");
  const original = dimensions(infos[0]);
  const canvas = zoom
    ? {
        width: Math.max(...infos.map((info) => dimensions(info).width), original.width),
        height: Math.max(...infos.map((info) => dimensions(info).height), original.height),
      }
    : original;
  const stageSize = fitImage(
    canvas,
    { width: size.width - 32, height: size.height - 32 },
    zoom,
  );
  const previewError = () => setError("图片加载失败，请返回工作台后重试");
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
        <div className="compare-status hint">
          {score && <span>{score}</span>}
          {different && mode === "split" && (
            <span>{zoom ? "按各自原始像素显示" : "按画面对齐显示，保持比例"}</span>
          )}
        </div>
        {report.output && (
          <div className="compare-tool-group" role="group" aria-label="对比方式">
            <Button
              size="sm"
              variant={mode === "split" ? "secondary" : "ghost"}
              aria-pressed={mode === "split"}
              onClick={() => setLayout("split")}
            >
              <SquareSplitHorizontal />叠加
            </Button>
            <Button
              size="sm"
              variant={mode === "side" ? "secondary" : "ghost"}
              aria-pressed={mode === "side"}
              onClick={() => setLayout("side")}
            >
              <Columns2 />并排
            </Button>
          </div>
        )}
        <div className="compare-tool-group" role="group" aria-label="显示尺寸">
          <Button
            size="sm"
            variant={!zoom ? "secondary" : "ghost"}
            onClick={() => setZoom(false)}
            aria-pressed={!zoom}
          >
            <Maximize />
            适应窗口
          </Button>
          <Button
            size="sm"
            variant={zoom ? "secondary" : "ghost"}
            onClick={() => setZoom(true)}
            aria-pressed={zoom}
            title="按图片实际像素显示，超出窗口可滚动查看"
          >
            <Square />
            原始尺寸
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
        ) : mode === "side" && urls.length > 1 ? (
          <div className="side-by-side">
            {urls.map((url, i) => (
              <ImagePane
                key={i}
                url={url}
                info={infos[i]}
                index={i}
                originalSize={zoom}
                onError={previewError}
              />
            ))}
          </div>
        ) : (
          <div
            className="compare-stage"
            style={stageSize}
          >
            <div className="compare-layer">
              <img
                src={urls[0]}
                alt="原图"
                onError={previewError}
                style={zoom ? original : undefined}
              />
            </div>
            {urls[1] && (
              <>
                <div className="compare-layer" style={{ clipPath: `inset(0 0 0 ${pos}%)` }}>
                  <img
                    src={urls[1]}
                    alt="处理后"
                    onError={previewError}
                    style={zoom ? dimensions(infos[1]) : undefined}
                  />
                </div>
                <button
                  className="compare-divider"
                  aria-label="拖动对比分割线"
                  role="slider"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(pos)}
                  aria-valuetext={`原图 ${Math.round(pos)}%，处理后 ${100 - Math.round(pos)}%`}
                  style={{ left: `${pos}%` }}
                  onPointerDown={(e) => {
                    e.currentTarget.setPointerCapture(e.pointerId);
                    pointer(e);
                  }}
                  onPointerMove={(e) => {
                    if (e.currentTarget.hasPointerCapture(e.pointerId))
                      pointer(e);
                  }}
                  onPointerUp={(e) => {
                    if (e.currentTarget.hasPointerCapture(e.pointerId))
                      e.currentTarget.releasePointerCapture(e.pointerId);
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
                <span className="image-label before-label">{imageLabel(infos[0], 0)}</span>
                <span className="image-label after-label">{imageLabel(infos[1], 1)}</span>
              </>
            )}
          </div>
        )}
      </div>
    </section>
  );
}
