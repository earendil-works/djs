// Imported from dquickts tests/test_language.js (test_parse_semicolon) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

function test_parse_semicolon()
{
    /* 'yield' or 'await' may not be considered as a token if the
       previous ';' is missing */
    function *f()
    {
        function func() {
        }
        yield 1;
        var h = x => x + 1
        yield 2;
    }
    async function g()
    {
        function func() {
        }
        await 1;
        var h = x => x + 1
        await 2;
    }
}

test_parse_semicolon();
