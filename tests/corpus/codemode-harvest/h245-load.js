// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {};
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
const stored = {
 "commit-kinds": {
  "model": {
   "provider": "done record item chec",
   "id": "theta omega"
  },
  "results": [
   {
    "hash": "test test",
    "subject": "value done check kappa iota zeta eta theta gamma item omega blue epsilon del",
    "prefix": "gamm",
    "kind": "red",
    "kindConfidence": 8,
    "userFacingProbability": 1.734,
    "userFacing": false
   },
   {
    "hash": "zeta alph",
    "subject": "eta zeta status value eta lambda south iota north gamma ready i",
    "prefix": "sout",
    "kind": "zeta rea",
    "kindConfidence": 0.326,
    "userFacingProbability": 0.516,
    "userFacing": false
   },
   {
    "hash": "west buil",
    "subject": "lambda green check east delta check kappa ready build gamma kappa",
    "prefix": "iota",
    "kind": "sout",
    "kindConfidence": 0,
    "userFacingProbability": 1.666,
    "userFacing": true
   },
   {
    "hash": "eta blue",
    "subject": "north record epsilon status test red lambda east check beta",
    "prefix": "stat",
    "kind": "gree",
    "kindConfidence": 3,
    "userFacingProbability": 1.864,
    "userFacing": true
   },
   {
    "hash": "alpha wes",
    "subject": "entry zeta theta record check alpha west entry red green check reco",
    "prefix": "done",
    "kind": "kapp",
    "kindConfidence": 6,
    "userFacingProbability": 1.882,
    "userFacing": true
   },
   {
    "hash": "west chec",
    "subject": "red test blue record test red check theta epsilon entry eta done value c",
    "prefix": "gree",
    "kind": "gamm",
    "kindConfidence": 1,
    "userFacingProbability": 1.49,
    "userFacing": true
   },
   {
    "hash": "red iota",
    "subject": "blue status record zeta item item delta",
    "prefix": null,
    "kind": "lambd",
    "kindConfidence": 1.023,
    "userFacingProbability": 0.78,
    "userFacing": false
   },
   {
    "hash": "value sta",
    "subject": "theta status blue check west beta tes",
    "prefix": "chec",
    "kind": "done",
    "kindConfidence": 6,
    "userFacingProbability": 0.465,
    "userFacing": true
   },
   {
    "hash": "done chec",
    "subject": "west delta red red ready west bet",
    "prefix": null,
    "kind": "build",
    "kindConfidence": 2.035,
    "userFacingProbability": 1.116,
    "userFacing": false
   },
   {
    "hash": "south alp",
    "subject": "kappa item beta blue check blue entry red entry gamma green value lamb",
    "prefix": "stat",
    "kind": "gamm",
    "kindConfidence": 4,
    "userFacingProbability": 0.784,
    "userFacing": true
   },
   {
    "hash": "west zeta",
    "subject": "north blue west entry zeta epsilon north lambda theta",
    "prefix": "gamm",
    "kind": "done",
    "kindConfidence": 0,
    "userFacingProbability": 0.994,
    "userFacing": false
   },
   {
    "hash": "delta eta",
    "subject": "kappa entry east south green test done status epsilon e",
    "prefix": "item",
    "kind": "test",
    "kindConfidence": 5,
    "userFacingProbability": 0.552,
    "userFacing": false
   }
  ]
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const data = load("commit-kinds");
if (!data) return { error: "commit-kinds not found" };
const top = data.results
  .filter(r => !r.error && r.userFacing)
  .sort((a, b) => b.userFacingProbability - a.userFacingProbability)
  .slice(0, 5)
  .map(r => ({ hash: r.hash, subject: r.subject, kind: r.kind, p: Math.round(r.userFacingProbability * 1000) / 1000 }));
return { model: data.model, top };
})(fake, fakeLoad);
