<script lang="ts">
  // "Save PNG" and "Copy" for an on-screen map (heatmap or replay).
  import { errText } from "../lib/api";
  import { copyPng, savePng, svgToPng } from "../lib/exportImage";
  import { t } from "../lib/i18n.svelte";
  import { toast } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  let { svg, title, name }: { svg: SVGSVGElement | undefined; title: string; name: string } = $props();
  let busy = $state(false);

  async function run(kind: "save" | "copy") {
    if (!svg || busy) return;
    busy = true;
    try {
      const blob = await svgToPng(svg, title);
      if (kind === "copy") {
        await copyPng(blob);
        toast("ok", t("Image copied"));
      } else {
        const p = await savePng(blob, name);
        if (p) toast("ok", t("Image saved"), p);
      }
    } catch (e) {
      toast("err", t("Couldn't export the image"), errText(e));
    }
    busy = false;
  }
</script>

<div class="row" style="gap:6px">
  <button class="btn btn-sm {busy ? 'is-loading' : ''}" onclick={() => run("save")} title={t("Save as PNG")}><Icon name="download" />{t("Save PNG")}</button>
  <button class="btn btn-sm btn-ghost" onclick={() => run("copy")} title={t("Copy image")}><Icon name="copy" />{t("Copy")}</button>
</div>
