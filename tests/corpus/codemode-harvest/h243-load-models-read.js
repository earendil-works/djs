// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "omega red south 8804\neast green beta value item blue theta check\nitem build zeta 7179"
  },
  {
   "ok": true,
   "value": "record south blue south omega\nrecord epsilon kappa entry value record 4583\neta green status alpha 1578\nalpha check south alpha south\ndelta delta\nomega gamma build record delta\nready beta delta south done green 3845\nblue done record iota\ngamma zeta north delta\nbeta entry iota\neast eta gamma build\nomega delta check value record eta blue south"
  },
  {
   "ok": true,
   "value": "item check alpha iota build beta\nnorth test\nitem done\ngamma build epsilon\nomega zeta record alpha test green gamma iota\nzeta check check build kappa build south 3532\nvalue east eta done gamma beta 3758\nbuild entry lambda alpha iota lambda"
  },
  {
   "ok": true,
   "value": "alpha east record alpha entry south epsilon south 903\nlambda iota north item eta red check\neta eta gamma status iota status iota\nvalue status record east eta ready north\ntheta epsilon omega status blue green omega lambda\nepsilon value\ntheta record entry red west west north done\nready omega\nentry north north ready done north\nwest done 4740\nsouth theta north south item 2145\nlambda eta north blue green record blue\nomega north east lambda epsilon done alpha record 9603\nentry status item east west alpha build"
  },
  {
   "ok": true,
   "value": "green epsilon delta ready theta 1798\nlambda iota omega\nnorth alpha eta value check entry\nentry iota entry\nlambda done entry record epsilon blue\ndone entry lambda iota"
  },
  {
   "ok": true,
   "value": "epsilon alpha item east eta\nlambda ready south west done test delta\nready alpha north gamma check green record ready\nstatus lambda lambda\nitem alpha item green\niota value\ngreen build eta north entry beta entry\ntheta blue status entry 6790\nvalue value\nlambda west south 1397\nvalue blue eta\nvalue north zeta build done build blue"
  },
  {
   "ok": true,
   "value": "omega item red item alpha epsilon lambda\nwest beta status done done check check green 9012\nomega blue alpha delta ready\ndelta eta zeta east omega 7367\nitem status record epsilon epsilon east green entry\nentry north south north record"
  },
  {
   "ok": true,
   "value": "record status iota lambda delta\ngreen zeta 8460\ngamma zeta status eta\nbeta red delta\nstatus alpha"
  },
  {
   "ok": true,
   "value": "omega gamma omega done alpha 5866\nalpha east blue gamma\ntest build red status\nitem iota east check east epsilon 4861\nalpha lambda east delta\nalpha record\nstatus done epsilon record west 766"
  },
  {
   "ok": true,
   "value": "red item test status green lambda value 7576\ntheta zeta 9578\ngreen test east alpha\nrecord south south delta red iota iota lambda\nwest iota west\nlambda eta zeta omega"
  },
  {
   "ok": true,
   "value": "north lambda done eta done kappa gamma gamma\neta entry zeta omega build done ready blue\nred done eta red red\nkappa status check record alpha eta record 8\neast zeta value item build\nepsilon item build eta item kappa south"
  },
  {
   "ok": true,
   "value": "red eta iota record west 563\nzeta east iota theta item entry\nomega south theta\nbuild build\nred east red\nstatus record iota test zeta zeta lambda ready\niota build omega omega delta alpha 7800\ncheck epsilon zeta red eta green 9539\nepsilon check alpha eta\nsouth kappa check north south kappa alpha"
  },
  {
   "ok": true,
   "value": "west record\ndone build check eta kappa beta\nsouth epsilon omega red beta theta\niota east east build item record"
  },
  {
   "ok": true,
   "value": "status west\nbeta north\ngamma alpha delta omega green value green\ndelta kappa delta eta value\nrecord gamma west 6403\nbeta status 4644\ndelta eta eta zeta beta theta 977\neta omega check status omega delta west\nbeta south build eta red lambda blue status\ngreen value check check 365\nbuild value build 5909\nbuild omega zeta\neast build east south zeta kappa value value\nbuild kappa delta done delta"
  },
  {
   "ok": true,
   "value": "delta build north record test ready\nred red west\nitem gamma north south blue north\nblue entry check record value 7830\nomega status blue ready 665\nred item entry theta\nsouth lambda lambda south blue done theta green\neast value build done omega\neta check green iota item zeta delta status"
  },
  {
   "ok": true,
   "value": "blue beta west alpha delta\nred green delta east beta\nbeta west gamma green\nbeta green kappa kappa south north\neta alpha\nzeta east item beta lambda east iota zeta 5890\nvalue blue status done build west north\nstatus green delta theta theta zeta lambda check\nwest blue"
  },
  {
   "ok": true,
   "value": "check omega beta status test\nbeta ready north gamma iota check test omega\nbeta east"
  },
  {
   "ok": true,
   "value": "check test zeta check eta value epsilon test\nvalue north\nitem test done red blue\nred north iota green\nred ready omega beta value ready entry\nbuild eta theta build\nepsilon north entry test lambda build check 4312\nnorth theta status red done build west\nvalue gamma theta delta epsilon gamma"
  },
  {
   "ok": true,
   "value": "theta beta iota east eta lambda\nnorth south omega theta green 2419\nblue delta beta check test blue\nstatus done theta omega kappa done\nvalue east green test green green\nsouth gamma ready kappa build zeta item east 7999\ntheta status blue lambda kappa\nstatus epsilon status lambda item red\neast kappa lambda\nzeta status alpha green build north 8340\niota kappa green done beta beta iota\nepsilon iota epsilon green\ndelta north"
  },
  {
   "ok": true,
   "value": "value theta kappa red north\nrecord iota omega\nbeta status north south zeta test done\nstatus north epsilon\nsouth red ready test entry ready iota\ncheck theta 1587\nnorth entry blue omega test ready omega\nred beta entry theta beta item\ngamma blue item epsilon 611\nvalue gamma\nbuild lambda east north west eta south\nlambda iota south epsilon west status 1286"
  },
  {
   "ok": true,
   "value": "delta omega delta done check delta north 1193\ndelta record\nkappa check blue gamma green status\nitem lambda record\ntest value done delta beta"
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
const answer = (f) => {
  try {
    return later(f());
  } catch (error) {
    return later(error, true);
  }
};
const modelsOfType = (type, provider) =>
  [1, 2].map((i) => ({ type, provider: provider ?? "synthetic", id: "model-" + i, name: "Model " + i, api: "synthetic", input: ["text"] }));
const fakeModels = {
  getModelsOfType(type, provider) {
    return answer(() => modelsOfType(type, provider));
  },
  getAvailableOfType(type, provider) {
    return answer(() => modelsOfType(type, provider));
  },
  getModelOfType(type, provider, id) {
    return answer(() => ({ type, provider, id, name: id, api: "synthetic", input: ["text", "image"] }));
  },
  classify(model, context) {
    return answer(() => {
      JSON.stringify(context);
      const answers = {};
      for (const [id, q] of Object.entries(context.questions ?? {})) {
        if (q.type === "choice") {
          const labels = Object.keys(q.criteria ?? {});
          const probabilities = Object.fromEntries(labels.map((l, i) => [l, i === 0 ? 0.7 : 0.3 / Math.max(1, labels.length - 1)]));
          answers[id] = { type: "choice", choice: labels[0], probabilities, confidence: 0.7 };
        } else if (q.type === "score") {
          answers[id] = { type: "score", score: Math.min(1, (q.criteria ?? []).length - 1), confidence: 0.6 };
        } else {
          answers[id] = { type: "bool", probability: 0.65 };
        }
      }
      return { provider: model?.provider, model: model?.id, answers, stopReason: "stop" };
    });
  },
  generateImages(model, context) {
    return answer(() => {
      JSON.stringify(context);
      return { provider: model?.provider, model: model?.id, output: [{ type: "image", data: "iVBORw0KGgo=", mimeType: "image/png" }], stopReason: "stop" };
    });
  },
};
const stored = {
 "questions": {
  "sentiment": {
   "type": "green",
   "instructions": "test beta green blue zeta item test done green blue record south zeta theta gr",
   "criteria": {
    "positive": "green west west gamma north lambda t",
    "neutral": "entry gamma lambda check test re",
    "frustrated": "red lambda beta check blue zet",
    "angry": "ready lambda blue entry theta item entry"
   }
  },
  "urgency": {
   "type": "epsil",
   "instructions": "epsilon east item kappa delta delta bl",
   "criteria": [
    "eta east theta gamma zeta test red build west beta delta eta eps",
    "epsilon status build zeta east value check",
    "gamma north build iota done done theta zeta done"
   ]
  },
  "impact": {
   "type": "test",
   "instructions": "east ready north status kappa zeta alpha zeta iota item record south del",
   "criteria": [
    "epsilon item lambda omega epsilon item",
    "alpha kappa green done beta delta check omega south blue",
    "zeta status test iota south theta epsilon"
   ]
  },
  "actionable": {
   "type": "red",
   "instructions": "south red value theta epsilon iota kappa done epsilon done west rec",
   "criteria": {
    "true": "west west ready theta iota west beta test value",
    "false": "red ready green theta gamma east gamma east south alpha ep"
   }
  }
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, models, load) => {
const q = load("questions");
q.urgency = {type: "score", instructions: "Qob bebevuze biku daro tosuv kude vazebopus voli mof tufovusufof?", criteria: [
  "Qap nolape: mumesuge, tezo, gokotekekup rena or dafo-to-loko",
  "Pif: ronol sarusebit qake datu munaselefu",
  "Lulipe: poki qop or dunofen toruzitida lilovoraf roma borik, munaselefu zeguze",
  "Qise: pevigu qote gukusifo, tevuv, zuner tomupur, qate borik vemilod",
  "Pipugodi: vizanif, suzi qape, QID, pulolovu, or tisogubano vekamude"]};
q.impact = {type: "score", instructions: "Qob qike vataz vepot derolarak daro ragip to pi borik kutulom? Fonokise totebir of luvatogi borik soz nutedinin megimukabu (virubafer, degarupa).", criteria: [
  "Dudivugolo: tufile kaba, tevos sisu kifa",
  "Mufum: tevos qat1/qat2 or qete sadazoduzupik",
  "Letolado: a rozotunuzo lelubo of borik or a sefadem dedinome",
  "Keseb: desu borik of a zesula gukusifo or gesemo nutedinin parelo",
  "Fabi keseb: pobulu qar borik or fabi qise megimukabu"]};
store("questions2", q);
const model = {provider: "pirakabu", id: "vav-latest"};
const issues = [];
for (let off = 1; off <= 161; off += 8) {
  const qap = await tools.read({path: "/qip/limi3.vimol", offset: off, limit: 8});
  for (const l of qap.split("\n")) if (l.startsWith("{")) issues.push(JSON.parse(l));
}

const probe = issues.filter(i => [9276, 8331, 6930, 4945].includes(i.number));
const pr = await Promise.all(probe.map(i => models.classify(model, {state: i, questions: {urgency: q.urgency, impact: q.impact}})));
return probe.map((i, k) => ({n: i.number, t: i.title.slice(0, 60), u: pr[k].answers.urgency?.score, im: pr[k].answers.impact?.score}));
})(fake, fakeModels, fakeLoad);
