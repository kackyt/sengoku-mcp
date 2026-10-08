// 勢力図（国ごとの支配大名）の組み立て
import type { Daimyo, MyStatus } from "./api/client";

/** 地図に表示する1国分の勢力情報 */
export interface Territory {
  kuniId: number;
  kuniName: string;
  owner: Daimyo;
  /** 自領かどうか */
  mine: boolean;
}

/** 自領と他国の情報を、国ID → 勢力情報 の表にまとめる */
export function buildTerritories(status: MyStatus): Map<number, Territory> {
  const territories = new Map<number, Territory>();
  for (const kuni of status.my_kunis) {
    territories.set(kuni.id, {
      kuniId: kuni.id,
      kuniName: kuni.name,
      owner: status.daimyo,
      mine: true,
    });
  }
  for (const kuni of status.other_kunis) {
    territories.set(kuni.id, {
      kuniId: kuni.id,
      kuniName: kuni.name,
      owner: kuni.daimyo,
      mine: false,
    });
  }
  return territories;
}

/** 大名ID → 領地数 */
export function countByOwner(territories: Iterable<Territory>): Map<number, number> {
  const counts = new Map<number, number>();
  for (const t of territories) {
    counts.set(t.owner.id, (counts.get(t.owner.id) ?? 0) + 1);
  }
  return counts;
}
