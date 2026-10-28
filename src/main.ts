import { createApp } from "vue";
import App from "./App.vue";
import "@fontsource/vazirmatn/latin-400.css";
import "@fontsource/vazirmatn/latin-600.css";
import "@fontsource/vazirmatn/arabic-400.css";
import "@fontsource/vazirmatn/arabic-600.css";
import "./style.css";
import { applyLang } from "./lib/i18n";

applyLang();

createApp(App).mount("#app");
