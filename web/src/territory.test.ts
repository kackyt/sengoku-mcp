import { describe, expect, it } from "vitest";
import { sampleStatus } from "./test/fixtures";
import { buildTerritories, countByOwner } from "./territory";

describe("buildTerritories", () => {
  it("自領と他国を全12国の勢力図にまとめる", () => {
    const territories = buildTerritories(sampleStatus);
    expect(territories.size).toBe(12);
    expect(territories.get(6)).toEqual({
      kuniId: 6,
      kuniName: "三河",
      owner: { id: 7, name: "織田" },
      mine: true,
    });
    expect(territories.get(4)?.owner.name).toBe("上杉");
    expect(territories.get(4)?.mine).toBe(false);
  });

  it("大名ごとの領地数を数える", () => {
    const counts = countByOwner(buildTerritories(sampleStatus).values());
    expect(counts.get(7)).toBe(2);
    expect(counts.get(3)).toBe(2);
    expect(counts.get(1)).toBe(1);
  });
});
