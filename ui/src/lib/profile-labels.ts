export const labels: Record<string, string> = {
  default: "日常压缩",
  jpg: "转 JPG",
  thumbnail: "缩小尺寸",
  archive: "无损归档",
};
export const profileLabel = (name: string) => labels[name] ?? name;
const order = Object.keys(labels);
export function sortProfiles(names: string[]) {
  return [...names].sort(
    (a, b) =>
      (order.includes(a) ? order.indexOf(a) : 99) -
        (order.includes(b) ? order.indexOf(b) : 99) || a.localeCompare(b),
  );
}
