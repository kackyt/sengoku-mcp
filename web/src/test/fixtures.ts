import type { MyStatus } from "../api/client";

/** テスト用の状況（織田家が尾張・三河を支配） */
export const sampleStatus: MyStatus = {
  turn: 3,
  daimyo: { id: 7, name: "織田" },
  my_kunis: [
    { id: 6, name: "三河", kin: 120, kome: 100, hei: 60, jinko: 300, kokudaka: 50, machi: 60, tyu: 90 },
    { id: 7, name: "尾張", kin: 200, kome: 120, hei: 80, jinko: 350, kokudaka: 60, machi: 100, tyu: 80 },
  ],
  other_kunis: [
    { id: 1, name: "蝦夷", daimyo: { id: 1, name: "蛎崎" } },
    { id: 2, name: "奥州", daimyo: { id: 2, name: "伊達" } },
    { id: 3, name: "越州", daimyo: { id: 3, name: "上杉" } },
    { id: 4, name: "甲信", daimyo: { id: 3, name: "上杉" } },
    { id: 5, name: "武蔵", daimyo: { id: 5, name: "北条" } },
    { id: 8, name: "山城", daimyo: { id: 8, name: "足利" } },
    { id: 9, name: "安芸", daimyo: { id: 9, name: "毛利" } },
    { id: 10, name: "四国", daimyo: { id: 10, name: "長宗我部" } },
    { id: 11, name: "豊後", daimyo: { id: 11, name: "大友" } },
    { id: 12, name: "薩摩", daimyo: { id: 12, name: "島津" } },
  ],
};

/** JSON レスポンスを返す fetch のモック用ヘルパー */
export function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
