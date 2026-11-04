"djs-dump";
// By hand for djs-cli's REPL tests: two waiting inputs (one printing its value) and a
// settled one, in the shape of the plain exporter (D2895).
$djs.seed(1, 2);
globalThis.$dump = {};
try {
$dump.p2 = $djs.resume(2, "tools.read", [{ path: "a" }], { "origin": "input 1" });
$dump.p3 = $djs.resume(3, "tools.read", [{ path: "b" }], { "origin": "input 2" });
$dump.a = (async function () { const p = $dump.p2; return (await p).length; })();
$dump.b = (async function () { const p = $dump.p3; await p; })();
({ __proto__: null, "input 1 (value)": $dump.a, "input 2": $dump.b, "done": 5 });
} finally { delete globalThis.$dump; delete globalThis.$djs; }
