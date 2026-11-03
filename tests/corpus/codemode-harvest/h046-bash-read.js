// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "omega eta green blue\nstatus south east beta\ndelta status blue done\nwest value omega test blue\nbuild omega eta omega gamma iota entry\nvalue entry build entry zeta done east\nbeta beta zeta south 1352\ntheta kappa done kappa done delta test theta 3441"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "1. North check status value check omega green\nEast omega entry lambda\n- Check west kappa test blue theta\n# Red blue eta omega\n# North west build item blue entry\n## Beta omega check west epsilon kappa\n## Alpha gamma theta beta blue kappa green\n# Value ready blue build gamma gamma\n# South red"
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
const names = ALL_TOOLS.filter(x => /artifacts/.test(typeof x === "string" ? x : x.name)).map(x=>typeof x==="string"?x:x.name); text(names);
text(await tools.bash({command:"lova_kufigak/nel1/nel2 /nos/nel3/nel4/*.qid && zas zoba --noruno /nos/nel3/nel4",timeout:120}));
text(await tools.read({path:"/nos/nel3/nel4/veku5.qum",offset:25,limit:22}));
})(fake);
