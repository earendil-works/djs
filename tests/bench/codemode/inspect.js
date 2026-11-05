// No tools: look through the tool catalog and do a small computation (about 9%).
const byNs = {};
for (const t of ALL_TOOLS) {
  const ns = t.name.includes("__") ? t.name.split("__")[0] : "(core)";
  (byNs[ns] ??= []).push(t.name);
}
text(Object.entries(byNs).map(([ns, names]) => `${ns}: ${names.length}`).join("\n"));
const matches = ALL_TOOLS.filter((t) => /report|metric/i.test(t.name + " " + t.description)).map((t) => t.name);
const sizes = [1280 * 720, 1920 * 1080, 3840 * 2160].map((px) => (px * 4 / 1024 / 1024).toFixed(1) + " MiB");
return { matches, sizes, total: ALL_TOOLS.length };
