// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "west build done item kappa check test value\nkappa alpha red\neast gamma south done alpha\nepsilon kappa build zeta record build\ntest ready blue zeta gamma gamma\ntheta theta eta epsilon south zeta 1355\ndelta blue delta south value test\ngamma blue\nalpha zeta\nbeta north lambda\ndone south green item north check theta\nentry record alpha 8212"
  },
  {
   "ok": true,
   "value": "green value kappa\nzeta kappa test south epsilon\nred eta build green beta eta\ncheck status south build\neta green delta kappa\nlambda record"
  },
  {
   "ok": true,
   "value": "iota done\nzeta east omega delta\nblue lambda entry test item omega 3652\nrecord ready red build item value omega blue 3684\ncheck lambda\nzeta eta theta kappa\nalpha done zeta 7536\nready test done east value north\nblue delta beta check build blue 6145\nentry eta done 9580\nkappa north status\nready epsilon\nentry omega delta done north status iota 753\nitem item east item"
  },
  {
   "ok": true,
   "value": "kappa check delta entry ready red\nbuild eta check delta red\ngamma theta west omega delta eta\nred south green lambda test\nnorth green theta ready ready build status 5024\nalpha build done kappa value eta ready"
  },
  {
   "ok": true,
   "value": "check build record zeta kappa\nblue west epsilon entry south green omega gamma\ncheck done green red eta\nepsilon gamma delta lambda 2528"
  },
  {
   "ok": true,
   "value": "red beta east lambda 3674\nalpha epsilon status delta\nwest item 6673\neta beta theta west lambda north ready 6929\ndelta lambda iota alpha\nkappa alpha epsilon\nepsilon gamma west beta theta iota delta delta\nvalue east kappa south\neta alpha green item north theta\nnorth check lambda west west iota done ready\nomega test value"
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
const ids=[38380805422,38380805456,38380805440,38380805264,38380805713,38380805742];
const results=await Promise.allSettled(ids.map(id=>tools.bash({command:`gh qam --mebuva DEGE qeg1/qeg2/qeg3/qeg4/qeg5/${id}/qeg6`,timeout:20})));
results.forEach((r,i)=>text({id:ids[i],result:r}));
})(fake);
