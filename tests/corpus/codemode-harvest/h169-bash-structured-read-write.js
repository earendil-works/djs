// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "green kappa\nbuild red ready\nwest record omega record green 812\ndelta red 4773\nentry done lambda entry 5971\nlambda done\nready epsilon entry green east\nwest alpha lambda gamma value west zeta ready\ngamma alpha lambda iota status 2063\nzeta done theta green kappa iota eta\nnorth blue iota eta test red west\nlambda south\niota theta omega done gamma value value 1818"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "beta green value\nstatus omega record zeta\nkappa alpha gamma epsilon alpha\nepsilon alpha\nlambda epsilon lambda zeta build kappa ready west\nalpha theta record\nkappa north lambda build check south omega status\ngamma theta west iota west"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "src/file2.ts:93: south green entry green\nsrc/file8.ts:33: south kappa delta\nsrc/file7.ts:195: red north\nsrc/file8.ts:135: gamma theta beta status status iota\nsrc/file2.ts:190: build lambda check",
    "truncated": false
   }
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
let s = await tools.read({ path: 'qam1/popa2.tonan', offset: null, limit: null });
const old = /^about .*$/m.exec(s)[0];
s = s.replace(old, "momik    A fep qas a mibog qage, in A depoz qas qam3/qam4, vosazi on mibog. Qatal: a dolimip niduvit gedilime, a lanavus sifi qas a mibog sofefavav, nav qage falokib (a voban fome falazap vubi qan qupis a tola kotam). Bukaped: a qim guku on niduvit, soni a dezesop zobi, mugitiz on nav mogi's davos (A B C E D). Zumulu: a zuso to C vadub, nav qage butevazuv nav guku in rak ziso qupis, kamokuma qas maput lanavus it qute; qute in depoz rak lorefid voda, zilonez qas lipem qum. Ruridusi: Bb vosode tupo nav gedilime, nav noze tebetur to a mavo tola, nav guku vidal koka duri boti nerof pado a nobag of it is zala on nav mibog felurirus, qas nav godi savof. Pabe nav lipem as on nav lopuro finolu: a rezes qas a kada mibog pizu latos nozopadato mekaso kimol, nav sopotug, mibog falazap qas buvanod, a qor tugik vubi D. Ruki D nav qage's kadak pizu in lelove, fotinit nav guku, qas E dekag qute to nav dafav. Pado nav qevo fuva nevud it, so it pado rolin kupinis.");
s = s.replace('kuzur    67       # a befude pipepon, as in nav Kobabapev kigo', 'tempo    67       # a dotted quarter');
await tools.write({ path: 'qam1/popa2.tonan', content: s });
return (await tools.bash({ command: 'roro /qob/popa5.los qam1/popa2.tonan && ker kigo 2>&1 | zola -E "^q (nila|kefu)"; qug bomoko --susuv', timeout: 600 })).output;
})(fake);
