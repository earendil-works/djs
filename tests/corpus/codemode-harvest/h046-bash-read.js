// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const names = ALL_TOOLS.filter(x => /artifacts/.test(typeof x === "string" ? x : x.name)).map(x=>typeof x==="string"?x:x.name); text(names);
text(await tools.bash({command:"lova_kufigak/nel1/nel2 /nos/nel3/nel4/*.qid && zas zoba --noruno /nos/nel3/nel4",timeout:120}));
text(await tools.read({path:"/nos/nel3/nel4/veku5.qum",offset:25,limit:22}));
})(fake);
