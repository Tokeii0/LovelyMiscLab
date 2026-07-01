export interface MenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** A cursor-positioned context menu with a full-screen backdrop to dismiss. */
export function ContextMenu({ x, y, items, onClose }: Props) {
  const left = Math.min(x, window.innerWidth - 180);
  const top = Math.min(y, window.innerHeight - (items.length * 32 + 16));

  return (
    <>
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        className="fixed z-50 min-w-[160px] rounded-md border border-border bg-popover p-1 text-xs text-popover-foreground shadow-lg"
        style={{ left, top }}
      >
        {items.map((it, i) => (
          <button
            key={i}
            onClick={() => {
              it.onClick();
              onClose();
            }}
            className={`block w-full rounded px-2 py-1.5 text-left transition-colors hover:bg-accent ${
              it.danger ? "text-destructive hover:bg-destructive/10" : ""
            }`}
          >
            {it.label}
          </button>
        ))}
      </div>
    </>
  );
}
