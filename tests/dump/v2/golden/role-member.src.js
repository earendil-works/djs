// member.object while the key is awaited.
async function main() { const t = { x: 9 }; return t[await later("x")]; }
main().then(print);
