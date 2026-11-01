// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "eta kappa ready kappa gamma west status delta\neast iota red gamma done theta omega blue 1625\nred omega east 6308\nentry build check record omega\ndone lambda\nomega zeta north red alpha done north alpha\nkappa kappa value omega theta\niota check build zeta omega eta zeta alpha 5481\nbuild beta blue iota entry delta value"
  },
  {
   "ok": true,
   "value": "red build omega beta gamma theta blue 8463\ntheta value 8166\nnorth item record\nepsilon item build east 1217\ndone status east iota item omega value lambda\nomega east done ready gamma 5556\ndelta west eta iota\nsouth alpha item\nepsilon red ready eta 1045\nwest epsilon east delta omega lambda delta"
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
tools.bash({command:"biv raf qola:lobaluture > /zes/loti1.vil 2>&1; sumami=$?; qome -55 /zes/loti1.vil; fado $sumami",timeout:120}),
tools.bash({command:"biv gino -- redoba --soba-finubune lotuseze/tul2/qut/timab/loti3.ts lotuseze/tul2/qut/loti4.ts lotuseze/tul2/qut/tul5/tul6/loti7.ts lotuseze/tul2/qola/loti8.ts && biv gino -- qural --pavel lotuseze/tul2/qut/timab/loti3.ts lotuseze/tul2/qut/loti4.ts lotuseze/tul2/qut/tul5/tul6/loti7.ts lotuseze/tul2/qola/loti8.ts && qib finu --pavel; qib sotume --varod; qib finu -- lotuseze/tul2/qut",timeout:30})
]); for (let i=0;i<results.length;i++) text({check:i,...results[i]});
})(fake);
