// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "lib/dir1/file1.ts\nsrc/file2.rs\nlib/dir9/file3.rs\nlib/dir1/file4.json\ndocs/file5.js\ntest/dir9/dir6/file6.js\ntest/file7.json",
    "truncated": false
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "eta omega value blue entry north entry iota 4401\nblue done delta status\ngreen eta record lambda green west kappa blue\nsouth check beta east\nomega red 1963\nbuild test record item delta gamma entry south\nitem south entry west green status kappa 8187\ntheta south\ngamma theta build east ready\neta lambda theta\nepsilon north green north"
  },
  {
   "ok": true,
   "value": "lambda check delta kappa item red kappa\nstatus theta theta build value eta check\ntest blue theta delta south eta beta south\nblue west green south status\nwest red build\ndelta red value lambda beta eta theta 8400\nkappa iota done check\nnorth blue beta south omega north check 8154\ncheck north blue gamma alpha ready check alpha 2779\nomega epsilon item kappa build theta blue alpha\nsouth value theta south\nbeta ready green eta delta lambda beta west 4632\nentry theta\nred iota lambda zeta green west eta ready 259"
  },
  {
   "ok": true,
   "value": "eta beta north green check test west kappa 3389\nlambda west eta\nomega ready check done check iota\nsouth build test south ready beta\nready item green ready gamma west item\ntheta status gamma value ready alpha build\ndelta blue test red north north 2202\niota record lambda test 2070\nomega red beta"
  },
  {
   "ok": true,
   "value": "alpha east lambda red item ready value build 9551\ndelta gamma lambda build zeta status item zeta 7426\nred west ready\nentry item done alpha red\nready gamma\nlambda ready east iota entry south\nlambda status kappa blue alpha epsilon ready status"
  },
  {
   "ok": true,
   "value": "build value test zeta zeta beta\nitem north build theta south 9705\nready lambda red lambda gamma blue alpha\nepsilon kappa green entry\ntest value north east\nnorth kappa 988\ngreen theta epsilon 7905\nvalue blue epsilon 1476\nvalue east"
  },
  {
   "ok": true,
   "value": "ready east eta blue epsilon south epsilon\nnorth west\neast iota lambda\nentry eta beta north zeta ready north 4663\nred entry beta alpha lambda eta\niota theta theta gamma ready build green test 2691\ntest status delta theta eta"
  },
  {
   "ok": true,
   "value": "zeta ready check red iota ready\niota blue north red theta blue\nblue zeta zeta lambda lambda west blue gamma 3927\ntheta build eta\nalpha gamma alpha delta\nwest iota beta status beta\ntheta beta west\neta blue status red zeta\nalpha item done kappa ready value\ngamma check check eta green green status\ndone theta kappa 3027"
  },
  {
   "ok": true,
   "value": "entry entry red eta eta item north green\nomega build green theta beta iota\nepsilon delta entry green test item eta\nkappa build done green alpha gamma omega item\nrecord iota done test kappa omega 6093\nrecord value green west green delta omega entry\niota west omega zeta epsilon iota record item"
  },
  {
   "ok": true,
   "value": "green ready omega\ndelta north zeta west\nlambda delta green blue zeta\nstatus status entry item epsilon lambda alpha epsilon\nitem omega south epsilon\nsouth entry east beta green test status lambda\nstatus value 4582"
  },
  {
   "ok": true,
   "value": "alpha north 7127\ntheta check green\nvalue green status gamma 2385"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
const dir = "/rufi/fave/qeb1/qeb2/qeb3/qeb4/zapel/qeb5/";
const files = (await tools.bash({command: `ls ${dir}*.dil`, timeout: null})).output.trim().split("\n");
for (const f of files) {
  const r = await tools.read({ path: f, offset: null, limit: null });
  if (typeof r !== "string") image(r);
}
})(fake, fakeImage);
