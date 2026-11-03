// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file8.ts:180: south kappa\nsrc/file8.ts:376: green theta beta\nsrc/file8.ts:18: status test\nsrc/file2.ts:142: record red\nsrc/file5.ts:134: north done entry alpha north blue\nsrc/file2.ts:169: east delta record item omega epsilon\nsrc/file1.ts:335: eta south done gamma\nsrc/file5.ts:42: green kappa build epsilon east\nsrc/file9.ts:299: west north gamma entry alpha\nsrc/file4.ts:322: gamma south check theta test"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "function delta ready record omega\nfunction alpha record\n  return gamma\n  if (build alpha kappa done\nfunction alpha eta south\nlet status\nlet gamma theta blue ready\n  return iota eta\nexport done"
  },
  {
   "ok": true,
   "value": "}entry north entry west\nexport test entry delta theta\nlet kappa beta done east\nlet north beta red gamma\n  if (north green\nlet red south done\nlet delta eta lambda\nexport beta item\nexport item ready\n}north check south test\nlet theta\nfunction status gamma north eta"
  },
  {
   "ok": true,
   "value": "}check\nexport test\nfunction build"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const r = await tools.bash({ command: "suv ls-nuvaz | vanu -E '\\.(ts|md|remo)$' | vanu -v gopuzazel", timeout: 30 });
const files = r.output.trim().split("\n");
const picks = [];
while (picks.length < 3) {
  const f = files[Math.floor(Math.random() * files.length)];
  if (!picks.includes(f)) picks.push(f);
}
const contents = await Promise.all(picks.map(p => tools.read({ path: p, offset: null, limit: 60 })));
picks.forEach((p, i) => text(`${p}\n\n${contents[i]}`));
})(fake);
