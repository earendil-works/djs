// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const alpha test south theta\nlet beta theta theta\n  if (record\nfunction entry\nfunction epsilon\n  if (gamma north build\n}west iota\nconst test blue\n  return ready eta value south\nlet theta epsilon"
  },
  {
   "ok": true,
   "value": "function ready item ready alpha\nfunction status test lambda red\nlet done blue eta\n  if (gamma epsilon test\nlet green\nlet kappa alpha\nlet item eta\nlet alpha\nexport epsilon entry done\nexport epsilon zeta kappa\n  return omega\n  return north test item"
  },
  {
   "ok": true,
   "value": "north record omega kappa\nrecord north red epsilon\nblue red done red test red\nitem value green theta delta blue\neast value beta eta theta red iota\neta lambda record blue red east omega beta 4985\nzeta west build 2780"
  },
  {
   "ok": true,
   "value": "}north zeta iota status\nfunction zeta build record entry\n  return south east build\nconst item build\nconst record\nexport green done gamma\n}status lambda\nconst value build\n}ready blue\n}record iota\nlet gamma alpha"
  },
  {
   "ok": true,
   "value": "west record blue status red east test\ncheck west lambda item lambda entry beta gamma\niota gamma check\nvalue status item west delta value done kappa\nomega east record gamma zeta check\ndone south item eta kappa 2000\niota iota item red iota east kappa value\nzeta check test delta alpha\nbuild done done lambda eta record gamma\nstatus zeta\ngreen south kappa beta green"
  },
  {
   "ok": true,
   "value": "blue record lambda gamma epsilon item\nred south alpha kappa omega 1654\nzeta item ready delta beta alpha entry\nrecord eta omega green\ncheck item zeta value west\ntheta south red"
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
const paths=["src/file1.ts","src/file2.ts","public/file3.html","public/file4.js","public/file5.css","wrangler.vatog"]; await Promise.allSettled(paths.map(async path=>text({path,content:await tools.read({path,offset:null,limit:null})})));
})(fake);
