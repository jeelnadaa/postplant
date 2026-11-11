import "@fontsource/big-shoulders-display/700";
import "@fontsource/big-shoulders-display/900";
import "@fontsource/jetbrains-mono/400";
import "@fontsource/jetbrains-mono/600";
import "@fontsource/inter/400";
import "@fontsource/inter/600";
import "@fontsource/geist-sans/400";
import "@fontsource/geist-sans/600";
import "@fontsource/atkinson-hyperlegible/400";
import "@fontsource/atkinson-hyperlegible/700";
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/components.css";
import "./styles/app.css";
import { mount } from "svelte";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
