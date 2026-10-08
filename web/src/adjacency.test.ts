import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { bordersOf, NEIGHBOR_PAIRS, neighborsOf } from "./adjacency";

describe("adjacency", () => {
  it("地図データの隣接情報がマスターデータ（neighbor.csv）と一致する", () => {
    const csv = readFileSync(resolve(__dirname, "../../static/master_data/neighbor.csv"), "utf8");
    const expected = csv
      .trim()
      .split(/\r?\n/)
      .slice(1)
      .map((line) => line.split(",").map(Number).sort((a, b) => a - b))
      .sort((x, y) => x[0]! - y[0]! || x[1]! - y[1]!);
    expect(NEIGHBOR_PAIRS.map((p) => [...p])).toEqual(expected);
  });

  it("国ごとの隣接国を双方向に引ける", () => {
    expect(neighborsOf(7)).toEqual([4, 6, 8]); // 尾張: 甲信・三河・山城
    expect(neighborsOf(10)).toEqual([9]); // 四国: 安芸（海を挟む）
    expect(neighborsOf(99)).toEqual([]);
  });

  it("自領に隣接する他国を重複なく返す", () => {
    // 尾張・三河を支配 → 甲信・武蔵・山城
    expect(bordersOf([6, 7])).toEqual([4, 5, 8]);
  });
});
