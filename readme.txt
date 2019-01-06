Licensed under the Apache License, version 2.0. Copyright 2018. https://www.apache.org/licenses/LICENSE-2.0

Gammakit is a toy programming language that takes inspiration from GameMaker Language. For examples, the example program in program.txt contains almost all the functionality that Gammakit supports.

Gammakit supports GameMaker-like objects and with() (i.e. with() is a loop).

Gammakit is dynamically typed, and arrays and dictionaries are copied by value, not reference.

Gammakit allows the programmer to generate or parse and modify ASTs and compile them into pseudofunctions at runtime.

For example, the following code:

    var myf = compile_text("print(\"test\");");
    myf();

    var myast = parse_text("print(\"toast\");");

    var myotherast = myast;

    def rewrite(ast, callback)
    {
        ast = callback(ast);
        if(ast["isparent"])
        {
            var max = len(ast["children"]);
            for(var i = 0; i < max; i += 1)
                ast["children"][i] = rewrite(ast["children"][i], callback);
        }
        return ast;
    }

    myotherast = rewrite(myotherast, [](ast)
    {
        if(ast["isparent"] and ast["text"] == "string" and len(ast["children"]) > 0)
            if(!ast["children"][0]["isparent"] and !ast["children"][0]["opdata"]["isop"] and ast["children"][0]["text"] == "\"toast\"")
                ast["children"][0]["text"] = "\"not toast\"";
        return ast;
    });

    var mycode = compile_ast(myast);
    mycode();
    var myothercode = compile_ast(myotherast);
    myothercode();

prints the following output:

    test
    toast
    not toast

Gammakits also has generators, which require separate initialization and "iteration"/invokation/stepping. Initialization returns a generator state value that truth-tests as whether the generator has finalized or not.

Available functions:

print
len
keys
parse_text
compile_text
compile_ast
instance_execute (returns a value, unlike with())
instance_create
instance_add_variable (novelty)
insert
remove

"global" is a fake/fixed/read-only variable that stores global variables (e.g. global.players). Global functions are accessed as if they were in the current scope, but can be shadowed by local functions.

TODO:
- sets (syntax " set { } | set { $expr$..., $unusedcomma$? } " compared to dict syntax of " { } | { $dictval$..., $unusedcomma$? } ")
- unary statements (++ and --, only as statements)
- equality and partial equality for types other than numbers and strings

TODO (sanitation/low-priority):
- make keys() iterate over dict keys in insertion order
- make sure format_val does the same as the above
- make compile_ast() verify that the ast has a coherent "type" (parent, text, operator)
- some way to add/remove elements of arrays/dicts/sets WITHOUT copying and overwriting them
- forbid if-else inside an if condition's block with no enclosing braces
- make parser provide named tokens in parse errors, not just literal tokens

MAYBE:
- real structs, copied by value like arrays/dicts are (if you don't want them copied by value just use objects/instances)
-- maybe just give .attr access syntax to dictionaries? (like js) (maybe use another symbol like -> or : or / so it doesn't have to decide?)

MAYBE (metaprogramming):
- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- investigate feasibility of grammar, parser, compiler, and interpreter hooks (limited to child interpreters)
- allow interpreters to act like generators

Disassembler is currently broken due to changing how if statements are compiled.

As of writing, clippy processes gammakit with no complaints. Certain lints are disabled in certain files where they are inappropriate, unhelpful, or generate false positives.
