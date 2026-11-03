// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function epsilon\n  if (red zeta\nconst entry\n  if (alpha value item beta\nconst lambda\nconst eta ready\nexport beta\nexport beta\n}epsilon gamma test south"
  },
  {
   "ok": true,
   "value": "  if (test lambda\nfunction epsilon west alpha\nlet check record west delta\n  if (item\n}north east\nexport delta entry green epsilon\n}lambda lambda done\nfunction done iota alpha\nexport theta beta west\nexport eta lambda\n  return item lambda"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file8.ts:351: eta entry check done iota epsilon\nsrc/file1.ts:390: eta value iota east\nsrc/file3.ts:227: entry status\nsrc/file2.ts:105: epsilon check omega beta status alpha\nsrc/file8.ts:343: kappa lambda item record alpha\nsrc/file7.ts:219: omega lambda blue\nsrc/file9.ts:33: ready north east\nsrc/file4.ts:254: omega omega value north status epsilon\nsrc/file4.ts:218: check gamma eta omega"
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
const [a,b,c] = await Promise.all([
tools.read({path:'qum/qun1/kito2.js',offset:null,limit:null}),
tools.read({path:'qum/qun3/kito4.js',offset:null,limit:null}),
tools.bash({command:"tas SUZISE.md | pavo -80; tas pizemul.fesi 2>/qun5/qun6; ls; rg -n 'gatolol' qum | pavo -40"}),
]);
text(a);text(b);text(c.output);
})(fake);
