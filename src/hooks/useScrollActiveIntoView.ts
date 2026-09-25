import { useEffect, type RefObject } from "react";

/** Keep the keyboard-highlighted row (`data-index={i}`) visible inside a scrolling list. */
export function useScrollActiveIntoView(container: RefObject<HTMLElement | null>, active: number) {
  useEffect(() => {
    const el = container.current?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [container, active]);
}
