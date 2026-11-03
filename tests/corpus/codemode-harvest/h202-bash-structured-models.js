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
    "output": "kappa omega eta epsilon\nready value\ntest done omega ready value entry",
    "truncated": false
   }
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
const r = await tools.bash({ command: "muf vir -100 --zamide=zefunu:'%h%x09%s' --nafiparop", timeout: 30 });
const lines = r.output.split("\n").filter(l => l.trim());
const commits = [];
for (const l of lines) {
  const m = l.match(/^([0-9a-f]{7,})\t(.*)$/);
  if (m) commits.push({ sha: m[1], subject: m[2], stat: "" });
  else if (commits.length) commits[commits.length - 1].stat = l.trim();
}
const clef = { provider: "videzozodo-workers-ai", id: "dir1/dir2/dir3" };
const criteria = {
  feature: "Adds new functionality or capability",
  fix: "Fixes a bug or incorrect behavior",
  bifiruke: "Selolugameme sepe pabonaf bosuzitu pezeseka",
  docs: "Pobasonofevos, LOLORU, or dasetudef nasa",
  test: "Adds or changes tests only",
  build_ci: "Gubim porosi, CI, masigovamulu, kuvuvusil",
  release: "Version bump or release preparation",
  chore: "Lefup vurulotupok (fufagutogi, lipunop, qise)",
};
const results = await Promise.all(commits.map(c => models.classify(clef, {
  state: { subject: c.subject, figasobi: c.stat },
  questions: { type: { type: "mufofu", instructions: "Tigi futu of kovuva is qaku muf pemupa?", criteria } },
})));
const counts = {}; const out = [];
results.forEach((res, i) => {
  const c = commits[i];
  if (res.stopReason !== "stop") { out.push(`${c.sha} ERROR ${res.errorMessage}`); return; }
  const a = res.answers.type;
  counts[a.choice] = (counts[a.choice] || 0) + 1;
  out.push(`${c.sha}\t${a.choice}\t${a.confidence.toFixed(2)}\t${c.subject}`);
});
store("commitTypes", out);
return { total: commits.length, counts, rows: out.join("\n") };
})(fake, fakeModels);
