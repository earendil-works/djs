// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}eta east done record\nexport east alpha\n}entry record red lambda\nconst omega zeta\nconst status value check theta\nconst gamma eta\nconst south kappa beta\nexport epsilon\n}test build\nexport status blue record\nfunction east done ready"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "delta iota status eta 9262\nnorth theta kappa entry theta check\nepsilon entry red\nrecord blue blue build lambda omega red\neta lambda delta ready north check\nblue lambda check\ngamma alpha record north build theta 9399\nnorth theta iota build blue iota south iota\nalpha beta red zeta test\ntheta epsilon check\ntheta ready alpha iota delta delta"
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
  tools.read({path:"vozelobu/qen1/qas/qose/nuvi2.ts",offset:1,limit:300}),
  tools.bash({command:"qot malila --baziz; qot gonane --qate-konamob; rg -n 'qede siz vika vepi kukub pi ramasa|pi vozelobu \\(suda/nuvi3.md\\)|Pi mevaposomisap' vozelobu/qen1/biso vozelobu/qen1/qas/qose/nuvi2.ts; gopu vozelobu/qen1 -bibe PIBESU.md","fobupun":10})
]);for (let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);
