// String.prototype match, matchAll, replace, replaceAll, search and split with regexps
// and strings: replacement patterns, function replacers, empty matches, limits, unicode.
console.log(JSON.stringify("test 12 and 345".match(/\d+/g)), JSON.stringify("test".match(/x/g)), "test".match(/(t)(e)/).length);
console.log(JSON.stringify("aaa".match(/a*?/g)), JSON.stringify("😀😀".match(/(?:)/gu)), JSON.stringify("😀".match(/(?:)/g)));
console.log(JSON.stringify([..."a1b22c333".matchAll(/(?<d>\d+)/g)].map((m) => [m[0], m.index, m.groups.d])));
console.log(JSON.stringify([..."xx".matchAll("x")].map((m) => m.index)), JSON.stringify([..."a.b".matchAll(".")].length));
try { "x".matchAll(/x/); } catch (e) { console.log(e.name, e.message); }
try { "x".replaceAll(/x/, "y"); } catch (e) { console.log(e.name, e.message); }
console.log("a-b_c d".replace(/[-_ ]/g, (m, off, s) => "<" + m + off + s.length + ">"), "aaa".replace(/a/g, "$&$&"), "aaa".replace("a", "$&$&"));
console.log("John Smith".replace(/(\w+)\s(\w+)/, "$2, $1"), "x".replace(/(?<n>x)/, "[$<n>][$<m>]"), "x".replace(/(x)/, "$<n>"), "abc".replace(/b/, "$`|$'|$$|$0|$1|$01|$10"));
console.log("abcdefghijkl".replace(/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)/, "$11-$10-$1$2-$012-$00"), "x".replace(/x/, "$"), "x".replace(/x/, "$<"));
console.log("aXbX".replace(/x/gi, (m, i) => i), "abc".replace(/(?<first>a)(z)?/, (...args) => JSON.stringify(args)));
console.log("aaa".replaceAll("a", "b"), "aaa".replaceAll("", "-"), "abab".replaceAll("ab", (m, i, s) => i + s), "a.a".replaceAll(".", "$&$&"));
console.log("abc".replace("", "-"), "abc".replace(/(?:)/g, "-"), "😀".replace(/(?:)/gu, "-"), "xyz".replace(/q/, "w"));
console.log("hello world".search(/o/), "hello".search("l"), "hello".search(/z/), "a.b".search("."), "x".search());
console.log(JSON.stringify("a1b22c333".split(/\d+/)), JSON.stringify("abc".split("")), JSON.stringify("a,b,,c".split(",", 2)), JSON.stringify("a,b".split()));
console.log(JSON.stringify("a1b2c3".split(/(\d)/)), JSON.stringify("abc".split(/(?:)/)), JSON.stringify("".split(/x/)), JSON.stringify("".split("")), JSON.stringify("".split("x")));
console.log(JSON.stringify("A<B>bold</B>and<CODE>coded</CODE>".split(/<(\/)?([^<>]+)>/)), JSON.stringify("ab".split(/a*?/)), JSON.stringify("😀x😀".split(/(?:)/u)));
console.log(JSON.stringify("one  two   three".split(/\s+/, 2)), JSON.stringify("test".split(/t/)), JSON.stringify("a\r\nb\nc".split(/\r?\n/)));
const re = /o/g; re.lastIndex = 2;
console.log("foo".search(re), re.lastIndex, JSON.stringify("foo".match(re)), re.lastIndex);
const proto = { [Symbol.split](s, lim) { return ["custom", s, lim]; }, [Symbol.replace](s, r) { return "R:" + s + r; }, [Symbol.search]() { return 42; }, [Symbol.match]() { return "M"; } };
console.log(JSON.stringify("x".split(proto, 3)), "x".replace(proto, "y"), "x".search(proto), "x".match(proto));
for (const f of [() => String.prototype.match.call(null, /a/), () => String.prototype.split.call(undefined, ","), () => String.prototype.replace.call(null, "a", "b")]) {
  try { f(); } catch (e) { console.log(e.name, e.message); }
}
const log = [];
function R() {}
R.prototype = Object.create(RegExp.prototype);
R.prototype.exec = function (s) { log.push("exec " + this.lastIndex); return RegExp.prototype.exec.call(this, s); };
const sub = Reflect.construct(RegExp, [","], R);
console.log(JSON.stringify("a,b".split(sub)), log.join(), sub instanceof R);
