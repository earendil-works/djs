// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const south\nexport status alpha record\nconst eta eta beta check\nexport delta\nlet build done\nfunction check kappa item value\nconst ready\nexport alpha delta green"
  },
  {
   "ok": true,
   "value": "const ready done delta blue\n  return north west build\nconst west\n}value\nconst kappa\n  if (gamma kappa\nlet epsilon iota\n  if (beta theta eta\nexport north green\nfunction east\n  return value zeta test\nconst blue\n}south omega zeta"
  },
  {
   "ok": true,
   "value": "export eta south status\nlet east item status delta\n  return theta delta red"
  },
  {
   "ok": true,
   "value": "function south\nfunction kappa test delta\nexport west\nlet gamma delta lambda red\n  if (value done gamma\n  if (kappa\nexport north red kappa\nconst south iota lambda\n  return eta\n  return zeta record zeta"
  },
  {
   "ok": true,
   "value": "item east omega\nitem red\neast delta check item build zeta 1615\nentry north lambda\nnorth east status\nnorth build theta item test blue\neta west record 2358\nsouth lambda north\nnorth zeta blue zeta zeta ready epsilon south 5125"
  },
  {
   "ok": true,
   "value": "- Alpha ready kappa status green alpha north\nValue lambda south test theta\n# South done eta value north south alpha entry\n1. Beta gamma\n## Record entry\n# Kappa lambda zeta green south\n- Status record entry\n- Blue iota epsilon done north status east test"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"check\",\n    \"status\": \"ok\",\n    \"count\": 329,\n    \"tags\": [\n      \"item\",\n      \"south\",\n      \"kappa\"\n    ]\n  }\n]"
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
tools.read({path:"meb1/qata2.js",offset:650,limit:165}),
tools.read({path:"meb1/qata2.js",offset:950,limit:108}),
tools.read({path:"meb1/qata2.js",offset:1160,limit:90}),
tools.read({path:"meb1/qata2.js",offset:1530,limit:130}),
tools.read({path:"meb1/qata3.bab",offset:1190,limit:210}),
tools.read({path:"meb4/qata5.md",offset:64,limit:35}),
tools.read({path:"gupefom.qave",offset:null,limit:null})
]);r.forEach(x=>text(x.value??x.reason));
})(fake);
