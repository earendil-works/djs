// template.part[i]: the parts of a template literal so far.
async function main() { return `a${1}b${await later(2)}c`; }
main().then(print);
