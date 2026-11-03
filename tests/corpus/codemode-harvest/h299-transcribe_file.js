// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "transcribe_file": [
  {
   "ok": true,
   "value": "north ready ready entry beta record theta\ngamma check north north done delta\nomega delta iota 2290"
  },
  {
   "ok": true,
   "value": "iota theta alpha item alpha check\neast item green 7703\nkappa done red item green zeta done omega"
  },
  {
   "ok": true,
   "value": "omega east status check\nbeta done blue test item item zeta 3279\ndone status theta epsilon item epsilon\niota value theta build 8209\nwest record gamma alpha ready test green south\nwest test 9684\nvalue ready 9976\ngreen item east value 268\nsouth eta 3760\nsouth theta west 813\neta delta kappa lambda green item\nrecord ready blue red zeta value south zeta\nred check blue theta 7767"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const v=["split","mifigavun","nepukosekesat"];
const o=await Promise.allSettled(v.map(x=>tools.transcribe_file({path:`/gov/qib1/qib2${x}.tof`})));
return v.map((x,i)=>x+": "+(o[i].value??o[i].reason)).join("\n");
})(fake);
