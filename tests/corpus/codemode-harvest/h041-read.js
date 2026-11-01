// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}blue lambda kappa\nlet status entry blue build\nfunction iota\nexport beta build epsilon\n}status\nlet zeta theta check north"
  },
  {
   "ok": true,
   "value": "const green lambda test\n  if (entry omega alpha check\n  if (value iota east zeta\nexport build east\n  if (green\nconst ready\nlet ready alpha value epsilon\n  if (omega beta test"
  },
  {
   "ok": true,
   "value": "const theta green iota\nconst ready\n  return red beta\n  if (east\nexport gamma iota kappa\nlet zeta item blue iota"
  },
  {
   "ok": true,
   "value": "let item east green\n  if (status eta green iota\nconst ready entry lambda beta\n}beta\nexport check south eta eta\nlet lambda value\nlet value test\n  return item omega kappa check\nconst blue record\nconst eta beta\nconst iota blue"
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
const r=await Promise.allSettled([
tools.read({path:"qet1/qave2.js",offset:35,limit:65}),
tools.read({path:"qet1/qave2.js",offset:870,limit:90}),
tools.read({path:"qet1/qave2.js",offset:1390,limit:90}),
tools.read({path:"/rigu/mato/qet3/qet4/busefonu/qet5/qot/qet6/qet7/qet8/qave9.ts",offset:1,limit:135})
]);r.forEach(x=>text(x.value??x.reason));
})(fake);
