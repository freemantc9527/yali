import type { Profile } from "./bridge";

/** 档名规则与内核 ProfileStore::path_for 对齐：非空、不含路径分隔符、不以点开头 */
export function validateProfileName(name: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) return "档名不能为空";
  if (trimmed.includes("/") || trimmed.includes("\\")) return "档名不能包含路径分隔符";
  if (trimmed.startsWith(".")) return "档名不能以点开头";
  if (trimmed.length > 64) return "档名太长（≤64 字符）";
  return null;
}

/** 另存为：换档名并返回新对象（不动原对象） */
export function renameProfile(profile: Profile, name: string): Profile {
  const error = validateProfileName(name);
  if (error) throw new Error(error);
  return { ...profile, name: name.trim() };
}

/**
 * 解析导入的 JSON：语法错误原样抛出（serde_json 风格的行列信息这里由 JSON.parse 给位置），
 * 结构校验交给内核 validate（保存时兜底）；这里只做「顶层必须是对象」的粗筛。
 */
export function parseProfileJson(raw: string): Profile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    throw new Error(`JSON 解析失败：${(e as Error).message}`);
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("Profile 必须是 JSON 对象");
  }
  return parsed as Profile;
}

export function exportProfileJson(profile: Profile): string {
  return JSON.stringify(profile, null, 2);
}
