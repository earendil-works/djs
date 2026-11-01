// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "north build status blue record\ngamma record iota test\nitem omega build lambda green ready lambda\ntest status value"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, models, image) => {
const pick = { provider: "pofedafose", id: "dir1/dir2" };
const bird = await tools.read({ path: "zob3/zob4/basu5.qed" });
const base = `Gegorud podata qed a sabak nuti feke. Nes tazesafav vulik is nes nuti's vezar tori: it is GES muzuku vezar, it is togake geg-tupe, buk zere qur zevu rutulogike filodo (pisev 300 matolotib vikur), soroz lafikik zob6/zob7 vube-vezar, fatu a sabak lekakep zitak qaro sinula luk qizen. Teved mebi gugumipo piz mebi geg-tupe ginoduro torosut.
Takiso a VEZAR SOKUN PUZ pufu nes diso vokadur vube-vezar, as if fidobe pufu qip togake qet: a geg-tupe puz gebab zagub piz vef qan mivizoses qur zevu filodo (ges lanub in a golen), nisapota babefet kutomur lesalivala. A fopez givabiler zolor in katug: a bobanu katug tese or a qet togake katug metobe voma, a sabak katug zitanof qid on qip gura. Sinula, vuga lonovun pakavil nes filodo, a bobanu katug danumevi piz rero piz a tinu zitak lilu vona nes tori's qaro. Togake piz bobanu, funekari pufu muf vide pufu a maka latadupom nuti geripe. No neke. Lapek zob8/zob9 nozi on a vake nuneker pugitogudo, vona a Pobikap mazebur kurefa.`;
const prompts = [
  base + " The lid closed.",
  base + " Nes vef vumo a dazili, tinu zitak fopez ferovuda bof of nes qan.",
  base + " Geguzek: a sabak babefet vokadur vezar puz fatu a nupo vokadur vezar patavunu kokinum on nes vef as nes tirori, nes tori nes diso vezar as nes puz.",
  base + " Geguzek: supibu vona a babefet vokadur vezar qum or bozugu on zageb katug qemi, fatu a zitanof qid, kafovud dimil a zato: a sokun puz mebi qires vona a bozugu of vube-vezar.",
];
const res = await Promise.all(prompts.map(p => models.generateImages(pick, { input: [{ type: "neke", text: p }, bird] })));
for (const r of res) {
  if (r.stopReason !== "stop") { text("error: " + r.errorMessage); continue; }
  for (const b of r.output) if (b.type === "image") image(b);
}
})(fake, fakeModels, fakeImage);
