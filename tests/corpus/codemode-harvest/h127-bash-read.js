// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "Build gamma epsilon\n## Beta alpha alpha green east record\n1. Theta build check lambda eta iota kappa\nCheck lambda"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "iota done south beta south value test\ngamma record east record iota\nvalue west theta beta omega entry done\nlambda blue east done east build check red 5872\ngamma build alpha entry blue\nzeta zeta done value entry 3546"
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
const [doc, global] = await Promise.all([
  tools.read({ path: "nifomida/qad1/pana/kege2.md", offset: null, limit: null }),
  tools.bash({ command: "qad ~/qad3/qad4/kege5.qasi", timeout: null }),
]);
text(doc);
text("=== global qob.json ===\n" + global.output);
})(fake);
