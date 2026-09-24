import { useEffect, useRef, type ReactNode } from "react";

import { cn } from "@/lib/utils";
import { useEscapeToClose } from "@/store/modal";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface DialogProps {
  open: boolean;
  onClose: () => void;
  /** false = Esc and backdrop clicks do nothing (busy, or a form with unsaved input). */
  dismissible?: boolean;
  /** Classes for the panel (width, max-height…). */
  className?: string;
  /** Classes for the full-screen overlay (z-index override for stacked dialogs). */
  overlayClassName?: string;
  ariaLabel?: string;
  children: ReactNode;
}

/** Shared modal shell: backdrop, Esc, focus trap + restore, role="dialog". */
export function Dialog({
  open,
  onClose,
  dismissible = true,
  className,
  overlayClassName,
  ariaLabel,
  children,
}: DialogProps) {
  const panel = useRef<HTMLDivElement>(null);
  useEscapeToClose(open, onClose, dismissible);

  // Move focus into the dialog on open and give it back on close.
  useEffect(() => {
    if (!open) return;
    const prev = document.activeElement as HTMLElement | null;
    const el = panel.current;
    if (el && !el.contains(document.activeElement)) {
      const auto = el.querySelector<HTMLElement>("[autofocus], [data-autofocus]");
      (auto ?? el.querySelector<HTMLElement>(FOCUSABLE) ?? el).focus();
    }
    return () => {
      if (prev && document.contains(prev)) prev.focus();
    };
  }, [open]);

  if (!open) return null;

  const trapTab = (e: React.KeyboardEvent) => {
    if (e.key !== "Tab" || !panel.current) return;
    const items = Array.from(panel.current.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (n) => n.offsetParent !== null
    );
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className={cn("fixed inset-0 z-[80] flex items-center justify-center bg-black/45 p-4", overlayClassName)}
      // mousedown (not click) so a text selection dragged out of the panel doesn't close it.
      onMouseDown={(e) => {
        if (dismissible && e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        tabIndex={-1}
        onKeyDown={trapTab}
        className={cn(
          "flex max-h-[85vh] max-w-[94vw] flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl outline-none",
          className
        )}
      >
        {children}
      </div>
    </div>
  );
}
