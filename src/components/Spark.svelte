<script lang="ts">
  // Hand-written SVG line chart with an optional dashed baseline (no chart library needed).
  let { values, base = null, h = 90, labels = [] as string[] }: { values: number[]; base?: number | null; h?: number; labels?: string[] } = $props();
  const W = 300;
  const pts = $derived.by(() => {
    if (!values.length) return { d: "", area: "", y: () => 0, dots: [] as [number, number][] };
    const all = base !== null ? [...values, base] : values;
    const lo = Math.min(...all) * 0.95, hi = Math.max(...all) * 1.05 || 1;
    const y = (v: number) => h - 8 - ((v - lo) / (hi - lo || 1)) * (h - 16);
    const x = (i: number) => (values.length === 1 ? W / 2 : (i / (values.length - 1)) * W);
    const dots: [number, number][] = values.map((v, i) => [x(i), y(v)]);
    const d = dots.map(([a, b], i) => `${i ? "L" : "M"}${a.toFixed(1)} ${b.toFixed(1)}`).join(" ");
    return { d, area: `${d} L${W} ${h} L0 ${h} Z`, y, dots };
  });
</script>

<svg class="chart" viewBox="0 0 {W} {h}" preserveAspectRatio="none" style="height:{h}px" role="img" aria-label="Trend">
  <defs><linearGradient id="gradA" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--chart-1)" stop-opacity=".35" /><stop offset="1" stop-color="var(--chart-1)" stop-opacity="0" /></linearGradient></defs>
  <g class="grid"><line x1="0" y1={h - 1} x2={W} y2={h - 1} /></g>
  {#if values.length > 1}<path class="area" d={pts.area} />{/if}
  {#if base !== null && values.length}<line class="line base" x1="0" x2={W} y1={pts.y(base)} y2={pts.y(base)} />{/if}
  {#if values.length}<path class="line" d={pts.d} vector-effect="non-scaling-stroke" />{/if}
  {#each labels as l, i (i)}<text class="axis" x={i * (W / Math.max(1, labels.length - 1))} y={h - 2} font-size="9">{l}</text>{/each}
</svg>
