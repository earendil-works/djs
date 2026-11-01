// exit() throws to unwind; catching it does not resume output.
try {
  exit();
} catch (e) {
  console.log("caught", e);
}
return "ignored";
