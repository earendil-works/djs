// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "ready west ready theta north red gamma\ngreen build omega green alpha alpha epsilon red\ngamma zeta lambda kappa eta north ready iota 1347\nstatus zeta alpha\nstatus ready west theta lambda epsilon zeta west 1864\nnorth build north red"
  },
  {
   "ok": true,
   "value": "ready gamma gamma check north east 8337\nlambda entry status test kappa green\ncheck done"
  },
  {
   "ok": true,
   "value": "blue ready done record south epsilon\niota epsilon epsilon alpha check kappa 5384\nalpha build\nomega kappa omega\nkappa done south\nlambda south epsilon alpha\nentry gamma check done gamma kappa 7975\nblue lambda check\ngamma check alpha kappa\neast item west west entry north 3225\nbuild zeta theta north south red\nbeta test\nsouth gamma north item record west beta\nvalue record entry build status alpha build"
  },
  {
   "ok": true,
   "value": "check build value 3395\nred check status test west done\nstatus epsilon omega value record check done\nbuild kappa build record entry alpha done\nnorth check lambda build\nbuild blue\nalpha blue\nsouth north record east build 2451\ndelta west gamma\nstatus east east record iota status\nvalue status iota delta iota gamma\ndone epsilon red check item status\nready iota\nbeta theta test gamma epsilon green"
  },
  {
   "ok": true,
   "value": "blue green record iota green green value\nomega blue iota item theta north iota 4550\ntheta done south lambda\nzeta north east epsilon item north red 9847\nkappa iota 8847\nred item omega eta gamma check\nready south alpha test west entry\nsouth delta theta beta record build item ready\nentry lambda 7366\nentry status alpha eta delta beta 1649\neta entry record lambda blue beta delta lambda 6068\ntest status check"
  },
  {
   "ok": true,
   "value": "iota south ready\ntheta lambda record zeta epsilon\ndone record epsilon east east done\nsouth beta red east epsilon 5436\nkappa beta ready alpha 6165\ntest north east\nepsilon zeta status west check"
  },
  {
   "ok": true,
   "value": "done alpha value south 765\ntest lambda done omega\nentry delta beta blue epsilon entry record green\nbeta test entry south eta\nlambda value theta lambda south value gamma iota\nred test delta\nbuild omega build blue green ready ready 4088"
  },
  {
   "ok": true,
   "value": "record delta lambda gamma omega lambda red entry 5964\nred beta delta entry\neast test 8728\nkappa east done 2853\nbuild item beta check kappa gamma zeta 2789\niota status kappa zeta test delta zeta gamma\nzeta build check lambda"
  },
  {
   "ok": true,
   "value": "done epsilon zeta test test gamma\ntest check status alpha test\nready record alpha west\ncheck status record delta\ndone build epsilon test theta build record 9511"
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
const repos = {"dir1/dir2":["v5","v6","v7"],"dir1/dir3":["v6","v7"],"dir1/dir4":["v5","v6","v7"],"dir1/dir5":["v5","v6","v7","v8"],"dir1/dir6":["v5","v6","v7"],"dir1/dir7":["v6","v7"],"dir1/dir8":["v4","v5"],"dir1/dir9":["v5"],"dir10/dir11":["v4","v5","v6","v7","v8","v9","v10"]};
const jobs = [];
for (const [repo, majors] of Object.entries(repos)) {
  jobs.push(tools.bash({command:`gh qog 'suluk/${repo}/qop12=100' --jq '.[] | musesa(.nem_zamo|fela("^(${majors.join("|")})\\\\.0\\\\.0$")) | "### ${repo} \\(.nem_zamo)\\n\\(.fuva)"' | qali -iE '^###|masokiko|gatumip|vezotob|fizarin|vutugile|zuro|derevi|gokomo|v[0-9]+\\.0\\.0' | vepa -40`, timeout:60}));
}
const r = await Promise.all(jobs);
return r.map(x=>x.output).join("\n");
})(fake);
