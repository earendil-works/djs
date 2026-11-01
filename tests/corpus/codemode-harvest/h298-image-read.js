// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "record check done iota epsilon delta item beta 8195\nnorth epsilon epsilon omega\ntest iota value eta east zeta\ntest value\ntest red delta\nzeta alpha red green\nitem check\nzeta beta ready value gamma 8005\nbuild entry gamma epsilon west kappa lambda\ntheta epsilon check test theta\nnorth delta"
  },
  {
   "ok": true,
   "value": "build zeta item value delta record\nalpha north iota gamma delta\nitem delta north item 836\nsouth ready kappa epsilon gamma\nnorth beta test blue"
  },
  {
   "ok": true,
   "value": "north lambda epsilon\nlambda kappa entry\nrecord build build eta red\nred ready epsilon test lambda epsilon gamma\nomega omega theta check theta 714\nzeta item item lambda west 7909\nrecord north 5838"
  },
  {
   "ok": true,
   "value": "ready iota red check item gamma\nvalue lambda kappa build\nalpha delta omega item 129\nblue entry zeta blue\niota test 3597\nnorth check delta record item kappa\niota check green\nepsilon kappa 9213\nbeta zeta value south iota beta record"
  },
  {
   "ok": true,
   "value": "record south 3824\nkappa gamma item lambda blue 7220\nentry epsilon\nwest gamma build kappa\nepsilon lambda omega\ndone kappa check blue beta item north 7739\nready record alpha ready status gamma beta record 7082\nomega iota\nzeta item test\nzeta blue ready blue blue ready theta\nbuild zeta eta zeta epsilon zeta omega\nready beta east done build iota build epsilon 5036"
  },
  {
   "ok": true,
   "value": "status beta value check item ready green red 3721\ndelta alpha entry item theta west west alpha 4885\nepsilon build done entry epsilon entry gamma test 2697\nready item\nalpha west done 8053\nready omega ready epsilon 3474\nomega beta delta zeta"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
for (const i of [1,2,3,4,5,6]) image(await tools.read({ path: `/qeb/vak1/vak2${i}.dut`, offset: null, limit: null }));
})(fake, fakeImage);
