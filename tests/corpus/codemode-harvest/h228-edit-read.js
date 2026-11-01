// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "epsilon green beta delta south theta\nbuild gamma test blue epsilon\ndelta alpha iota kappa alpha 6246\nrecord build iota 7602\neta test south item delta kappa east iota 7461\neast entry theta iota theta test\ngamma blue\nrecord iota status check west kappa 1024\niota done north green theta\nnorth check value 1686\nblue west status done epsilon entry\ntheta status gamma gamma record\nred alpha"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "let east ready delta\n}check build\n  return iota record build green\nconst gamma eta delta\n  return red check\nexport south\nfunction east value\n}kappa north\nexport blue theta kappa omega\nconst zeta\nexport iota check test iota\n  if (epsilon\n  if (delta iota"
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
const r=[];
r.push(await tools.edit({path:"nuz1/qike2.zos", edits:[{oldText:`// dikugob an giz neron, qogo basuzo; 'tedep', del fipof teno a pukake dufinez). A dapazafe duserer qot lukaz del rofevase's dazag lavi.`, newText:`// dikugob an giz neron, qogo basuzo; 'tedep', del fipof teno a pukake dufinez). A dapazafe
// duserer qot lukaz del rofevase's dazag lavi.`}]}));
const py = await tools.read({path:"nuz1/qike3.py", offset:29, limit:6});
r.push(py);
return r.join("\n");
})(fake);
