import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { initI18n } from "$lib/i18n/setup";

initI18n();

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
