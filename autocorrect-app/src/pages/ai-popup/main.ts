import "../../app.css";
import { mount } from "svelte";
import AiPopup from "./AiPopup.svelte";
import { initI18n } from "$lib/i18n/setup";

initI18n();

mount(AiPopup, { target: document.getElementById("app")! });
