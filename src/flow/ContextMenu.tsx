import { cn } from "@/lib/utils";
import { useEscapeToClose } from "@/store/modal";

export interface MenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  /** Right-aligned shortcut hint, e.g. "Ctrl+D". */
  hint?: string;
  /** Draw a divider above this item. */
  separator?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** A cursor-positioned context menu with a full-screen backdrop to dismiss. */
export function ContextMenu({ x, y, items, onClose }: Props) {
  // Counts as a modal layer: Esc closes it and canvas shortcuts pause meanwhile.
  useEscapeToClose(true, onClose);
  const left = Math.max(4, Math.min(x, window.innerWidth - 230));
  const top = Math.max(4, Math.min(y, window.innerHeight - (items.length * 32 + 16)));

  return (
    <>
      <div
        className="fixed inset-0 z-[70]"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        role="menu"
        className="fixed z-[71] min-w-[180px] max-w-[260px] rounded-md border border-border bg-popover p-1 text-xs text-popover-foreground shadow-lg"
        style={{ left, top }}
      >
        {items.map((it, i) => (
          <div key={i}>
            {it.separator && <div className="my-1 h-px bg-border" />}
            <button
              role="menuitem"
              disabled={it.disabled}
              onClick={() => {
                onClose();
                it.onClick();
              }}
              className={cn(
                "flex w-full items-center gap-3 rounded px-2 py-1.5 text-left transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-40",
                it.danger && "text-destructive hover:bg-destructive/10"
              )}
            >
              <span className="flex-1 truncate">{it.label}</span>
              {it.hint && <span className="text-[10px] text-muted-foreground">{it.hint}</span>}
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
