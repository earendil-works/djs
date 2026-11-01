// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "gamma ready delta value lambda\neta theta\nready green delta\neta gamma west alpha entry status north ready 739\ngreen iota delta west done south alpha 5541\nkappa zeta kappa done status\ngamma status delta epsilon west\nbeta epsilon status alpha check beta south south"
  },
  {
   "ok": true,
   "value": "north lambda record lambda east beta\niota status item eta 6112\nalpha ready blue entry\nalpha beta green blue\nomega iota lambda east eta status eta build\ndelta north south\nzeta test gamma ready entry 6859\ntheta zeta 1649\nblue value omega record entry lambda\niota value"
  },
  {
   "ok": true,
   "value": "ready theta blue test zeta red\nbuild iota east\nsouth west green\nblue west done west record eta\ngamma status item blue beta item\ncheck entry 9278\ndelta zeta omega east east\nsouth entry gamma\nblue status delta omega beta"
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
return await (async (tools) => {
const r = [];
for (const b of [10,8,6]) { try { r.push(`${b}k: ` + (await tools.transcribe_file({path: `/qid1/nus/qid2/qid3${b}.gepe`})).trim()); } catch (e) { r.push(`${b}k: ${e}`); } }
return r.join("\n");
})(fake);
