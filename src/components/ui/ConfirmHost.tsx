import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useConfirmStore } from "@/store/confirm";

/** Renders the head of the confirm/prompt/choose queue. Mounted once in App. */
export function ConfirmHost() {
  const req = useConfirmStore((s) => s.queue[0]);
  const settle = useConfirmStore((s) => s.settle);
  const [text, setText] = useState("");

  useEffect(() => {
    if (req?.kind === "prompt") setText(req.initial);
  }, [req]);

  if (!req) return null;

  const cancel = () => settle(req.kind === "confirm" ? false : null);
  const submitPrompt = () => settle(text);

  return (
    <Dialog key={req.id} open onClose={cancel} className="w-[420px]" overlayClassName="z-[100]" ariaLabel={req.title}>
      <div className="px-4 pt-4">
        <div className="text-base font-semibold">{req.title}</div>
        {req.message && (
          <p className="mt-1.5 whitespace-pre-wrap text-sm text-muted-foreground">{req.message}</p>
        )}
        {req.kind === "prompt" && (
          <input
            data-autofocus
            value={text}
            placeholder={req.placeholder}
            onChange={(e) => setText(e.target.value)}
            onFocus={(e) => e.currentTarget.select()}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                e.preventDefault();
                submitPrompt();
              }
            }}
            className="mt-3 w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm outline-none focus:border-ring"
          />
        )}
      </div>
      <div className="flex items-center justify-end gap-2 px-4 py-3">
        <Button variant="outline" size="sm" onClick={cancel}>
          取消
        </Button>
        {req.kind === "confirm" && (
          <Button
            size="sm"
            data-autofocus
            variant={req.danger ? "destructive" : "default"}
            onClick={() => settle(true)}
          >
            {req.confirmText}
          </Button>
        )}
        {req.kind === "prompt" && (
          <Button size="sm" onClick={submitPrompt}>
            {req.confirmText}
          </Button>
        )}
        {req.kind === "choose" &&
          req.options.map((o, i) => (
            <Button
              key={o.label}
              size="sm"
              variant={o.variant ?? "default"}
              data-autofocus={i === req.options.length - 1 ? true : undefined}
              onClick={() => settle(o.value)}
            >
              {o.label}
            </Button>
          ))}
      </div>
    </Dialog>
  );
}
