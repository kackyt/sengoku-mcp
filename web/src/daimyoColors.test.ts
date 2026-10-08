import { describe, expect, it } from "vitest";
import { NEIGHBOR_PAIRS } from "./adjacency";
import { daimyoColor } from "./daimyoColors";

describe("daimyoColor", () => {
  it("12大名すべてに異なる色を割り当てる", () => {
    const colors = Array.from({ length: 12 }, (_, i) => daimyoColor(i + 1));
    expect(new Set(colors).size).toBe(12);
  });

  it("ゲーム開始時に隣接する大名（国ID＝大名ID）同士は別の色になる", () => {
    for (const [a, b] of NEIGHBOR_PAIRS) {
      expect(daimyoColor(a)).not.toBe(daimyoColor(b));
    }
  });

  it("未知の大名には既定の色を返す", () => {
    expect(daimyoColor(99)).toBe(daimyoColor(100));
  });
});
