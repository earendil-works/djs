// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "beta gamma north\ngamma green done\nnorth alpha iota green omega status\ngreen delta check blue test\ngreen east epsilon entry kappa east alpha\ntest blue test iota eta\nnorth status item zeta item test lambda item\nentry kappa\nentry delta 2957\ntheta value\nkappa omega\neast blue check blue value item\nzeta green beta 9500"
  },
  {
   "ok": true,
   "value": "build west south\ngamma done\nomega delta north done delta check beta test 1842\nrecord south test west\nnorth test\ngamma green eta ready\nzeta build item east build red theta 9273\nready green epsilon build north\ngamma omega east build test"
  },
  {
   "ok": true,
   "value": "red zeta\ndone record red\nnorth iota lambda red epsilon delta\nzeta build entry epsilon\nbeta alpha beta west red omega omega zeta 635\niota done\ngreen south ready epsilon east beta eta gamma\nzeta test done test epsilon zeta record kappa\nsouth north\nbuild eta eta theta blue entry zeta kappa\nready red 4531\ngamma epsilon zeta red eta status alpha 9615\ntheta zeta red"
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
const r=await Promise.allSettled([
tools.bash({command:"qit fes bepov",timeout:120}),
tools.bash({command:"cd lifipida/vom1 && dute \"$(fak qas-zogas --savo-gabisefo)/dute_lakipid/vom2/susa/lopo3.js\" --fes keke/lopo4.ts keke/vom5/lopo6.ts keke/vom5/lopo7.ts",timeout:120}),
tools.bash({command:"fak barap feneko; fak qas-gori --labo-gufop --korel vom8/vom9; fak poko --zukeva --tozu-vipa",timeout:120})
]);r.forEach(text);
})(fake);
