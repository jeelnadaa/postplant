<script lang="ts">
  import type { J } from "../lib/api";
  import { fmtDate, kd, queueLabel, signed } from "../lib/format";
  import { nav } from "../lib/store.svelte";
  import Agent from "./Agent.svelte";
  import Icon from "./Icon.svelte";
  let { m }: { m: J } = $props();
  const cls = $derived(m.result === "W" ? "w" : m.result === "L" ? "l" : "d");
</script>

<button class="match-row {cls}" style="width:100%;text-align:left" onclick={() => nav("match", { id: m.id })}>
  <span class="res"></span>
  <Agent name={m.agent} id={m.agent_id} sm />
  <div style="min-width:0"><b>{m.map}</b><div class="faint" style="font-size:12px">{queueLabel(m.queue)} · {fmtDate(m.started_at)}</div></div>
  <span class="score">{m.score[0]}–{m.score[1]}</span>
  <div><b class="mono">{m.k}/{m.d}/{m.a}</b><div class="faint" style="font-size:11px">K/D {kd(m.k, m.d)}</div></div>
  <div><b class="mono">{m.acs}</b><div class="faint" style="font-size:11px">ACS</div></div>
  <div><b class="mono">{m.hs}%</b><div class="faint" style="font-size:11px">HS</div></div>
  <div>{#if m.rr !== null && m.rr !== undefined}<span class="badge {m.rr > 0 ? 'win' : m.rr < 0 ? 'loss' : ''}">{signed(m.rr)} RR</span>{/if}</div>
  <Icon name="chevron-right" cls="faint" />
</button>
