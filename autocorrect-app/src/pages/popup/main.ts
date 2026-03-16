import { mount } from "svelte";
import "../../app.css";
import Popup from "./Popup.svelte";
import { initI18n } from "$lib/i18n/setup";

initI18n();

const app = mount(Popup, {
  target: document.getElementById("app")!,
});

export default app;
