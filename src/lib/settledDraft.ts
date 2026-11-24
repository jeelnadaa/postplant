type Pick = { locked?: boolean; agent?: string | null };

export function settleDraft<T extends Pick>(publish: (players: T[] | undefined) => void) {
  let previous: T[] | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  return (players?: T[]) => {
    clearTimeout(timer);
    const immediate = !players || !previous || players.length !== previous.length ||
      players.some((p, i) => p.locked !== previous?.[i]?.locked ||
        (p.locked && p.agent !== previous?.[i]?.agent));
    previous = players;
    if (immediate) publish(players);
    else timer = setTimeout(() => publish(players), 2000);
  };
}
