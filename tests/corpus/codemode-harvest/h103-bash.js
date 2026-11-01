// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "iota record green\ntheta gamma record lambda check\ngreen build south test\nblue record omega red build red\nsouth test lambda\nkappa blue north lambda entry green 953\nzeta east lambda\nblue status build\ntheta omega check epsilon alpha south status gamma\ndone lambda ready lambda value\nbuild check item test south west green 2170\nwest theta blue west eta omega status omega\nready record test value beta\niota status east item kappa"
  },
  {
   "ok": true,
   "value": "alpha done theta eta\nbuild check blue\nitem gamma south green record east eta west 7673\ncheck lambda zeta south theta theta east 9928\ntest epsilon record red red kappa\ndelta value alpha lambda gamma 6691\ndone build done south beta kappa 4975\nwest epsilon epsilon green east epsilon entry eta 5575\ndone gamma check\ngreen test status omega iota record build omega"
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
  tools.bash({command:"qul nera",timeout:120}),
  tools.bash({command:"qul sus letam; qen rila --letam",timeout:120})
]); results.forEach((result,i)=>text({check:i,result}));
})(fake);
