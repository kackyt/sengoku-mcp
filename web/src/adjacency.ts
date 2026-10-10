// 国同士の隣接（行き来できる国）の情報
//
// マスターデータ（static/master_data/neighbor.csv）をビルド時に文字列として取り込む
// （REST API は勢力図のみを返すため、隣接情報はマスターデータから直接得る）。
import neighborCsv from "../../static/master_data/neighbor.csv?raw";

/** neighbor.csv（ヘッダー: ID1,ID2）を、国IDの小さい順の組に変換する */
export function parseNeighborCsv(csv: string): Array<readonly [number, number]> {
  return csv
    .trim()
    .split(/\r?\n/)
    .slice(1)
    .map((line) => line.split(",").map((v) => Number(v.trim())))
    .filter(([a, b]) => Number.isInteger(a) && Number.isInteger(b))
    .map(([a, b]) => (a! < b! ? ([a!, b!] as const) : ([b!, a!] as const)))
    .sort((x, y) => x[0] - y[0] || x[1] - y[1]);
}

/** 隣接する国の組（国IDの小さい順） */
export const NEIGHBOR_PAIRS: ReadonlyArray<readonly [number, number]> =
  parseNeighborCsv(neighborCsv);

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
