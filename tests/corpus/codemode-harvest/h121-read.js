// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Done alpha lambda test red omega lambda ready\nDone status alpha theta omega zeta east item\n1. Record gamma\n- Lambda value east test omega kappa omega\n- Zeta north build delta check east\n## Item blue iota test green west delta east\n- Gamma entry test kappa gamma check\n# Zeta north beta blue gamma record\n## Record green alpha south\n- Alpha red delta south"
  },
  {
   "ok": true,
   "value": "let west north\nconst item blue north\n}red record west"
  },
  {
   "ok": true,
   "value": "const value build kappa omega\nlet west zeta status\n}check check south iota\nexport blue\nexport delta\nexport gamma entry blue west\nfunction gamma\nconst iota done\n}eta iota\n}eta theta\n  if (red blue south\nconst green record check\nconst west lambda"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"check\",\n    \"status\": \"open\",\n    \"count\": 301,\n    \"tags\": [\n      \"item\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"south\",\n    \"status\": \"closed\",\n    \"count\": 210,\n    \"tags\": [\n      \"omega\",\n      \"item\",\n      \"entry\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"beta\",\n    \"status\": \"closed\",\n    \"count\": 50,\n    \"tags\": [\n      \"beta\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"lambda\",\n    \"status\": \"pending\",\n    \"count\": 248,\n    \"tags\": [\n      \"entry\",\n      \"epsilon\",\n      \"east\"\n    ]\n  },\n  {\n    \"id\": 5,\n    \"name\": \"iota\",\n    \"status\": \"pending\",\n    \"count\": 19,\n    \"tags\": [\n      \"done\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "check east record red check\nitem zeta epsilon done\ndone theta beta omega red lambda record red 7378\nalpha iota alpha green entry 5875\nvalue status eta zeta omega north red theta\nnorth kappa eta red eta 1285\nvalue alpha east 6398\nepsilon done item ready 8014\nrecord omega delta iota status eta 6355\nzeta green south south green green zeta\nready record entry entry alpha\nzeta value zeta east item\nentry entry test ready gamma delta alpha\ncheck theta item"
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
const results = await Promise.allSettled([
tools.read({path:"qes1/qede2.md"}),
tools.read({path:"qes3/qede4.js", offset:1, limit:420}),
tools.read({path:"qes3/qede4.js", offset:1160, limit:400}),
tools.read({path:"ditikup.qupa"}),
tools.read({path:"qes3/qede5.qal",offset:1020,limit:270})
]); results.forEach((r,i)=>text({i,...r}));
})(fake);
