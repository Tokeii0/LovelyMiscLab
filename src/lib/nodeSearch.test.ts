import { describe, expect, it } from "vitest";

import { searchDescriptors } from "@/lib/nodeSearch";
import type { NodeDescriptor } from "@/lib/types";

const d = (id: string, displayName: string, category = "编码", description = ""): NodeDescriptor => ({
  id,
  displayName,
  category,
  description,
  color: "#888",
  inputs: [],
  outputs: [],
  params: [],
  cost: "cheap",
});

const list = [
  d("hex_decode", "Hex 解码"),
  d("base64_decode", "Base64 解码"),
  d("base64_encode", "Base64 编码"),
  d("zip_extract", "解压", "压缩包", "解开 ZIP/7z/RAR 压缩包"),
];

describe("searchDescriptors", () => {
  it("returns everything for an empty query", () => {
    expect(searchDescriptors(list, "  ")).toHaveLength(4);
  });

  it("matches name, id and description, ranking name/id hits first", () => {
    expect(searchDescriptors(list, "base64").map((x) => x.id)).toEqual(["base64_decode", "base64_encode"]);
    expect(searchDescriptors(list, "rar").map((x) => x.id)).toEqual(["zip_extract"]);
    expect(searchDescriptors(list, "base64_encode")[0].id).toBe("base64_encode");
  });

  it("requires every token", () => {
    expect(searchDescriptors(list, "base64 解码").map((x) => x.id)).toEqual(["base64_decode"]);
  });
});
