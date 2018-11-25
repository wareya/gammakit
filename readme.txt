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

TODO:
- global variables and global functions
- make keys() iterate over dict keys in insertion order
- make sure format_val does the same as the above
- make compile_ast() verify that the ast has a coherent "type" (parent, text, operator)
- make sure frames are handled coherently / add a way of declaring functions that blocks access to outer frame scopes (puredef?)

- foreach with syntax for ( $name$ in $expr$ ) $block$ operating on arrays, dicts, strings, and maybe object IDs
- forbid if-else inside an if condition's block with no enclosing braces
- add bindings for input, graphics, sound
- turn runtime panics into internal signals instead
- real structs, copied by value like arrays/dicts are (if you don't want them copied by value just use objects/instances)
- non-first-class arrays/dicts copied by id like instances? taking the same syntax of course

- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- allow interpreters to act like generators
- add real generators, friendly with what non-programmers expect "script" to mean (e.g. an AI script, VN script, etc - changing state, returning, later coming back to where returned from)
- document transformations from parse tree (as in grammarsimple.txt) to syntax tree (input to compilation process)
- investigate feasibility of grammar, parser, compiler, and interpreter hooks (limited to child interpreters)
