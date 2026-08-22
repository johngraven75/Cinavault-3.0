export const TITLE_LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
export type TitleInitialFilter = "all" | "#" | (typeof TITLE_LETTERS)[number];

export function getTitleInitial(title: string): string {
  const first = title.trim().charAt(0).toUpperCase();
  return /^[A-Z]$/.test(first) ? first : "#";
}

export function filterItemsByTitleInitial<T extends { title: string }>(
  items: T[],
  selectedInitial: TitleInitialFilter,
): T[] {
  if (selectedInitial === "all") return items;
  return items.filter(
    (item) => getTitleInitial(item.title) === selectedInitial,
  );
}
