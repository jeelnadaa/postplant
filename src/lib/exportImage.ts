// Turn an on-screen map (heatmap or replay SVG) into a PNG to save or copy.
// Styles come from stylesheets and CSS variables, which don't travel with a serialized SVG, so the
// computed paint of every element is copied inline first, and the minimap image is embedded.
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "./api";

const PROPS = [
  "fill", "fill-opacity", "stroke", "stroke-width", "stroke-opacity", "stroke-linecap", "stroke-linejoin", "opacity",
  "font-size", "font-family", "font-weight", "paint-order", "stop-color", "stop-opacity", "text-anchor", "display", "visibility",
];

async function toDataUrl(url: string): Promise<string> {
  if (url.startsWith("data:")) return url;
  // Loaded as an image (allowed by the app's img-src) with CORS, then copied through a canvas.
  const im = new Image();
  im.crossOrigin = "anonymous";
  await new Promise<void>((res, rej) => {
    im.onload = () => res();
    im.onerror = () => rej(new Error("Couldn't load the map image."));
    // A separate URL: the copy on screen was cached without CORS headers and can't be reused here.
    im.src = url + (url.includes("?") ? "&" : "?") + "export=1";
  });
  const c = document.createElement("canvas");
  c.width = im.naturalWidth;
  c.height = im.naturalHeight;
  c.getContext("2d")!.drawImage(im, 0, 0);
  return c.toDataURL("image/png");
}

/** Render `svg` to a PNG blob `size` pixels wide, with a title strip at the bottom. */
export async function svgToPng(svg: SVGSVGElement, title: string, size = 1600): Promise<Blob> {
  const clone = svg.cloneNode(true) as SVGSVGElement;
  const src = [svg, ...svg.querySelectorAll("*")] as Element[];
  const dst = [clone, ...clone.querySelectorAll("*")] as Element[];
  src.forEach((el, i) => {
    const cs = getComputedStyle(el);
    const style = PROPS.map((p) => `${p}:${cs.getPropertyValue(p)}`).join(";");
    dst[i].setAttribute("style", style);
  });
  for (const img of clone.querySelectorAll("image")) {
    const href = img.getAttribute("href") ?? "";
    if (href) img.setAttribute("href", await toDataUrl(href));
  }
  const vb = svg.viewBox.baseVal;
  const w = vb?.width || 1000;
  const h = vb?.height || 1000;
  clone.setAttribute("width", String(w));
  clone.setAttribute("height", String(h));
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  const xml = new XMLSerializer().serializeToString(clone);
  // A data: URL, since the app's content policy allows data: images but not blob: ones.
  const url = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(xml);
  {
    const im = new Image();
    await new Promise<void>((res, rej) => {
      im.onload = () => res();
      im.onerror = () => rej(new Error("Couldn't draw the image."));
      im.src = url;
    });
    const root = getComputedStyle(document.documentElement);
    const bg = root.getPropertyValue("--bg").trim() || "#0a0b0e";
    const fg = root.getPropertyValue("--text").trim() || "#ede8df";
    const scale = size / w;
    const strip = 64;
    const c = document.createElement("canvas");
    c.width = size;
    c.height = Math.round(h * scale) + strip;
    const g = c.getContext("2d")!;
    g.fillStyle = bg;
    g.fillRect(0, 0, c.width, c.height);
    g.drawImage(im, 0, 0, size, h * scale);
    g.fillStyle = fg;
    g.font = "600 26px Satoshi, Inter, sans-serif";
    g.textBaseline = "middle";
    g.fillText(title, 28, c.height - strip / 2);
    g.globalAlpha = 0.6;
    g.textAlign = "right";
    g.fillText("Postplant", size - 28, c.height - strip / 2);
    return await new Promise((res, rej) => c.toBlob((b) => (b ? res(b) : rej(new Error("Couldn't make the PNG."))), "image/png"));
  }
}

/** Ask where to save, then write the PNG. Returns the path, or null if cancelled. */
export async function savePng(blob: Blob, name: string): Promise<string | null> {
  const path = await save({ defaultPath: `${name.replace(/[^\w.-]+/g, "-")}.png`, filters: [{ name: "PNG image", extensions: ["png"] }] });
  if (!path) return null;
  const buf = new Uint8Array(await blob.arrayBuffer());
  let bin = "";
  for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
  await api.savePng(path.toLowerCase().endsWith(".png") ? path : `${path}.png`, btoa(bin));
  return path;
}

export async function copyPng(blob: Blob): Promise<void> {
  await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
}
