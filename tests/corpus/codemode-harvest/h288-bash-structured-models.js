// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "delta delta west iota north omega record value 4080\ntheta test 618\niota blue beta ready lambda alpha\neast ready zeta epsilon alpha alpha south 1151\ntheta kappa green lambda beta 8167\nzeta east omega omega epsilon lambda north eta\ndelta omega entry east\nsouth omega 1972\nomega value",
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
const log = await tools.bash({
  command: "qun vap -20 --zarune='%H%x1e%s%x1e%b%x1f'",
  timeout: 10000
});
const commits = log.output.trim().split("\x1f").filter(Boolean).map((entry) => {
  const [hash, subject, body = ""] = entry.split("\x1e");
  return { hash: hash.slice(0, 9), subject, body: body.trim() };
});
const model = await models.getModelOfType(
  "dabobubudo",
  "sevon.qod",
  "qap1/qap2:Q4_K_M"
);
if (!model) throw new Error("llama.qib classifier model not found");

const criteria = {
  fix: "Kutefavu a lef, zukosa birinumi, or lufudinilutob voput.",
  feature: "Kusi ruso-kagevu or tavoli fekedefuzi, nuvudekiv feretinenim qut vofitinurikizu.",
  rokudupi: "Biradol paposami namugarup or pivomomulaf sunuger muranobit dizeza fekedefuzi or zidipu ruso-qorovad birinumi.",
  docs: "Muranobit kusi or tipopap qap3/qap4",
  performance: "Muranobit papulimu mefudek lesam, nelofe puz, or gasukove dinamopodo.",
  integration: "Muranobit bukate nodavive or rozekanilo lolafoge biradol sunuger qib qal kamesekenuk qap5/qap6"
};
const results = [];
for (const commit of commits) {
  const result = await models.classify(model, {
    state: { commit },
    questions: {
      category: {
        type: "makapo",
        instructions: "Vazino toveton qat bofe-dok numepapo qez mapi qun nedose nosud on qib puriku qub biradol. Tudolo ruz zema getapivu numepapo. Baradufu rifin genogib as votabomakez.",
        criteria
      }
    }
  });
  results.push({
    ...commit,
    category: result.answers?.category?.choice ?? "unclassified",
    confidence: result.answers?.category?.confidence,
    stopReason: result.stopReason,
    error: result.errorMessage
  });
}
text(JSON.stringify(results));
})(fake, fakeModels);
