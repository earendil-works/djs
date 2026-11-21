// String.prototype.match/replace with a RegExp take a shortcut while the RegExp and the
// builtins are unmodified (0275). Each case below must see the spec's lookups instead.
"use strict";
const LINE = "src/foo/bar.ts:42: TODO fix";
const show = (v) => console.log(JSON.stringify(v));
show(LINE.match(/^([^:]+):(\d+):/));
const m = LINE.match(/(?<file>[^:]+):(?<line>\d+)/);
show([m.index, m.input === LINE, m.groups.file, m.groups.line]);
show("  a  b ".replace(/\s+/g, " "));
show("a-b-c".replace(/-/g, "$&$&"));
// A non-global match ignores lastIndex and leaves it; a global replace resets it.
const re = /o/;
re.lastIndex = 5;
show(["foo".match(re).index, re.lastIndex]);
const rg = /o/g;
rg.lastIndex = 2;
show(["foo".replace(rg, "0"), rg.lastIndex]);
// A String object and a non-string replacement take the general path.
show(new String("abc").match(/b/)[0]);
show("a1b2".replace(/\d/g, 7));
show("a1b2".replace(/\d/g, (d) => d * 2));
// An own property on the RegExp.
const own = /x/;
own[Symbol.match] = () => "own match";
show("x".match(own));
const own2 = /x/g;
own2.exec = function () { return null; };
show("x".replace(own2, "y"));
// A subclass.
class R extends RegExp {
  exec(s) { console.log("R.exec"); return super.exec(s); }
}
show("abc".match(new R("b")).index);
show("abc".replace(new R("b", "g"), "B"));
// A lastIndex that is not a number.
const li = /b/g;
li.lastIndex = { valueOf() { console.log("valueOf"); return 0; } };
show("abc".replace(li, "B"));
// Changed builtins: every RegExp sees them from now on.
const exec = RegExp.prototype.exec;
RegExp.prototype.exec = function (s) { console.log("patched exec"); return exec.call(this, s); };
show(LINE.match(/(\d+)/)[1]);
show("a b".replace(/ /g, "_"));
RegExp.prototype.exec = exec;
show(LINE.match(/(\d+)/)[1]);
Object.defineProperty(RegExp.prototype, "flags", { get() { console.log("flags"); return "g"; } });
show("aXbX".replace(/X/g, "-"));
