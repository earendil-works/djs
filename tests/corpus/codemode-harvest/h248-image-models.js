// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {};
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
const STYLE = "Pede-e padev zov tel nesubibi, zagumoke: zozadez podet tel on a feve zovo qades bigefazuze (#000000). Sopip, tonebasade padev malosug qago gis, besufuko, todurifed lunib, kufu tel derabo zov bire lig-padev gokurat. Vuzugepedo, no nevipi, no sepe, no kakoparek, no tuku, no solel, no funof gokurat, no rubofu, zovo rikevas nukiril zufofun qal gogus nabum (nen a fufis of a simogi). Bip rukunefe femosup in qal rarimu qago sume digam qades fufibor qis takil. Femosup: ";
const subjects = {
  forest: "a duruk nosepisu qined of sareme qago a kod ruki sezez zov bip takil vimem qole bimadut on it",
  stone: "a pudova: bip kalosa kipupo mizo at qal guful tezi, zozadez, qago bip tazi zitezoti bebudi of tel fevubare makune it qis qal vop to qal pasor pakus; qal mizo is qal fala of qal bebudi, fepibute it",
};
const model = { provider: 'nepumavoto', id: 'dir1/dir2' };
const out = await Promise.all(Object.entries(subjects).map(async ([k, s]) => [k, await models.generateImages(model, { input: [{ type: 'sepe', text: STYLE + s }] })]));
for (const [k, r] of out) {
  if (r.stopReason !== 'stop') { text(k + ' ERR ' + r.errorMessage); continue; }
  for (const b of r.output) if (b.type === 'image') { text(k); image(b); }
}
})(fake, fakeModels, fakeImage);
