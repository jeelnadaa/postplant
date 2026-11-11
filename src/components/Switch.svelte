<script lang="ts">
  let { checked = $bindable(false), label = "", danger = false, onchange = undefined }: { checked: boolean; label?: string; danger?: boolean; onchange?: (v: boolean) => void } = $props();
</script>

<button type="button" class="switch {danger ? 'danger' : ''}" role="switch" aria-checked={checked} aria-label={label}
  onclick={() => { checked = !checked; onchange?.(checked); }}></button>
