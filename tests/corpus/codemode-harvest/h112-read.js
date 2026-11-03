// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "ready blue blue west record gamma alpha 9821\nblue build iota\neast east test done\nlambda east value eta\ngamma kappa iota record alpha value north\ntest green zeta beta kappa delta red beta 7624\ntheta gamma gamma entry entry blue\niota east green\nkappa check theta omega epsilon entry\nitem beta delta eta\nwest lambda omega check test alpha blue ready 7454\ncheck theta north build zeta north beta\neast blue 9675\nomega done epsilon done gamma item"
  },
  {
   "ok": true,
   "value": "  if (north test record red\nconst kappa blue\n  if (blue done status\nconst north lambda\n  return eta delta theta status\nlet west zeta lambda\nconst red build alpha theta\n  if (theta\n  return delta ready south\n  return blue alpha eta\nconst green green beta iota\nlet kappa status entry"
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
await Promise.allSettled([["public/file1.html",125,20],["public/file2.js",708,47]].map(async ([path,offset,limit])=>text({path,content:await tools.read({path,offset,limit})})));
})(fake);
