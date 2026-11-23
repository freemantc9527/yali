import { useEffect, useState } from "react";
import { isWindows } from "../lib/platform";
import type { Profile } from "../lib/bridge";
import { chooseDirectory } from "../lib/bridge";
import {
  preflightError,
  withFormat,
  withOutput,
  withQuality,
  withResize,
} from "../lib/profile-edit";
import { profileLabel } from "../lib/profile-labels";
import { toast } from "sonner";
import { ChevronDown, FolderOpen, MoreHorizontal, Save } from "lucide-react";
import { Choice, Field } from "./Controls";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { Slider } from "./ui/slider";
import { Switch } from "./ui/switch";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "./ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";

export function NumberField({
  label,
  value,
  onChange,
  scale = 1,
  optional = false,
  allowZero = false,
}: {
  label: string;
  value: number | null;
  onChange: (value: number | null) => void;
  scale?: number;
  optional?: boolean;
  allowZero?: boolean;
}) {
  const [raw, setRaw] = useState(value === null ? "" : String(value / scale));
  useEffect(() => {
    if (!Number.isNaN(value))
      setRaw(value === null ? "" : String(value / scale));
  }, [value, scale]);
  const invalid =
    raw.trim() === ""
      ? !optional
      : !Number.isFinite(Number(raw)) ||
        (allowZero ? Number(raw) < 0 : Number(raw) <= 0) ||
        !Number.isInteger(Number(raw) * scale) ||
        (scale === 1 && !Number.isInteger(Number(raw)));
  return (
    <div>
      <Input
        aria-label={label}
        aria-invalid={invalid}
        inputMode="decimal"
        placeholder={optional ? "不限" : undefined}
        value={raw}
        onChange={(e) => {
          const text = e.target.value;
          setRaw(text);
          const n =
            text.trim() === "" ? (optional ? null : NaN) : Number(text) * scale;
          onChange(n);
        }}
      />
      {invalid && (
        <p className="field-error">请输入有效{scale === 1 ? "整数" : "数值"}</p>
      )}
    </div>
  );
}

