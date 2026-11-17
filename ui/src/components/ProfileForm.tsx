import type { Profile } from "../lib/bridge";

interface Props {
  profile: Profile | null;
  names: string[];
  selected: string;
  onSelect: (name: string) => void;
}

const dash = (v: unknown) => (v === null || v === undefined || v === "" ? "—" : String(v));

/** Profile 只读视图（T-005 只做「读」；编辑能力在 M1 视觉/表单任务卡）。 */
export function ProfileForm({ profile, names, selected, onSelect }: Props) {
  return (
    <section className="card" data-testid="profile-form">
      <header className="card-head">
        <h2>Profile</h2>
        <select
          aria-label="profile"
          value={selected}
          onChange={(e) => onSelect(e.target.value)}
        >
          {names.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </header>

      {!profile ? (
        <p className="muted">读取中…</p>
      ) : (
        <dl className="fields">
          <dt>质量</dt>
          <dd data-testid="quality">
            {profile.quality.kind === "level"
              ? `level ${dash(profile.quality.level)}（1–10）`
              : profile.quality.kind === "targetBytes"
                ? `目标 ${dash(profile.quality.targetBytes)} 字节（floor ${profile.quality.minQualityFloor}）`
                : "无损"}
          </dd>

          <dt>格式</dt>
          <dd data-testid="format">
            {profile.format.output} · 动图 {profile.format.animated}
          </dd>

          <dt>尺寸</dt>
          <dd>
            {profile.resize.mode === "none"
              ? "不缩放"
              : `≤ ${dash(profile.resize.maxWidth)}×${dash(profile.resize.maxHeight)}（${profile.resize.mode}）`}
          </dd>

          <dt>元数据</dt>
          <dd>
            EXIF {profile.metadata.exif} · ICC {profile.metadata.icc}
          </dd>

          <dt>输出</dt>
          <dd>
            {profile.output.dir} · 后缀 {profile.output.suffixTemplate} ·{" "}
            {profile.output.largerPolicy === "keepOriginal" ? "负优化拦截开" : "允许变大"} ·{" "}
            {profile.output.undoManifest ? "撤销清单开" : "撤销清单关"}
          </dd>
        </dl>
      )}
    </section>
  );
}
