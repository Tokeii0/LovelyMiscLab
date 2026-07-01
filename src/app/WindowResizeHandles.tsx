import { getCurrentWindow } from "@tauri-apps/api/window";

import { inTauri } from "@/lib/devMocks";

// With `decorations: false` the native resize border is gone, so we provide our
// own thin edge/corner grips that trigger Tauri's window resize dragging.
type Dir =
  | "North"
  | "South"
  | "East"
  | "West"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

const grips: { dir: Dir; className: string }[] = [
  { dir: "North", className: "top-0 left-2 right-2 h-1 cursor-n-resize" },
  { dir: "South", className: "bottom-0 left-2 right-2 h-1 cursor-s-resize" },
  { dir: "West", className: "left-0 top-2 bottom-2 w-1 cursor-w-resize" },
  { dir: "East", className: "right-0 top-2 bottom-2 w-1 cursor-e-resize" },
  { dir: "NorthWest", className: "top-0 left-0 h-2 w-2 cursor-nw-resize" },
  { dir: "NorthEast", className: "top-0 right-0 h-2 w-2 cursor-ne-resize" },
  { dir: "SouthWest", className: "bottom-0 left-0 h-2 w-2 cursor-sw-resize" },
  { dir: "SouthEast", className: "bottom-0 right-0 h-2 w-2 cursor-se-resize" },
];

export function WindowResizeHandles() {
  if (!inTauri) return null;

  const onPointerDown = (dir: Dir) => (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    // ResizeDirection is serialized from these names on the Rust side.
    getCurrentWindow()
      .startResizeDragging(dir as never)
      .catch(() => {});
  };

  return (
    <>
      {grips.map((g) => (
        <div
          key={g.dir}
          onPointerDown={onPointerDown(g.dir)}
          className={`fixed z-50 ${g.className}`}
        />
      ))}
    </>
  );
}
