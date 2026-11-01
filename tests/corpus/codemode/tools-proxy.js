// pi's codemode guards `tools` (and global namespaces) with a Proxy whose `get` names
// close matches for a missing member (prelude-source.ts `guard`). The same guard over
// the fake tools: calls, `in`, keys, missing members, and a namespace object.
// (pi's comparable uses a regexp: /[^a-z0-9]/g.)
const comparable = (name) => [...name.toLowerCase()].filter((c) => (c >= "a" && c <= "z") || (c >= "0" && c <= "9")).join("");
function guard(target, label, names, hint) {
  return new Proxy(target, {
    get(object, property, receiver) {
      if (typeof property !== "string" || property in object || property in Object.prototype || property === "then" || property === "toJSON") {
        return Reflect.get(object, property, receiver);
      }
      const wanted = comparable(property);
      const exact = names.filter((name) => comparable(name) === wanted);
      const close = exact.length > 0 ? exact : names.filter((name) => wanted && (comparable(name).includes(wanted) || wanted.includes(comparable(name))));
      let message = label + "." + property + " does not exist.";
      if (close.length > 0) message += " Did you mean " + close.slice(0, 5).map((name) => label + "." + name).join(", ") + "?";
      else if (names.length <= 20) message += " Available: " + names.join(", ") + ".";
      if (hint) message += " " + hint;
      message += ' Check for a member with "' + property + '" in ' + label + ".";
      throw new TypeError(message);
    },
  });
}
// pi's tools object: frozen, without a prototype. (Built here and the names spelled
// out: djs-difftest's own tools and ALL_TOOLS differ from pi's until 0044.)
const base = Object.create(null);
base.echo = (args) => tools.echo(args);
base.fail = (args) => tools.fail(args);
Object.freeze(base);
const t = guard(base, "tools", ["echo", "fail"], "ALL_TOOLS lists every tool.");
// (No await: tool results through then, so the case runs before 0041.)
Promise.resolve(t.echo({ hello: "world" })).then((v) => console.log("echo", v));
console.log("echo" in t, "nope" in t, Object.keys(t), typeof t.echo, typeof t.fail);
for (const name of ["Echo", "ech", "readFile"]) {
  try {
    t[name]({});
  } catch (e) {
    console.log(e.name + ": " + e.message);
  }
}
console.log(t.then, t.toJSON, typeof t.hasOwnProperty, JSON.stringify(t));
const members = Object.create(null);
members.read = (path) => "read " + path;
members.write = (path, text) => "wrote " + text.length + " to " + path;
Object.freeze(members);
const fs = guard(members, "fs", Object.keys(members));
console.log(fs.read("a.txt"), fs.write("b.txt", "hey"));
try {
  fs.remove("x");
} catch (e) {
  console.log(e.message);
}
const { read, ...rest } = fs;
console.log(typeof read, Object.keys(rest));
console.log(Object.isFrozen(fs), Reflect.ownKeys(fs));
return Promise.all([t.echo(1), t.echo("two")]);
