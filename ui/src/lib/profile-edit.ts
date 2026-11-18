import type { Profile } from "./bridge";

/**
 * Profile 编辑的纯函数层：只负责不可变更新与「提前提示」。
 * 校验权威在内核 Profile::validate()——这里只是让用户少跑一趟来回；
 * 保存失败时以 command 返回的错误文案为准（不吞错）。
 */

export interface QualityPatch {
  kind?: string;
  level?: number | null;
  targetBytes?: number | null;
  minQualityFloor?: number;
}

export interface FormatPatch {
  output?: string;
  animated?: string;
}

export interface ResizePatch {
  maxWidth?: number | null;
  maxHeight?: number | null;
  mode?: string;
  keepRatio?: boolean;
}

export interface MetadataPatch {
  exif?: string;
  icc?: string;
}

export interface OutputPatch {
  dir?: string;
  dirPath?: string | null;
  suffixTemplate?: string;
  largerPolicy?: string;
  undoManifest?: boolean;
}

export const withQuality = (p: Profile, patch: QualityPatch): Profile => ({
  ...p,
  quality: { ...p.quality, ...patch },
});

export const withFormat = (p: Profile, patch: FormatPatch): Profile => ({
  ...p,
  format: { ...p.format, ...patch },
});

export const withResize = (p: Profile, patch: ResizePatch): Profile => ({
  ...p,
  resize: { ...p.resize, ...patch },
});

export const withMetadata = (p: Profile, patch: MetadataPatch): Profile => ({
  ...p,
  metadata: { ...p.metadata, ...patch },
});

export const withOutput = (p: Profile, patch: OutputPatch): Profile => ({
  ...p,
  output: { ...p.output, ...patch },
});

/** 数字输入：空串 → null（字段可空语义），非法 → NaN 交给 validate 报错 */
export function parseOptionalNumber(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed === "") return null;
  const n = Number(trimmed);
  return Number.isInteger(n) && n > 0 ? n : Number.NaN;
}

/** 提前提示（返回 null 表示看起来没问题；最终以内核校验为准） */
export function preflightError(p: Profile): string | null {
  if (!p.name.trim()) return "档名不能为空";
  const q = p.quality;
  if (q.kind === "level") {
    if (!Number.isInteger(q.level) || !q.level || q.level < 1 || q.level > 10) return "质量等级需要在 1–10 之间";
  }
  if (q.kind === "targetBytes") {
    if (!Number.isInteger(q.targetBytes) || !q.targetBytes || q.targetBytes <= 0) return "目标大小必须大于 0";
  }
  if (!p.output.suffixTemplate.trim()) return "后缀模板不能为空（否则会覆盖原图）";
  if (p.output.dir === "custom" && !(p.output.dirPath ?? "").trim()) return "自定义目录需要填写路径";
  if (Number.isNaN(q.level ?? 0) || Number.isNaN(q.minQualityFloor)) return "请输入合法数字";
  if (p.resize.maxWidth !== null && (!Number.isInteger(p.resize.maxWidth) || p.resize.maxWidth <= 0)) return "最大宽度必须是正整数";
  if (p.resize.maxHeight !== null && (!Number.isInteger(p.resize.maxHeight) || p.resize.maxHeight <= 0)) return "最大高度必须是正整数";
  return null;
}
