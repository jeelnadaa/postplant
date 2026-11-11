import "@fontsource/big-shoulders-display/900";
import "@fontsource/jetbrains-mono/600";
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/components.css";
import "./styles/app.css";
import { mount } from "svelte";
import Overlay from "./Overlay.svelte";

mount(Overlay, { target: document.getElementById("overlay")! });
