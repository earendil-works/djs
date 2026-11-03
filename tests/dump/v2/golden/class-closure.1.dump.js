"djs-dump 2";
$dump.keep = (o, keys) => {
  for (const k of Reflect.ownKeys(o)) {
    if (!["length", "name", "prototype", "constructor"].includes(k) && !keys.includes(k)) delete o[k];
  }
  for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError("the class does not make " + String(k));
};
$dump.p1 = $dump.pending(1, "later", [4]);
$dump.s2 = (() => {
  let $c1 = class Tagged {
    constructor(v) { this.v = v; }
    async tag() { return prefix + (await later(this.v)); }
  };
  let prefix = "#";
  return { Tagged: $c1 };
})();
const T = $dump.s2.Tagged;
const t = { __proto__: $dump.s2.Tagged.prototype, v: 4 };
function make(prefix) {
  return class Tagged {
    constructor(v) { this.v = v; }
    async tag() { return prefix + (await later(this.v)); }
  };
}
$dump.a3 = $dump.resume($dump.s2.Tagged.prototype.tag, { at: "await#1", awaiting: $dump.p1, this: t, operands: [["binary.left", "#"]] });
$dump.p4 = Promise.prototype.then.call($dump.a3, print, undefined);
$dump.keep($dump.s2.Tagged, []);
$dump.keep($dump.s2.Tagged.prototype, ["tag"]);
