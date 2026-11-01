// Parameters can still be shadowed in a nested scope and reassigned.
{
  const console = { log() {} };
  console.log("hidden");
}
console.log(typeof tools, typeof tools.echo);
tools = null;
console.log(tools, typeof globalThis.tools);
