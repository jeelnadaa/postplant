import { describe, expect, it } from "vitest";
import { hms, kd, mmss, parseRiotId, signed, tierClass, tierName, timeAgo, toMinimap } from "./format";

describe("ranks", () => {
  it("maps tier ids to names and classes", () => {
    expect(tierName(3)).toBe("Iron 1");
    expect(tierName(19)).toBe("Diamond 2");
    expect(tierName(19, true)).toBe("D2");
    expect(tierName(27)).toBe("Radiant");
    expect(tierName(0)).toBe("Unranked");
    expect(tierClass(21)).toBe("ascendant");
    expect(tierClass(27)).toBe("radiant");
  });
});

describe("riot ids", () => {
  it("parses name#tag", () => {
    expect(parseRiotId("Nova#ap1")).toEqual({ name: "Nova", tag: "ap1" });
    expect(parseRiotId("Two Words # tag")).toEqual({ name: "Two Words", tag: "tag" });
    expect(parseRiotId("no-tag")).toBeNull();
    expect(parseRiotId("x#toolong")).toBeNull();
  });
});

describe("numbers and time", () => {
  it("formats", () => {
    expect(kd(21, 14)).toBe("1.50");
    expect(signed(21)).toBe("+21");
    expect(signed(-5)).toBe("-5");
    expect(mmss(71_000)).toBe("1:11");
    expect(hms(3725)).toBe("1:02:05");
    expect(timeAgo(1000, 1000 + 5 * 60_000)).toBe("5 min ago");
  });
});

describe("minimap", () => {
  it("swaps axes like valorant-api.com", () => {
    const mm = { x_multiplier: 0.0001, y_multiplier: -0.0001, x_scalar: 0.5, y_scalar: 0.4 };
    expect(toMinimap(mm, 1000, 2000)).toEqual([0.7, 0.30000000000000004]);
    expect(toMinimap(null, 1, 2)).toBeNull();
  });
});
