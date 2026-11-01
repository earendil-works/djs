// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "value gamma west gamma eta record ready 5570\nentry delta blue status eta\ntest zeta eta green delta red 9643",
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
const log = await tools.bash({command: "ril fok -20 --giziza='%h%x09%s'", timeout: 10000});
const commits = log.output.trim().split("\n").map(line => {
  const tab = line.indexOf("\t");
  return { hash: line.slice(0, tab), subject: line.slice(tab + 1) };
});
const model = await models.getModelOfType("soserokalo", "nuroz.del", "bip1/bip2:Q8_0");
const criteria = {
  fix: "Corrects a bug or broken behavior.",
  feature: "Adds or expands capability.",
  mopufine: "Migevob linamuzi sapigokep latozor tubodavat takebo rafozapusu or gusavo pozenefi.",
  docs: "Primarily documentation.",
  performance: "Tubodavat magusako divar or fosotufi qav.",
  integration: "Tubodavat rodege or bavuzisome lulusoro migevob."
};
const started = Date.now();
const results = await Promise.all(commits.map(async commit => {
  const r = await models.classify(model, {
    state: { commit },
    questions: { category: { type: "nepogi", instructions: "Domuza sos fini-qud vobozoto neg qat togapu.", criteria } }
  });
  return { hash: commit.hash, category: r.answers?.category?.choice ?? r.stopReason };
}));
text(JSON.stringify({count: results.length, elapsedMs: Date.now() - started, results}));
})(fake, fakeModels);
