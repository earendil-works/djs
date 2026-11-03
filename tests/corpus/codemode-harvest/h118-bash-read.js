// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "entry omega delta north south west\ndone check gamma epsilon entry gamma 7609\niota gamma test\ntest north\nready beta build delta iota check\neta north check alpha south test test 777\nzeta iota ready value zeta omega blue\nlambda test test\nalpha status value zeta gamma beta entry check\ngreen item blue blue delta check alpha north\nentry zeta east\nepsilon value build south eta entry alpha 6525\nbeta entry omega epsilon done\nbuild alpha eta eta check"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "let test west omega\n  if (blue entry east zeta\nexport epsilon blue done\nlet red item\nexport kappa green\nfunction build iota\nconst kappa beta ready entry"
  },
  {
   "ok": true,
   "value": "  if (south ready\nexport check\nexport zeta check beta lambda\n}value kappa test\nexport blue\nfunction record lambda\nfunction west\nexport gamma omega delta\nexport north blue red\nlet eta eta\nlet north check zeta\nconst status test record build"
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
tools.bash({command:"puk tesu -- mulaluke/tuf1/ziv/tuf2/tuf3/volun.ts mulaluke/tuf1/dobo4.md",timeout:null}),
tools.read({path:"mulaluke/tuf1/ziv/tuf2/tuf3/volun.ts",offset:null,limit:null}),
tools.read({path:"mulaluke/tuf1/qale/tuf5/tuf6/dobo7.ts",offset:null,limit:null})
]);for(let i=0;i<results.length;i++)text({i,result:results[i]});
})(fake);
