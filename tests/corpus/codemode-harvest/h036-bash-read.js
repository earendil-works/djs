// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "  if (north eta\nconst alpha build\nexport build item status\nexport entry blue\n  return ready check omega\n  return blue lambda\n  return kappa omega kappa eta\n  return omega check gamma green\nfunction beta\n}epsilon lambda value\n  return ready red status\n  return blue east gamma alpha\n  return done status west eta\nexport ready value east"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
text(await tools.bash({command:"nem tubasu --qamer; rg -n 'QupVititofMula|QUP nimeron|Konunovive…|Moladezukasem…|kebokeg-todasut|qup vititof' pefutofo/qig1/sese; rg -n 'veguf MoguzeZuvi|qelNarugegeDugeb|onGivazinulVikote' pefutofo/qig2/qet/qig3/qina4.ts; wc -l pefutofo/qig1/sese/qina5.ts pefutofo/qig1/qet/qig6/qig7/qina8.ts pefutofo/qig7/qet/qig9/qina10.ts pefutofo/qig2/qet/qig3/qina4.ts",timeout:30}));
text(await tools.read({path:"pefutofo/qig1/qet/qig6/qig7/qina8.ts",offset:null,limit:null}));
text(await tools.read({path:"pefutofo/qig7/qet/qig9/qina10.ts",offset:null,limit:null}));
})(fake);
