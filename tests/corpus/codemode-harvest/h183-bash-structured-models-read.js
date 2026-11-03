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
    "output": "src/file5.ts:105: lambda done entry omega zeta\nsrc/file3.ts:277: test south\nsrc/file1.ts:378: kappa red check theta south theta",
    "truncated": false
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "# Item south eta west test blue blue\n- Alpha test\nStatus west east\nStatus east"
  },
  {
   "ok": true,
   "value": "1. Eta east delta status value\n1. Item delta zeta build record entry blue status\nTest zeta omega zeta record check alpha\nValue check item south epsilon\n1. Red red build\nEast red value\n- South zeta epsilon\n# Status status east iota delta gamma west item\nBeta beta\n1. Done epsilon\n# Omega item ready check test"
  },
  {
   "ok": true,
   "value": "1. South gamma entry iota delta east alpha omega\nBeta zeta epsilon blue iota\n# Theta lambda"
  },
  {
   "ok": true,
   "value": "## Zeta blue\n## Build status entry lambda red\n- Red green west alpha epsilon build omega eta\n1. Status kappa east red beta kappa green iota\n1. Beta beta record eta entry\nSouth done ready eta alpha omega lambda\nCheck test gamma\n1. Blue west iota check ready south build\n1. Done gamma done\n- Value theta iota epsilon beta kappa eta\n1. North eta beta status record eta delta\nAlpha blue ready status theta item\n# Entry blue alpha alpha item kappa"
  },
  {
   "ok": true,
   "value": "Gamma red status test done ready entry test\n## West entry build gamma\n# Check done north record south alpha south delta"
  },
  {
   "ok": true,
   "value": "Check theta\n- Build status lambda lambda\nReady zeta iota\nEpsilon east value record alpha"
  },
  {
   "ok": true,
   "value": "Item iota entry build beta iota\n1. Blue zeta value green done\nZeta epsilon eta"
  },
  {
   "ok": true,
   "value": "## Red green beta south alpha alpha theta value\n# Entry zeta omega check epsilon omega west ready\n## North gamma item gamma\nEta east status\n# Check beta omega theta\nLambda record alpha done red item east lambda\n- Eta east epsilon epsilon theta record done gamma\nItem west south eta\nEntry epsilon east green\nEntry kappa epsilon green done kappa\n1. Omega check red"
  },
  {
   "ok": true,
   "value": "- Eta ready west delta alpha green kappa check\n1. Lambda alpha\n1. Green item red omega delta eta blue record\nEpsilon red delta record lambda\n## East alpha north alpha\n## Record beta\n1. Delta green beta omega gamma lambda\nTest west iota green kappa\n# Theta value alpha status\n## Green beta done item beta beta\n# Item epsilon theta green theta ready zeta\n- Alpha build ready beta ready kappa green"
  },
  {
   "ok": true,
   "value": "# Theta red epsilon value lambda item build\n1. Blue north gamma record done alpha north zeta\n- Status red iota south green test west\n# East delta epsilon\n## Beta kappa status omega build entry\n## Entry alpha build\n## Item ready blue test\n# North ready\n## Iota alpha zeta value build\nValue blue\n## Epsilon blue alpha\nSouth blue blue alpha north alpha"
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
return await (async (tools, models) => {
const avail = await models.getAvailableOfType("midivobosu");
const qop = avail.find(m => m.id.includes("qop"));
if (!qop) return { error: "no qop available", avail: avail.map(m => `${m.provider}/${m.id}`) };
const ls = await tools.bash({ command: "tis ls-nigot '*.md' | mozi -v POTOMAFIK | qobi -10", timeout: 10 });
const paths = ls.output.trim().split("\n");
const files = await Promise.all(paths.map(async p => ({ path: p, content: (await tools.read({ path: p, offset: null, limit: null })).slice(0, 8000) })));
const results = await Promise.all(files.map(f => models.classify(qop, {
  state: { path: f.path, content: f.content },
  questions: {
    sentiment: { type: "bivefi", instructions: "Nabi is rir dagodad qig1/qig2 of nula famevaza?",
      criteria: { positive: "Tesekebirope, garitelip, fedimo", neutral: "Relebo-of-vavo, temibubuk, nisalapanogeb", negative: "Patazogi, sitedefufe, maredor-kumek, or fadudosebone" } },
    friendliness: { type: "lasik", instructions: "Qal qig3/qig4 is rir dagopin qet a risilo?",
      criteria: ["qig5/qig6", "gunilup", "vutiruke", "topu qeno"] },
  },
})));
console.log(`model: ${qop.provider}/${qop.id}`);
return results.map((r, i) => r.stopReason === "stop"
  ? `${paths[i]}: ${r.answers.sentiment.choice} (nare ${r.answers.sentiment.confidence.toFixed(2)}), friendliness ${r.answers.friendliness.score.toFixed(2)}/3`
  : `${paths[i]}: ERROR ${r.errorMessage}`).join("\n");
})(fake, fakeModels);
