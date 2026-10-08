// 国同士の隣接（行き来できる国）の情報
//
// マスターデータ（static/master_data/neighbor.csv）の隣接情報は地図データの生成時に
// japanMap.json へ取り込んでいる（REST API は勢力図のみを返すため）。
import japanMap from "./map/japanMap.json";

/** 隣接する国の組（国IDの小さい順） */
export const NEIGHBOR_PAIRS: ReadonlyArray<readonly [number, number]> = japanMap.edges.map(
  (e) => [e.a, e.b] as const,
);

/** 国ID → 隣接する国IDの一覧（国ID順） */
export const NEIGHBORS: ReadonlyMap<number, readonly number[]> = (() => {
  const map = new Map<number, number[]>();
  for (const [a, b] of NEIGHBOR_PAIRS) {
    map.set(a, [...(map.get(a) ?? []), b]);
    map.set(b, [...(map.get(b) ?? []), a]);
  }
  for (const list of map.values()) list.sort((x, y) => x - y);
  return map;
})();

/** 指定した国の隣接国ID（隣接国がなければ空配列） */
export function neighborsOf(kuniId: number): readonly number[] {
  return NEIGHBORS.get(kuniId) ?? [];
}
