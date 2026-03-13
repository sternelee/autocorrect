import { mount } from "svelte";
import "../../app.css";
import Widget from "./Widget.svelte";

const app = mount(Widget, {
  target: document.getElementById("app")!,
});

export default app;
