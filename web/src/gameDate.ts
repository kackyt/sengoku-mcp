// ターン数 → ゲーム内の年・季節
//
// 1ターン＝1季節で、第1ターンを1560年春とする。季節の順序はエンジンの
// TurnNumber::season()（(ターン - 1) % 4 → 0:春, 1:夏, 2:秋, 3:冬）と揃えている。

/** 第1ターンの年 */
export const START_YEAR = 1560;

const SEASONS = ["春", "夏", "秋", "冬"] as const;

export type Season = (typeof SEASONS)[number];

export interface GameDate {
  year: number;
  season: Season;
}

/** ターン数（1始まり）を年・季節に変換する */
export function turnToDate(turn: number): GameDate {
  const index = Math.max(0, Math.floor(turn) - 1);
  return { year: START_YEAR + Math.floor(index / 4), season: SEASONS[index % 4]! };
}
