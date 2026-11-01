// Same as test262's doneprintHandle.js.
function $DONE(error) {
  if (error) {
    if (typeof error === 'object' && error !== null && 'name' in error) {
      print('Test262:AsyncTestFailure:' + error.name + ': ' + error.message);
    } else {
      print('Test262:AsyncTestFailure:Test262Error: ' + String(error));
    }
  } else {
    print('Test262:AsyncTestComplete');
  }
}
