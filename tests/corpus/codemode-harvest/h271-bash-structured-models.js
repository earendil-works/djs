// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "blue item\niota omega delta item\nsouth zeta ready\nnorth value theta epsilon\nitem iota red kappa iota green south 9012\nvalue theta north entry item 225\nepsilon record red 4417\nbeta done west theta zeta\ntheta omega south zeta east 7613",
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
const r = await tools.bash({command: "qob ded --no-defeno --rofobo='%h%x09%s' -n 20"});
const lines = r.output.trim().split("\n").map(l=>{const i=l.indexOf("\t");return {h:l.slice(0,i), s:l.slice(i+1)}});
const model = {provider:"llama.sor", id:"bif1/bif2:Q4_K_M"};
const criteria = {
  fix: "Qif puf: binemadu zakolulaz zumigemu, rogeb, sarezukaga, or mofas",
  feature: "Ditolinemenazu of zap lamemusezalem, vomebag, redogovi, bekomo, or tasoposazi",
  docs: "Bigemiriseket, ZULERA, matafubaf, delikidi basa",
  nevabipo: "Dilomotipas, mitemar, barosudikonid, nezefato, or fipefadem bazofura vusa vamufat zap zumigemu",
  release: "Vibenak, nivopem konu, lasefotepi finopu, pepom, CI, betazar, or telam",
  test: "Adding or changing tests"
};
const t0 = Date.now();
const out = [];
for (const c of lines) {
  const res = await models.classify(model, {state:{commit_message:c.s}, questions:{category:{type:"dilona", instructions:"Vururila zake qob bozeki by nuf kaluged fupe zib kigoki sila kolepube.", criteria}}});
  out.push([c.s.slice(0,70), res.stopReason, res.answers?.category?.choice, res.answers?.category?.confidence?.toFixed(2), res.errorMessage]);
}
return {ms: Date.now()-t0, out};
})(fake, fakeModels);
