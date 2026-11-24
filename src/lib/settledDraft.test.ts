import { afterEach, expect, it, vi } from "vitest";
import { settleDraft } from "./settledDraft";

afterEach(() => vi.useRealTimers());

it("holds the last recommendation through hover changes and publishes a lock immediately", () => {
  vi.useFakeTimers();
  const publish = vi.fn();
  const receive = settleDraft(publish);
  const pick = (agent: string, locked = false) => [{ agent, locked }];
  receive(pick("Jett"));
  receive(pick("Reyna"));
  vi.advanceTimersByTime(1500);
  receive(pick("Raze"));
  vi.advanceTimersByTime(1500);
  expect(publish.mock.calls).toEqual([[pick("Jett")]]);
  vi.advanceTimersByTime(500);
  expect(publish).toHaveBeenLastCalledWith(pick("Raze"));
  receive(pick("Omen"));
  receive(pick("Omen", true));
  expect(publish).toHaveBeenLastCalledWith(pick("Omen", true));
  vi.runAllTimers();
  expect(publish).toHaveBeenCalledTimes(3);
});

it("clears pending hovers when leaving agent select", () => {
  vi.useFakeTimers();
  const publish = vi.fn();
  const receive = settleDraft(publish);
  receive([{ agent: "Jett" }]);
  receive([{ agent: "Raze" }]);
  receive();
  vi.runAllTimers();
  expect(publish).toHaveBeenLastCalledWith(undefined);
  expect(publish).toHaveBeenCalledTimes(2);
});
