// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function value kappa\n  return test\n}lambda\n  if (green value entry\nconst iota record done zeta"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"lambda\",\n      \"status\": \"open\",\n      \"count\": 351,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"ready\",\n      \"status\": \"pending\",\n      \"count\": 309,\n      \"tags\": [\n        \"east\",\n        \"iota\"\n      ]\n    }\n  ],\n  \"total\": 2\n}"
  },
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": true,
   "value": "- Done record\nEpsilon check item epsilon zeta iota gamma\n## Check south build north delta kappa green\n1. Omega entry eta delta red zeta east entry\n# Epsilon item theta entry item record eta beta\n1. Check entry record test done green\n- Green epsilon alpha entry south\n- Delta lambda red omega value\n# Eta zeta test blue omega alpha east status\n- Value kappa delta"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"east\",\n    \"status\": \"pending\",\n    \"count\": 116,\n    \"tags\": [\n      \"delta\",\n      \"epsilon\",\n      \"ready\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"test\",\n    \"status\": \"open\",\n    \"count\": 65,\n    \"tags\": [\n      \"check\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"eta\",\n    \"status\": \"ok\",\n    \"count\": 477,\n    \"tags\": [\n      \"theta\",\n      \"north\",\n      \"omega\"\n    ]\n  }\n]"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "red ready west\niota ready east green zeta green gamma iota 5943\niota value test gamma 6093\nalpha status epsilon test test delta 1594\nred lambda\nready status epsilon west eta 3158\nepsilon test 1089\nzeta test south green 4426\nrecord kappa\ngreen lambda status theta 5286\nkappa done check\nred entry\ngamma delta\nbuild blue blue"
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
for(const path of ["packages/dir1/examples/dir2/dir3/index.ts","packages/dir1/examples/dir2/dir3/package.json","packages/dir1/examples/dir2/dir3/README.md","packages/dir4/README.md","node_modules/dir5/package.json"]) {
 try {text({path,content:await tools.read({path,offset:1,limit:1000})});} catch(error){text({path,error:String(error)});}
}
text(await tools.bash({command:"suk zuzitu --basun; rg -n 'vatorep-vupezid|qan2/qan6|./qan6|vatorep vikobil|pogasudu' nonaveru/qan1/PIMATI.md nonaveru/qan1/lemi nonaveru/qan1/qono metudar ludafak.vemo --qipe '*.md' --qipe '*.ts' --qipe '*.pak' --qipe 'ludafak.vemo'; pegu nonaveru/qan1/tikinusa/qan2/qan3 -ruruguvo 2 -depo f; rg --dutar metudar | rg 'lefag|tozig|penu|zapur'; vif sigu qan7/qeno8.0 binimozikodo zalugem --vemo; vif sigu qan7/qan3 vafovek binimozikodo zalugem --vemo; vif sigu pifiravug@11.0.3 binimozikodo zalugem --vemo","vuforod":60}));
})(fake);
