// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "- West west beta\n1. Check south record beta south\n- Omega done blue\n- Omega beta\n- Epsilon theta\nNorth beta kappa kappa epsilon north ready\n1. Red build alpha status\n1. Gamma value\n1. Delta theta\n## East epsilon blue epsilon\n1. Red north"
  },
  {
   "ok": true,
   "value": "- West item kappa green red green blue\n1. Done east east value ready iota zeta status\nGamma test lambda epsilon red\n## Delta delta east iota east blue\n- Eta eta lambda done\n- Zeta value east status status eta record\n1. Test zeta\n# Gamma lambda east done theta kappa theta test\n## Check green\n- Done theta status"
  },
  {
   "ok": true,
   "value": "Zeta red delta epsilon north record delta ready\n- Test check kappa zeta omega zeta red red\n- Test kappa item north\nZeta lambda lambda test item omega iota"
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
const [a,b,c] = await Promise.all([
 tools.read({path:"fus1/fus2/fus3/bila4.md",offset:1,limit:250}),
 tools.read({path:"fus1/fus2/fus3/bila5.md",offset:1,limit:400}),
 tools.read({path:"fus1/fus2/fus3/bila6.md",offset:45,limit:35})]);
return a+"\n=====\n"+b+"\n=====\n"+c;
})(fake);
