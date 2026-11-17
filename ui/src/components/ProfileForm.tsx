import { useEffect, useState } from "react";
import type { Profile } from "../lib/bridge";
import {
  parseOptionalNumber,
  preflightError,
  withFormat,
  withMetadata,
  withOutput,
  withQuality,
  withResize,
} from "../lib/profile-edit";

interface Props {
  profile: Profile | null;
  names: string[];
  selected: string;
  onSelect: (name: string) => void;
  /** 保存回调（走 Tauri command；返回错误文案时展示） */
  onSave?: (profile: Profile) => Promise<void>;
}

const LEVELS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const FORMATS = ["auto", "jpeg", "png", "webp", "avif", "heic", "jxl"];
const EXIF = ["keep", "strip", "privacy"];
const ICC = ["srgb", "keep", "strip"];

/** Profile 编辑器（T-007）：受控输入 + 内核校验兜底；不做视觉迭代。 */
export function ProfileForm({ profile, names, selected, onSelect, onSave }: Props) {
  const [draft, setDraft] = useState<Profile | null>(profile);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  // 只在「切档」时重置草稿与状态；保存后 setProfile 不应把「已保存」提示清掉
  useEffect(() => {
    setDraft(profile);
    setStatus(null);
  }, [profile?.name]);

  const dirty = draft !== null && profile !== null && JSON.stringify(draft) !== JSON.stringify(profile);
  const localError = draft ? preflightError(draft) : null;

  const save = async () => {
    if (!draft || !onSave) return;
    if (localError) {
      setStatus({ kind: "error", text: localError });
      return;
    }
    setSaving(true);
    setStatus(null);
    try {
      await onSave(draft);
      setStatus({ kind: "ok", text: "已保存" });
    } catch (e) {
      setStatus({ kind: "error", text: String(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="card" data-testid="profile-form">
      <header className="card-head">
        <h2>Profile</h2>
        <div className="row">
          <select aria-label="profile" value={selected} onChange={(e) => onSelect(e.target.value)}>
            {names.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
          {onSave && (
            <button type="button" onClick={save} disabled={!dirty || saving}>
              {saving ? "保存中…" : "保存"}
            </button>
          )}
        </div>
      </header>

      {!draft ? (
        <p className="muted">读取中…</p>
      ) : (
        <>
          <dl className="fields">
            <dt>质量</dt>
            <dd className="row" data-testid="quality">
              <select
                aria-label="quality-kind"
                value={draft.quality.kind}
                onChange={(e) => setDraft(withQuality(draft, { kind: e.target.value }))}
              >
                <option value="level">档位</option>
                <option value="targetBytes">目标大小</option>
                <option value="lossless">无损</option>
              </select>
              {draft.quality.kind === "level" && (
                <select
                  aria-label="quality-level"
                  value={draft.quality.level ?? 4}
                  onChange={(e) => setDraft(withQuality(draft, { level: Number(e.target.value) }))}
                >
                  {LEVELS.map((l) => (
                    <option key={l} value={l}>
                      level {l}
                    </option>
                  ))}
                </select>
              )}
              {draft.quality.kind === "targetBytes" && (
                <label className="inline">
                  目标
                  <input
                    aria-label="quality-targetBytes"
                    type="number"
                    min={1}
                    value={draft.quality.targetBytes ?? ""}
                    onChange={(e) =>
                      setDraft(withQuality(draft, { targetBytes: parseOptionalNumber(e.target.value) }))
                    }
                  />
                  字节 · floor
                  <input
                    aria-label="quality-floor"
                    type="number"
                    min={1}
                    max={10}
                    value={draft.quality.minQualityFloor}
                    onChange={(e) =>
                      setDraft(
                        withQuality(draft, { minQualityFloor: Number(e.target.value) || 3 }),
                      )
                    }
                  />
                </label>
              )}
            </dd>

            <dt>格式</dt>
            <dd className="row" data-testid="format">
              <select
                aria-label="format-output"
                value={draft.format.output}
                onChange={(e) => setDraft(withFormat(draft, { output: e.target.value }))}
              >
                {FORMATS.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
              <label className="inline">
                动图
                <select
                  aria-label="format-animated"
                  value={draft.format.animated}
                  onChange={(e) => setDraft(withFormat(draft, { animated: e.target.value }))}
                >
                  <option value="keep">保持 GIF</option>
                  <option value="webp">转 WebP</option>
                </select>
              </label>
            </dd>

            <dt>尺寸</dt>
            <dd className="row">
              <label className="inline">
                宽
                <input
                  aria-label="resize-width"
                  type="number"
                  min={1}
                  placeholder="不限"
                  value={draft.resize.maxWidth ?? ""}
                  onChange={(e) =>
                    setDraft(withResize(draft, { maxWidth: parseOptionalNumber(e.target.value) }))
                  }
                />
              </label>
              <label className="inline">
                高
                <input
                  aria-label="resize-height"
                  type="number"
                  min={1}
                  placeholder="不限"
                  value={draft.resize.maxHeight ?? ""}
                  onChange={(e) =>
                    setDraft(withResize(draft, { maxHeight: parseOptionalNumber(e.target.value) }))
                  }
                />
              </label>
              <select
                aria-label="resize-mode"
                value={draft.resize.mode}
                onChange={(e) => setDraft(withResize(draft, { mode: e.target.value }))}
              >
                <option value="shrinkOnly">只缩不放</option>
                <option value="always">总是缩放</option>
                <option value="none">不缩放</option>
              </select>
            </dd>

            <dt>元数据</dt>
            <dd className="row">
              <label className="inline">
                EXIF
                <select
                  aria-label="metadata-exif"
                  value={draft.metadata.exif}
                  onChange={(e) => setDraft(withMetadata(draft, { exif: e.target.value }))}
                >
                  {EXIF.map((v) => (
                    <option key={v} value={v}>
                      {v}
                    </option>
                  ))}
                </select>
              </label>
              <label className="inline">
                ICC
                <select
                  aria-label="metadata-icc"
                  value={draft.metadata.icc}
                  onChange={(e) => setDraft(withMetadata(draft, { icc: e.target.value }))}
                >
                  {ICC.map((v) => (
                    <option key={v} value={v}>
                      {v}
                    </option>
                  ))}
                </select>
              </label>
            </dd>

            <dt>输出</dt>
            <dd className="row">
              <label className="inline">
                后缀
                <input
                  aria-label="output-suffix"
                  type="text"
                  value={draft.output.suffixTemplate}
                  onChange={(e) => setDraft(withOutput(draft, { suffixTemplate: e.target.value }))}
                />
              </label>
              <label className="inline">
                <input
                  type="checkbox"
                  checked={draft.output.undoManifest}
                  onChange={(e) => setDraft(withOutput(draft, { undoManifest: e.target.checked }))}
                />
                撤销清单
              </label>
            </dd>
          </dl>

          {status && (
            <p className={status.kind === "ok" ? "status ok" : "status error"} data-testid="save-status">
              {status.kind === "ok" ? "✓ " : "✗ "}
              {status.text}
            </p>
          )}
        </>
      )}
    </section>
  );
}
