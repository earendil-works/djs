// The browser half of the smoke test: the result goes to `window.smokeResult`, which
// js/pkg-test/browser.js waits for.
import { smoke } from "./smoke.js";

smoke().then(
  (lines) => (window.smokeResult = { ok: true, lines }),
  (e) => (window.smokeResult = { ok: false, error: String(e?.stack ?? e) }),
);
