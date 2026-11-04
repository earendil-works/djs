import { toPlain } from "./plain.mjs"; import fs from "node:fs";
let ok = 0, hok = 0, htot = 0, bad = {};
fs.mkdirSync("plain", { recursive: true });
for (const f of fs.readdirSync("today").filter((f) => f.endsWith(".dump.js"))) {
  const h = f.startsWith("h"); if (h) htot++;
  try { const r = toPlain(fs.readFileSync("today/" + f, "utf8")); fs.writeFileSync("plain/" + f, r.text); ok++; if (h) hok++; } catch (e) { const k = (h ? "H " : "") + e.message.replace(/\d+/g, "N").slice(0, 60); bad[k] = (bad[k] || 0) + 1; if (!e.constructor.name.includes("Refuse")) { console.log(f, e.stack.split("\n").slice(0,3).join("\n")); } }
}
console.log(ok, `harvested ${hok}/${htot}`, bad);
