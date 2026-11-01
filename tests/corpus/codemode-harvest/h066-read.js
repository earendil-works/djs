// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const theta green done\n  return gamma value\n  if (eta zeta\nlet zeta alpha item\n  if (kappa red"
  },
  {
   "ok": true,
   "value": "}delta east lambda\nlet red test\nfunction build test item gamma\n  if (kappa\nexport build status\nconst green alpha iota west\n}zeta lambda east\nfunction entry north\n}delta item theta\nconst south red\n  return red test test\nexport test red\nconst delta record alpha green\nfunction north record record"
  },
  {
   "ok": true,
   "value": "export red omega theta\n  if (south green entry status\n}eta west epsilon gamma\n}zeta lambda blue red\n  if (alpha\nlet lambda gamma\n  if (omega east check\n  if (west done\nlet gamma status status test"
  },
  {
   "ok": true,
   "value": "}north green east done\nexport north\n  if (item blue check record\n  if (green\n  return test blue delta\n  if (ready entry beta status"
  },
  {
   "ok": true,
   "value": "function epsilon blue record gamma\nconst item lambda record ready\nlet alpha check east\n  if (omega record\nfunction epsilon zeta zeta green\nfunction green green blue\n  if (item done beta ready\n}value entry\nexport lambda\nlet green\n  if (build status alpha blue\nlet kappa"
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
tools.read({path:"boseteso/qig1/zin/qig2/fepe3.ts",offset:1,limit:600}),
tools.read({path:"boseteso/qig1/zin/qig2/fepe3.ts",offset:601,limit:600}),
tools.read({path:"boseteso/qig1/zin/qig2/fepe3.ts",offset:1201,limit:600}),
tools.read({path:"boseteso/qig1/sato/fepe4.ts",offset:null,limit:null}),
tools.read({path:"boseteso/qig1/zin/monum/fepe5.ts",offset:null,limit:null})
]); for (let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);
