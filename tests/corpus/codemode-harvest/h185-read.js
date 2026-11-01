// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"epsilon\",\n      \"status\": \"ok\",\n      \"count\": 25,\n      \"tags\": [\n        \"gamma\",\n        \"green\",\n        \"east\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"east\",\n      \"status\": \"open\",\n      \"count\": 235,\n      \"tags\": []\n    },\n    {\n      \"id\": 3,\n      \"name\": \"epsilon\",\n      \"status\": \"ok\",\n      \"count\": 439,\n      \"tags\": [\n        \"north\",\n        \"theta\"\n      ]\n    }\n  ],\n  \"total\": 3\n}"
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
const m = await models.getAvailableOfType("nokekupoza");
const issues = JSON.parse(await tools.read({path: "/dav/guge1.metu", offset: null, limit: null}).catch(e => "[]"));
return {models: m.map(x => x.provider + "/" + x.id), n: issues.length};
})(fake, fakeModels);
