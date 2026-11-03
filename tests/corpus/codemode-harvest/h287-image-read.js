// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "south done blue\nrecord south entry test\nomega entry\nnorth north zeta eta build ready south\ntest build test blue\nlambda south 5225\nred test\nrecord east beta check status value eta\nitem test epsilon delta\nnorth ready south west east done blue epsilon\nstatus status epsilon epsilon theta status 4703\nvalue check south item done south status\ncheck test west"
  },
  {
   "ok": true,
   "value": "blue check eta record\neta test test beta omega eta value\ncheck gamma omega check status alpha theta\neast east\nblue omega beta gamma north delta iota 3180\ntest east beta east green beta epsilon alpha 9835\niota west\nalpha item omega red eta beta north 7349\nred beta east red entry item value omega 9813\nalpha alpha\ngreen eta delta south blue beta iota zeta"
  },
  {
   "ok": true,
   "value": "test south eta red eta 9173\nzeta theta\ngamma iota north kappa kappa eta zeta 6875\nstatus check west blue 6669\niota test omega kappa build theta status theta 1312\nred value entry eta green green\neta entry ready green blue\ntest green east west\ntest eta done west blue north green\neast iota eta west eta green south test 3745\nbuild check build\ndelta red alpha green red item theta\nnorth epsilon green entry blue beta 6470\nkappa eta check north ready test check record"
  },
  {
   "ok": true,
   "value": "red alpha kappa beta item epsilon test done 9897\nbeta epsilon gamma blue test green\nblue iota\nred gamma test item gamma done\ntest south theta delta build blue\nitem west green\ncheck omega beta done theta alpha"
  },
  {
   "ok": true,
   "value": "ready green status north iota check south\nred status iota test build zeta\nsouth south epsilon green kappa\nstatus alpha red test kappa theta\nrecord done record beta\nitem ready status zeta\ngreen value red 3545\nepsilon item epsilon entry item check 3518"
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
for (const i of [2,3,4,5,6]) image(await tools.read({ path: `/tev/gef1/gef2${i}.qor`, offset: null, limit: null }));
})(fake, fakeImage);
