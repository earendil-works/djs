// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "item west red theta eta iota\ndone alpha 6932\nrecord value eta record green alpha\nepsilon gamma alpha delta zeta entry iota green\nomega omega\nsouth east gamma alpha green item ready omega 1104\nbeta eta record\ngreen red\nrecord red\nblue value check east 7935\nred lambda\nitem blue west value red kappa 898\neta test iota 4816"
  },
  {
   "ok": true,
   "value": "delta item item eta\nkappa west west record kappa value 5289\nstatus iota theta blue iota\nnorth green zeta epsilon zeta epsilon eta\nwest epsilon red\nbeta iota south 1590\nomega item iota zeta omega build kappa iota\nomega item\ndelta beta west done status test\nlambda kappa epsilon south red value lambda 7297\ndelta alpha zeta south north build entry"
  },
  {
   "ok": true,
   "value": "status done zeta item 9321\nomega lambda eta test status south 5606\neast status iota omega theta west status\nnorth eta\nstatus north theta ready\niota epsilon theta"
  },
  {
   "ok": true,
   "value": "entry value epsilon east check 2581\nzeta theta\nstatus status ready status delta status red value 9384\nzeta iota\niota omega\nepsilon ready status blue done eta\nkappa check gamma south south red south\nwest gamma delta\nred gamma north green\nzeta omega eta alpha test check delta\nentry build omega west\ndelta south 1222"
  },
  {
   "ok": true,
   "value": "status done red theta\nomega iota 235\nready status test iota 9244\nvalue test west\ngreen status beta red item green value 1230\nready ready status north 2767\ngamma build delta beta build\nred omega south record 825\nepsilon green zeta build delta item blue\nwest west delta delta build theta red check 3707\nentry ready zeta zeta ready green item\ndelta entry west green south build east"
  },
  {
   "ok": true,
   "value": "red theta lambda south west test build green 6443\niota south blue lambda entry kappa 4745\nready ready blue blue item 9791\nlambda value east zeta test eta south\nkappa epsilon eta north\ntheta record blue lambda north 4901"
  },
  {
   "ok": true,
   "value": "red theta\nentry kappa red build south 6241\nkappa item value kappa blue iota\ntest west zeta alpha kappa iota red lambda\nlambda beta south east ready entry\nrecord build beta test west build status south 9872\nwest red kappa done east delta red\neast check status south epsilon\neta kappa build record beta epsilon ready\nlambda entry ready build value eta\nnorth red iota red eta eta south delta"
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
const ns=[15,19,39,54,94,79,87];
const r=await Promise.allSettled(ns.map(n=>tools.bash({command:`gh nukub fufo ${n} -R nel1/nel2 --papu lokag,qone,fuzenilo --jq '"# \\(.lokag)\\n" + (.qone[:700]) + "\\n" + ([.fuzenilo[] | "-- \\(.fodabi.loguv): " + (.qone[:600])] | .[0:4] | budo("\\n"))'`,timeout:30})));
r.forEach((x,i)=>console.log(`\n===== #${ns[i]}\n`+(x.status==='fulfilled'?x.value.output:x.reason)));
})(fake);
