import { nodeSummary } from "@/flow/nodeDescriptions";
import type { NodeDescriptor } from "@/lib/types";

/**
 * The one node search used everywhere (library, search menu, palette, help).
 * Every whitespace-separated token must appear in the name, id, category or
 * description; results rank exact > prefix > substring matches on name/id.
 */
export function searchDescriptors(list: NodeDescriptor[], query: string): NodeDescriptor[] {
  const tokens = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return list;
  const q = tokens.join(" ");
  const scored: { d: NodeDescriptor; score: number; i: number }[] = [];
  list.forEach((d, i) => {
    const name = d.displayName.toLowerCase();
    const id = d.id.toLowerCase();
    const hay = `${name} ${id} ${d.category.toLowerCase()} ${nodeSummary(d).toLowerCase()} ${(
      d.description ?? ""
    ).toLowerCase()}`;
    if (!tokens.every((t) => hay.includes(t))) return;
    let score = 0;
    if (name === q || id === q) score = 100;
    else if (name.startsWith(q) || id.startsWith(q)) score = 60;
    else if (name.includes(q) || id.includes(q)) score = 40;
    else if (tokens.every((t) => name.includes(t) || id.includes(t))) score = 25;
    scored.push({ d, score, i });
  });
  scored.sort((a, b) => b.score - a.score || a.i - b.i);
  return scored.map((s) => s.d);
}
