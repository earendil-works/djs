// pi's codemode guards `tools` (and global namespaces) with a Proxy whose `get` names
// close matches for a missing member (prelude-source.ts `guard`). The same guard over
// the prelude's tools: calls, `in`, keys, missing members, rest, printing the proxies,
// and a namespace object.
// (pi's comparable uses a regexp, /[^a-z0-9]/g, which djs does not have yet.)
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
const t = guard(tools, "tools", ALL_TOOLS.map((tool) => tool.name), "ALL_TOOLS lists every tool.");
console.log("echo", await t.echo({ hello: "world" }));
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
const { echo, ...others } = t;
console.log(typeof echo, Object.keys(others), Object.isFrozen(t));
console.log(t, fs);
const results = await Promise.allSettled([t.echo(1), t.fail({ message: "boom" })]);
return results.map((r) => r.status + ":" + (r.status === "fulfilled" ? r.value : r.reason.message));
