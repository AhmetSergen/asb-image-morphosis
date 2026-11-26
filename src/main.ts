// SPDX-License-Identifier: GPL-3.0-or-later
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

const app = mount(App, { target: document.getElementById("app")! });

// Dev builds only: lets automated UI checks drive the app state.
if (import.meta.env.DEV) {
  import("./lib/state.svelte").then((m) => {
    (window as unknown as { __morphosis: unknown }).__morphosis = m.app;
  });
}

export default app;
