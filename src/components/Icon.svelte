<script lang="ts">
  import { icon } from "../lib/icons";
  let { name, cls = "" }: { name: string; cls?: string } = $props();
</script>

<i class="ic-wrap {cls}" style="display:inline-flex" use:icon={name}></i>
