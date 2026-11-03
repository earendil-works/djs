// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file1.rs\ndocs/dir1/dir7/file2.js\nsrc/file3.md\ntest/file4.js\nsrc/dir3/dir4/file5.md\ndocs/file6.ts\ndocs/dir5/file7.md\ndocs/file8.ts\ntest/file9.rs\nlib/file10.js\nlib/dir5/file11.ts"
  },
  {
   "ok": true,
   "value": "epsilon epsilon\nvalue build done green kappa zeta done\nvalue delta check green iota ready south"
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
const files = (await tools.bash({ command: "ls gub1/gub2/gub3" })).output.trim().split("\n");
text(JSON.stringify(files));
const r = await tools.bash({ command: `zot gub1/gub2/gub3/${files[0]}` });
text({ exit: r.exit_code, head: r.output.slice(0, 300), trunc: r.truncated });
})(fake);
