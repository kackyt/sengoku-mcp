import { describe, expect, it } from "vitest";
import { turnToDate } from "./gameDate";

describe("turnToDate", () => {
  it.each([
    [1, 1560, "春"],
    [2, 1560, "夏"],
    [4, 1560, "冬"],
    [5, 1561, "春"],
    [11, 1562, "秋"],
  ])("第%iターンは %i年%s", (turn, year, season) => {
    expect(turnToDate(turn)).toEqual({ year, season });
  });

  it("0以下のターンは第1ターンとして扱う", () => {
    expect(turnToDate(0)).toEqual({ year: 1560, season: "春" });
  });
});
