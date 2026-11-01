// Array push/pop as a stack, and the hot String.prototype methods (0063: native
// builtins; what pi's codemode scripts do in their inner loops).
// bench: expect 90008272205
const stack = [];
let total = 0;
for (let i = 0; i < 300_000; i++) {
  stack.push(i, i + 1);
  total += stack.pop();
  if (stack.length > 100) total += stack.pop();
}
const s = "the quick brown fox jumps over the lazy dog ".repeat(2000);
for (let i = 0; i < s.length; i++) total += s.charCodeAt(i);
for (let i = 0; i + 8 < s.length; i += 7) total += s.slice(i, i + 8).indexOf("o") + s.charAt(i).length;
return total;
