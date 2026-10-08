import { describe, expect, it } from "vitest";
import { NEIGHBOR_PAIRS, neighborsOf, parseNeighborCsv } from "./adjacency";

describe("adjacency", () => {
  it("neighbor.csv を国IDの小さい順の組に変換する", () => {
    expect(parseNeighborCsv("ID1,ID2\n2,1\r\n3,2\n\n")).toEqual([
      [1, 2],
      [2, 3],
    ]);
  });

  it("マスターデータの隣接情報（17組）を読み込む", () => {
    expect(NEIGHBOR_PAIRS).toHaveLength(17);
  });

  it("国ごとの隣接国を双方向に引ける", () => {
    expect(neighborsOf(7)).toEqual([4, 6, 8]); // 尾張: 甲信・三河・山城
    expect(neighborsOf(10)).toEqual([9]); // 四国: 安芸（海を挟む）
    expect(neighborsOf(99)).toEqual([]);
  });
});