export function ProfileForm({
  profile: p,
  onChange,
  busy,
  onSaveAs,
  onUpdate,
  dirty,
}: {
  profile: Profile | null;
  onChange: (p: Profile) => void;
  busy: boolean;
  onSaveAs: () => void;
  onUpdate: () => void;
  dirty: boolean;
}) {
  const [unit, setUnit] = useState("KB");
  if (!p) return <p className="loading">读取处理设置中…</p>;
  const error = preflightError(p);
  const commonResize =
    p.resize.mode === "none" || p.resize.mode === "shrinkOnly";
  const resize = !commonResize
    ? "special"
    : p.resize.maxWidth !== null || p.resize.maxHeight !== null
      ? "limit"
      : "original";
  const setDirectory = async (dir: string) => {
    if (dir !== "custom") {
      onChange(withOutput(p, { dir }));
      return;
    }
    try {
      const path = await chooseDirectory();
      if (path) onChange(withOutput(p, { dir, dirPath: path }));
    } catch (e) {
      toast.error(String(e));
    }
  };
  return (
    <div data-testid="profile-form" className="profile-form">
      <div className="inspector-heading">
        <h2>处理设置</h2>
        <span className="hint">应用于本批图片</span>
      </div>
      <div className="profile-caption">
        <strong>{profileLabel(p.name)}</strong>
        <p className="hint">
          {p.format.output === "auto"
            ? "保持格式"
            : `输出 ${p.format.output.toUpperCase()}`}{" "}
          ·{" "}
          {p.quality.kind === "lossless"
            ? "无损模式"
            : p.quality.kind === "targetBytes"
              ? "目标大小模式"
              : p.quality.kind === "recommended"
                ? "推荐压缩"
                : "手动压缩强度"}
          {dirty && <span className="draft-mark"> · 已调整</span>}
        </p>
      </div>
      <fieldset disabled={busy} className="parameter-grid">
        <Field label="输出格式" id="format-output">
          <Choice
            id="format-output"
            label="输出格式"
            value={p.format.output}
            disabled={busy}
            onChange={(output) => onChange(withFormat(p, { output }))}
            options={["auto", "jpeg", "png", "webp", ...(isWindows ? [] : ["avif", "heic", "jxl"])].map(
              (v) => [v, v === "auto" ? "自动，保持原格式" : v.toUpperCase()],
            )}
          />
        </Field>
        <Field label="图片尺寸" id="resize-mode">
          <Choice
            id="resize-mode"
            label="图片尺寸"
            value={resize}
            disabled={busy}
            onChange={(value) =>
              onChange(
                withResize(
                  p,
                  value === "original"
                    ? {
                        mode: "none",
                        maxWidth: null,
                        maxHeight: null,
                        keepRatio: true,
                      }
                    : {
                        mode: "shrinkOnly",
                        maxWidth: p.resize.maxWidth ?? 1920,
                        keepRatio: true,
                      },
                ),
              )
            }
            options={[
              ["original", "保持原尺寸"],
              ["limit", "限制最大宽高"],
              ...(!commonResize
                ? [
                    ["special", `导入模式：${p.resize.mode}`] as [
                      string,
                      string,
                    ],
                  ]
                : []),
            ]}
          />
        </Field>
        {resize === "limit" && (
          <Field
            label="最大宽 × 高（px）"
            hint="保持比例，只缩不放；留空表示不限制。"
          >
            <div className="input-pair">
              <NumberField
                label="最大宽度"
                value={p.resize.maxWidth}
                optional
                onChange={(maxWidth) => onChange(withResize(p, { maxWidth }))}
              />
              <NumberField
                label="最大高度"
                value={p.resize.maxHeight}
                optional
                onChange={(maxHeight) => onChange(withResize(p, { maxHeight }))}
              />
            </div>
          </Field>
        )}
        {resize === "special" && (
          <p className="hint">保留导入的尺寸策略：{JSON.stringify(p.resize)}</p>
        )}
        <Field label="保存位置" id="output-dir">
          <Choice
            id="output-dir"
            label="保存位置"
            disabled={busy}
            value={p.output.dir}
            onChange={(value) => void setDirectory(value)}
            options={[
              ["sibling", "原图所在文件夹"],
              ["downloads", "下载文件夹"],
              ["custom", "自定义文件夹"],
              ...(p.output.dir === "override"
                ? [["override", "跟随调用环境"] as [string, string]]
                : []),
            ]}
          />
          {p.output.dir === "custom" && (
            <>
              <p className="path-text">{p.output.dirPath}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void setDirectory("custom")}
              >
                <FolderOpen />
                选择目录
              </Button>
            </>
          )}
        </Field>
        <Collapsible
          key={p.name}
          className="advanced-fields"
          defaultOpen={p.quality.kind !== "recommended" && p.quality.kind !== "lossless"}
        >
          <CollapsibleTrigger asChild>
            <Button variant="ghost" className="advanced-trigger">
              <ChevronDown />
              高级设置
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="advanced-content">
            <Field label="质量模式" id="quality-kind">
              <Choice
                id="quality-kind"
                label="质量模式"
                value={p.quality.kind}
                disabled={busy}
                onChange={(kind) =>
                  onChange(
                    withQuality(p, {
                      kind,
                      ...(kind === "level" && p.quality.level === null
                        ? { level: 4 }
                        : {}),
                      ...(kind === "targetBytes" && p.quality.targetBytes === null
                        ? { targetBytes: 500000 }
                        : {}),
                    }),
                  )
                }
                options={[
                  ["recommended", "推荐，兼顾画质与体积"],
                  ["level", "压缩强度"],
                  ["targetBytes", "目标大小"],
                  ["lossless", "无损"],
                ]}
              />
            </Field>
            {p.quality.kind === "level" && (
              <Field
                label={`压缩强度 ${p.quality.level} / 10`}
                hint="编码参数因格式而异，不预测结果体积。"
              >
                <Slider
                  aria-label="压缩强度"
                  value={[p.quality.level ?? 4]}
                  min={1}
                  max={10}
                  step={1}
                  disabled={busy}
                  onValueChange={([level]) => onChange(withQuality(p, { level }))}
                />
                <div className="range-labels">
                  <span>更清晰</span>
                  <span>更小体积</span>
                </div>
              </Field>
            )}
            {p.quality.kind === "targetBytes" && (
              <Field
                label="目标大小"
                hint={`目标为参考，受画质下限影响；保留下限 ${p.quality.minQualityFloor}。`}
              >
                <div className="input-pair">
                  <NumberField
                    label="目标大小"
                    value={p.quality.targetBytes}
                    scale={unit === "KB" ? 1000 : 1000000}
                    onChange={(targetBytes) =>
                      onChange(withQuality(p, { targetBytes }))
                    }
                  />
                  <Choice
                    label="大小单位"
                    value={unit}
                    disabled={busy}
                    onChange={setUnit}
                    options={[
                      ["KB", "KB"],
                      ["MB", "MB"],
                    ]}
                  />
                </div>
              </Field>
            )}
            {p.quality.kind === "lossless" && (
              <p className="hint">
                使用当前格式的无损编码方案；格式转换不保证可逆归档。
              </p>
            )}

            <Field label="动图策略">
              <Choice
                label="动图策略"
                value={p.format.animated}
                disabled={busy}
                onChange={(animated) => onChange(withFormat(p, { animated }))}
                options={[
                  ["keep", "保持 GIF"],
                  ["webp", "转 WebP"],
                ]}
              />
            </Field>
            <Field
              label="结果文件后缀"
              id="output-suffix"
              hint="例如 照片-default.jpg；支持 {profile}、{date}、{size}。"
            >
              <Input
                id="output-suffix"
                value={p.output.suffixTemplate}
                onChange={(e) =>
                  onChange(withOutput(p, { suffixTemplate: e.target.value }))
                }
              />
              {!p.output.suffixTemplate.trim() && (
                <p className="field-error">后缀不能为空</p>
              )}
            </Field>
            <label className="switch-row">
              <span>保留撤销记录</span>
              <Switch
                checked={p.output.undoManifest}
                disabled={busy}
                onCheckedChange={(undoManifest) =>
                  onChange(withOutput(p, { undoManifest }))
                }
              />
            </label>
            <div className="hint">
              元数据与颜色策略（只读）
              <br />
              EXIF：{p.metadata.exif} · ICC：{p.metadata.icc}
              <br />
              保留现值，执行效果待内核专项验证。
            </div>
          </CollapsibleContent>
        </Collapsible>
      </fieldset>
      {error && (
        <p role="alert" className="field-error">
          {error}
        </p>
      )}
      {busy && <p className="hint">本批参数已冻结，处理结束后可调整。</p>}
      <div className="inspector-save">
        <p className="hint">仅调整本批，不改变已保存预设</p>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={busy || !!error}
            onClick={onSaveAs}
          >
            <Save />
            另存为预设
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                aria-label="预设保存操作"
                disabled={busy || !!error}
              >
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuItem onSelect={onUpdate} disabled={!dirty}>
                更新当前预设
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    </div>
  );
}
