// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return theta gamma\nconst entry\nfunction eta\n}beta theta done\n  return done\nexport build record eta ready\n}theta theta east\nconst build east\n  return eta ready eta\n}delta zeta check lambda"
  },
  {
   "ok": true,
   "value": "function test\nconst build west west beta\n  return ready west omega blue\nlet blue omega blue\n  if (beta\nconst zeta gamma\nfunction west red\nexport test"
  },
  {
   "ok": true,
   "value": "const lambda ready\n  return delta value\nlet value\nconst status eta kappa green\nexport theta epsilon beta\n}done\nconst gamma\nlet north west\nexport item iota\nlet epsilon blue red\n  if (red\nfunction theta lambda ready\n  if (check lambda omega\nlet gamma"
  }
 ],
 "bash": [
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
const results = await Promise.allSettled([
tools.read({path:"nuzukofe/sas1/qabe/lalu2.ts",offset:null,limit:null}),
tools.read({path:"nuzukofe/sas1/qabe/lalu3.ts",offset:null,limit:null}),
tools.read({path:"nuzukofe/sas1/raf/suzib/lalu4.ts",offset:null,limit:null}),
tools.bash({command:"rg -n '\"sas5/sas6\"|\"ws\"' robabad.fapa nuzukofe/sas7/robabad.fapa robabad-luko.fapa; rg -n 'netivelQarKur' nuzukofe/sas1/raf/lalu8.ts nuzukofe/sas1/raf/sas9*; qip qena --bufe; ls kiko_vokenot/sas5/sas6",timeout:30})
]); for(let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);
