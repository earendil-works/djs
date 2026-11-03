// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return north\n  if (south east lambda\nfunction check status delta\n  return red alpha omega check\n}test\nfunction south\n  if (lambda delta iota\n  return eta zeta alpha"
  },
  {
   "ok": true,
   "value": "}south north north kappa\nlet north lambda\nexport epsilon status green kappa\n  return lambda gamma\n  return alpha blue alpha west\nfunction entry epsilon item beta\nfunction value done red north\nconst entry west red"
  },
  {
   "ok": true,
   "value": "export alpha\nconst north item iota\nfunction zeta status ready green\n}south delta build\nfunction record east omega blue\nconst iota test west beta\n}eta test\n  return test done\nexport status red\n  return delta green theta delta\nconst west iota\nlet lambda west eta\nexport eta build red\n}red gamma blue"
  },
  {
   "ok": true,
   "value": "let build entry item\n}test\nexport blue build green theta"
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
 tools.read({path:"rulikibi/qog1/qun/qog2/qog3/rivig.ts"}),
 tools.read({path:"rulikibi/qog1/qun/qog2/qog3/qape4.ts"}),
 tools.read({path:"rulikibi/qog1/qun/neda/qog5/qape6.ts"}),
 tools.read({path:"rulikibi/qog1/qun/neda/qog5/qape7.ts"}),
 tools.bash({command:"sul qat --nimurof -15 -- rulikibi/qog1/qun/neda/qog5/qape6.ts rulikibi/qog1/qun/neda/qape8.ts; rg -n 'nedabe|mimimuveranNubopeku' rulikibi/qog1/qun/qog9 rulikibi/qog1/qun/neda/qape8.ts rulikibi/qog1/zibu/qape10.ts; rg -n '^rezile voriritis VoziMelu |nedabe\\??:|goto\\??: .vurapo' tada_fodugin/qog11/qog12/qog13/qog14/qog15/qape16.ts | dadi -30; rg -n 'benobetu|mofabKugikukeg|difavu' rulikibi/qog1/qun/neda/qape17.ts",timeout:30})
]);for(let i=0;i<results.length;i++){text(i);if(results[i].status==="fulfilled")text(results[i].value);else text(String(results[i].reason));}
})(fake);
