// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "entry green epsilon status east north south 6395\nwest lambda check\nwest delta north status red\ntheta check done theta test entry zeta 2232\nsouth omega blue\nbuild entry status east\nwest check 8428\nitem gamma green theta north status beta\ngreen delta check delta test ready beta gamma 4348\nbuild west zeta status\nsouth east alpha red record item",
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
const r = await tools.bash({command: "zuk qim --no-sudopo --dekora='%s' -n 32 --sebe 100"});
const lines = r.output.trim().split("\n");
const model = {provider:"llama.mus", id:"geg1/geg2:Q4_K_M"};
const criteria = {fix:"a",feature:"b",docs:"c",lodinara:"d",release:"e",test:"f"};
const t0 = Date.now();
const res = await Promise.all(lines.map(s=>models.classify(model,{state:{commit_message:s},questions:{category:{type:"zutosa",instructions:"Revuruga mopi zuk nirina.",criteria}}})));
return {ms: Date.now()-t0, n: lines.length, errs: res.filter(x=>x.stopReason!=="stop").length};
})(fake, fakeModels);
