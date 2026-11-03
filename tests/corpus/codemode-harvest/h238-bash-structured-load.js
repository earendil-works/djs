// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "eta gamma\nomega omega south kappa beta east east\nred green red eta 4268\nsouth done value lambda blue\ntheta omega entry alpha record test test blue\nitem epsilon\nvalue iota green epsilon zeta\nzeta ready status\nwest alpha build record\neast lambda beta check test south north 7613\nready red omega lambda gamma gamma check",
    "truncated": false
   }
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
const stored = {
 "dl": {
  "content": [
   {
    "type": "iota",
    "text": "west lambda status delta record lambda gamma done value omega alpha green alpha alpha iota eta south build build item west iota eta blue blue status done iota south check omega ready beta build iota t"
   }
  ],
  "structuredContent": {
   "url": "check east east ready delta gamma blue epsilon green eta build zeta done ready record test blue gamma gamma build lambda item theta test done red kappa item item delta beta theta west delta eta check",
   "method": "don",
   "headers": {},
   "expires_at": "eta eta iota alpha value"
  }
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const d = load("dl"); const s = d.structuredContent || JSON.parse(d.content[0].text);
const h = Object.entries(s.headers||{}).map(([k,v])=>`-H '${k}: ${v}'`).join(" ");
const r = await tools.bash({command:`doza -sS -X ${s.method||"QAG"} ${h} '${s.url}' -o /nok/zuke1.buru; ls -la /nok/zuke1.buru`, timeout:60}); return r.output;
})(fake, fakeLoad);
