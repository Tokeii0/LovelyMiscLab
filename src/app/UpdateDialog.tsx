import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, Download, ExternalLink, Loader2, Sparkles, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { inTauri } from "@/lib/devMocks";
import { useUpdate } from "@/store/update";

export function UpdateDialog() {
  const status = useUpdate((s) => s.status);
  const info = useUpdate((s) => s.info);
  const error = useUpdate((s) => s.error);
  const open = useUpdate((s) => s.dialogOpen);
  const install = useUpdate((s) => s.install);
  const close = useUpdate((s) => s.closeDialog);

  if (!open) return null;

  const installing = status === "installing";
  const openRelease = () => {
    if (inTauri && info?.releaseUrl) void openUrl(info.releaseUrl);
  };

  return (
    <div
      className="fixed inset-0 z-[85] flex items-center justify-center bg-black/45 p-4"
      onClick={installing ? undefined : close}
    >
      <div
        className="flex max-h-[80vh] w-[560px] max-w-[94vw] flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border px-4 py-3">
          <span className="flex h-8 w-8 items-center justify-center rounded-md bg-primary/10 text-primary">
            {status === "uptodate" ? <Check className="h-4 w-4" /> : <Sparkles className="h-4 w-4" />}
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">
              {status === "uptodate"
                ? "已是最新版本"
                : status === "available" || installing
                  ? "发现新版本"
                  : "检查更新"}
            </div>
            {info && (
              <div className="text-xs text-muted-foreground">
                当前 v{info.current}
                {info.latest ? ` · 最新 v${info.latest}` : ""}
              </div>
            )}
          </div>
          {!installing && (
            <button
              onClick={close}
              className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <X className="h-5 w-5" />
            </button>
          )}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3">
          {status === "uptodate" && (
            <p className="text-sm text-muted-foreground">
              你已在使用最新版本 <span className="font-medium text-foreground">v{info?.current}</span>。
            </p>
          )}

          {status === "error" && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
              {error || "检查更新时出错。"}
            </div>
          )}

          {(status === "available" || installing) && info && (
            <>
              {installing ? (
                <div className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  正在下载并替换程序，完成后会自动重启……请勿关闭窗口。
                </div>
              ) : (
                <>
                  <div className="mb-2 text-xs font-medium text-muted-foreground">更新内容</div>
                  <pre className="max-h-[40vh] select-text overflow-auto whitespace-pre-wrap break-words rounded-md bg-background p-3 text-xs leading-relaxed">
                    {info.notes?.trim() || "（此版本没有提供更新说明）"}
                  </pre>
                  {info.assetName && (
                    <div className="mt-2 text-[10px] text-muted-foreground">下载文件：{info.assetName}</div>
                  )}
                </>
              )}
            </>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-border px-4 py-3">
          {(status === "available" || status === "error") && info?.releaseUrl && (
            <Button variant="outline" size="sm" onClick={openRelease}>
              <ExternalLink className="mr-1 h-3.5 w-3.5" /> 发布页
            </Button>
          )}
          {status === "available" && !installing && (
            <>
              <Button variant="outline" size="sm" onClick={close}>
                稍后
              </Button>
              <Button size="sm" onClick={() => void install()} disabled={!info?.downloadUrl}>
                <Download className="mr-1 h-3.5 w-3.5" /> 下载并安装
              </Button>
            </>
          )}
          {installing && (
            <Button size="sm" disabled>
              <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 安装中…
            </Button>
          )}
          {(status === "uptodate" || status === "error") && (
            <Button size="sm" onClick={close}>
              知道了
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
