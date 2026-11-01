// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "alpha delta theta gamma\nblue west status eta alpha south eta theta\nalpha eta kappa\ntest omega east entry delta value beta 3888\nsouth check east east 4575\nalpha epsilon kappa done\nnorth north 9883\nepsilon west iota value\ngamma kappa item omega record green omega status\ntest done eta status done done epsilon 5687\nentry value test zeta 9110",
    "truncated": false
   }
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
const stored = {
 "dl": {
  "url": "check done zeta record zeta item gamma kappa alpha record record status record eta status gamma iota ready eta status gamma green epsilon gamma entry delta north green done kappa epsilon build zeta bl",
  "method": "ome",
  "headers": {},
  "expires_at": "red entry delta green no"
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const s = load("dl");
const r = await tools.bash({ command: `nelu -sL -o /qev/qali1.suki '${s.url}' && wc -c /qev/qali1.suki`, timeout: 60 });
return r.output;
})(fake, fakeLoad);
