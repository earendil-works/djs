// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "check test\nzeta omega eta 7955\nred eta kappa item zeta 6411\neast south eta omega 9586\ndelta omega lambda alpha north\nred north east north blue red record\nblue omega beta iota green alpha\ngreen record\nwest lambda lambda value ready\neast status\ngamma item zeta 2091\ngamma record iota build alpha 4267\neast west beta value"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "const test done iota\n}lambda kappa\n  return south\n}theta blue\n}gamma check\nlet eta build\n  return gamma check beta theta"
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
text(await tools.bash({command:"fude -s sevin://qot5/qot6/qot7 /qame/qure/qot8/qot9/qot10/qot11/favitaf/nata12.js 'sevin://nata13.ses/qot14/qot15' --fak","mukutaz":60}));
text(await tools.read({path:'/qame/qure/qot8/qot9/qot10/qot11/favitaf/nata16.js'}));
})(fake);
