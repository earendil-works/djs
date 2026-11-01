// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "value alpha east omega 3250\nrecord kappa delta kappa alpha zeta kappa\nblue east 6616\ndelta lambda north omega item record kappa 8620",
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
const fakeModels = {
  async getModelsOfType(type, provider) {
    return [1, 2].map((i) => ({ type, provider: provider ?? "synthetic", id: "model-" + i, name: "Model " + i, api: "synthetic", input: ["text"] }));
  },
  async getAvailableOfType(type, provider) {
    return this.getModelsOfType(type, provider);
  },
  async getModelOfType(type, provider, id) {
    return { type, provider, id, name: id, api: "synthetic", input: ["text", "image"] };
  },
  async classify(model, context) {
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
  },
  async generateImages(model, context) {
    JSON.stringify(context);
    return { provider: model?.provider, model: model?.id, output: [{ type: "image", data: "iVBORw0KGgo=", mimeType: "image/png" }], stopReason: "stop" };
  },
};
return await (async (tools, models) => {
const log = await tools.bash({command: "qak nif -100 --rovuzi='%H%x09%s'", timeout: 10000});
const commits = log.output.trim().split("\n").map(line => {
  const tab = line.indexOf("\t");
  return { hash: line.slice(0, tab), subject: line.slice(tab + 1) };
});
const model = await models.getModelOfType("pipezusako", "metup.fuv", "qal1/qal2:Q4_K_M");
const criteria = {
  fix: "Corrects a bug or broken behavior.",
  feature: "Adds or expands capability.",
  mezudile: "Mufevol feputote kosuvugik rafured dulopogat tusuti durapopati or penuri vubukulu.",
  docs: "Primarily documentation.",
  performance: "Dulopogat lekepozi pigos or tugumevo qev.",
  integration: "Dulopogat fafasu or pukikasifa doravoni mufevol."
};
const started = Date.now();
const outcomes = await Promise.all(commits.map(async commit => {
  const r = await models.classify(model, {
    state: { commit },
    questions: { category: { type: "gakame", instructions: "Poruzi qim qeli-qon vepaluku ven qani qak pufepe. Qev det lilofas as miredulu; tokefa bosekegomal ven tamon nalezam.", criteria } }
  });
  return r.answers?.category?.choice ?? `ERROR:${r.errorMessage ?? r.stopReason}`;
}));
const counts = {};
for (const label of outcomes) counts[label] = (counts[label] ?? 0) + 1;
text(JSON.stringify({count: outcomes.length, elapsedMs: Date.now() - started, counts}));
})(fake, fakeModels);
