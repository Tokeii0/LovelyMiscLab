/** Shape every Tauri command rejects with (see `src-tauri/src/error.rs`). */
export interface AppError {
  code: string;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    !!e &&
    typeof e === "object" &&
    typeof (e as AppError).code === "string" &&
    typeof (e as AppError).message === "string"
  );
}

/** Human-readable text for anything thrown or rejected — never "[object Object]". */
export function errorMessage(e: unknown): string {
  if (e == null) return "未知错误";
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message || e.name;
  if (typeof e === "object" && "message" in e) {
    const m = (e as { message: unknown }).message;
    if (typeof m === "string" && m) return m;
  }
  try {
    const json = JSON.stringify(e);
    if (json && json !== "{}") return json;
  } catch {
    /* fall through */
  }
  return typeof e === "object" ? "未知错误" : String(e);
}

/** The backend error code (`ai_config`, `io`, `cancelled`…) or null for non-AppErrors. */
export function errorCode(e: unknown): string | null {
  return isAppError(e) ? e.code : null;
}

/** True when the error means "the user/engine cancelled", not a real failure. */
export function isCancelled(e: unknown): boolean {
  if (errorCode(e) === "cancelled") return true;
  return /cancel|取消/i.test(errorMessage(e));
}
