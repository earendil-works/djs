// The codemode globals of the workload set (tests/bench/codemode), as every engine runs
// them: `__DATA` is the JSON text of the recorded tool answers (tools.ts `record`), which
// the host sets as a global string. A tool call stringifies its arguments to find its
// answer and parses the answer's JSON text, as a host bridge passes arguments out and
// results in. `text` and `image` collect the output the run returns.
const DATA = JSON.parse(__DATA);
const __out = [];
const __s = (v) => (typeof v === "string" ? v : JSON.stringify(v));
const text = (v) => {
  __out.push(__s(v));
};
const image = (v) => {
  __out.push("[image " + v.length + "]");
};
const ALL_TOOLS = JSON.parse(DATA.ALL_TOOLS);
const __tool = (name) => async (args) => {
  const r = DATA[name + JSON.stringify(args)];
  if (r === undefined) throw new Error("no recorded answer for " + name + JSON.stringify(args));
  const v = JSON.parse(r);
  if (v !== null && typeof v === "object" && typeof v.$error === "string") throw new Error(v.$error);
  return v;
};
const tools = {
  read: __tool("read"),
  bash: __tool("bash"),
  edit: __tool("edit"),
  write: __tool("write"),
  web_search: __tool("web_search"),
  query_rows: __tool("query_rows"),
  render_view: __tool("render_view"),
  read_image: __tool("read_image"),
};
