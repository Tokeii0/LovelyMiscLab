import { describe, expect, it } from "vitest";

import { errorCode, errorMessage, isAppError, isCancelled } from "@/lib/errors";

describe("errorMessage", () => {
  it("unwraps the backend {code, message} shape", () => {
    expect(errorMessage({ code: "ai_config", message: "AI 文本模型未配置" })).toBe("AI 文本模型未配置");
  });

  it("handles strings, Errors and junk without [object Object]", () => {
    expect(errorMessage("boom")).toBe("boom");
    expect(errorMessage(new Error("bad"))).toBe("bad");
    expect(errorMessage({ foo: 1 })).toBe('{"foo":1}');
    expect(errorMessage(null)).toBe("未知错误");
    expect(errorMessage({})).not.toContain("[object");
  });
});

describe("errorCode / isCancelled", () => {
  it("reads codes only from AppErrors", () => {
    expect(errorCode({ code: "io", message: "x" })).toBe("io");
    expect(errorCode(new Error("x"))).toBeNull();
    expect(isAppError({ code: "io" })).toBe(false);
  });

  it("detects cancellation by code or message", () => {
    expect(isCancelled({ code: "cancelled", message: "已取消" })).toBe(true);
    expect(isCancelled({ code: "core", message: "operation cancelled" })).toBe(true);
    expect(isCancelled({ code: "core", message: "missing input" })).toBe(false);
  });
});
