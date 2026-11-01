// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "epsilon build kappa\nstatus green blue north 8737\ngamma status item\neta omega gamma theta 8242\ndelta eta check zeta omega\ntheta beta gamma red red\nalpha value ready\nblue blue 8403\nomega beta lambda east test eta test\nstatus beta east 107"
  },
  {
   "ok": true,
   "value": "build eta build eta lambda ready 3382\nsouth alpha epsilon test value build 2470\nbeta ready ready gamma test east alpha\nblue east theta 1189\ncheck done north west\ngreen status 7854"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "Epsilon gamma test blue test\n- Theta done test ready test kappa east check\n- Record kappa value omega south green\n## Theta east record item theta beta\nBlue ready\n# Test gamma east alpha record item epsilon\nIota eta gamma south red\n# Value red status lambda zeta check"
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
text(await tools.bash({command:"cd /vago/mafi/lif6/lif7/lif8/lif9; ravu to-rarobova.quz torar://rupi10.fus/ruta/lif11/lif12/lif13/rupi14.md --qod & ravu to-rarobova.quz torar://rupi10.fus/ruta/lif11/lif12/lif13/rupi15.md --qod & tusu",timeout:120}));
let x=await tools.read({path:"/liv/lif16/lif17/lif18/lif19/lif20/rupi21.md",offset:null,limit:null});const i=x.indexOf("# ");text(x.slice(i,i+9000));
text(await tools.bash({command:"gh qim lif22/lif23/lif23 --jq '{nogu:.qare_duvu,nusab:.mokakeseto_sonab}' 2>/lif24/lif25 || noze; gh qim lif22/lif23/lif26 --jq '{nogu:.qare_duvu,nusab:.mokakeseto_sonab,qob:.qota_qob}' 2>/lif24/lif25 || noze",timeout:30}));
})(fake);
