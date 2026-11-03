// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "check ready ready check test eta iota kappa 5981\nomega record green 7605\nalpha status beta iota status green\ngamma status test check delta build\ndone lambda blue beta status kappa gamma east\nlambda lambda theta lambda\nkappa item entry done iota\neast build 9726\ncheck west entry alpha lambda record\nzeta ready alpha value theta theta south\nentry check gamma 755"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "let west ready ready\n  return north south omega test\nlet omega blue test\n  return ready epsilon item build\nlet build alpha north check\nconst lambda entry\nlet north value epsilon value\n  return green check entry theta\nconst west epsilon build omega\nconst south east entry\nfunction lambda lambda iota record\nfunction record beta\nexport zeta iota"
  },
  {
   "ok": true,
   "value": "function epsilon blue theta red\nexport delta blue red ready\nexport omega test\nlet omega red green\nlet red value entry iota\n  if (zeta iota"
  },
  {
   "ok": true,
   "value": "let item red kappa\n  return west\nlet south eta\n  if (gamma green alpha\n  return alpha value\nfunction ready north item\nconst gamma theta beta\nlet red iota south east\nexport east value zeta\n  return green red\nconst item gamma\n}item blue gamma south\nfunction ready gamma\nlet north"
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
text(await tools.bash({command:"biv kenuse --fogev; ls nopag-gerodafe-kunu; rg -n 'qib|zotob' qad1/rizab/bupa2.rs nopag-{tako,gokalud}/rizab/bupa2.rs; rg -n 'qib_vutupon|Pasifa|pila_qib' qad3/kut/bupa4.rs qad1/kut/bupa5.rs qad6/kut/bupa5.rs qad7/kut/bupa5.rs; rg -n 'sazudegi' Bitusazi qad1/bupa8.py; biv vig -5 --nuronuv",timeout:null}));
await Promise.allSettled(["dir3/src/dir9/file10.rs","dir1/tests/file2.rs","dir1/file11.rs"].map(async path=>text({path,content:await tools.read({path,offset:null,limit:null})})));
})(fake);
