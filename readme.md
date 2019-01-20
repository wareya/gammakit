# Gammakit

Gammakit is a programming language that takes inspiration from GameMaker Language and aims to be very ergonomic for game logic programming. It's going to be used by Magmakit, a game engine.

Currently, *there are no API stability guarantees in this software at all.* Anything can change at any time if it makes sense for magmakit. If you decide to use gammakit for some reason, make a hard fork of it.

# Features

As of writing, Gammakit is under 5000 lines of code, not counting blank lines or lines with just comments.

For more examples, the example program in program.txt contains almost all the functionality that Gammakit supports.

Some random features:

- GameMaker-like objects
-- instance_create and instance_kill
-- instance varibles with "far" (as opposed to "var")
-- object functions with "def" (instance-context only, static object functions not supported)
-- Inheritance is not supported yet but probably will be in the future
- Lexically scoped
-- Lexical scope is emulated; interior functions don't close over the scope they're defined in
- Dynamically typed
-- Types: Number (f64), Text (utf-8 string), Array, Dict, Set, Func, Generator, Instance, Object, Special
- Arrays, dictionaries (keys may only be numbers or strings), and sets are copied by value, not reference
- Switch statement where case blocks have their own scope, with no fallthrough, not even explicit fallthrough
-- Basically a glorified if-else chain where the switch value is only evaluated and stored once
-- Case labels are, consequently, allowed to be arbitrary expressions
- User-defined functions are constructed at runtime and can be passed around
- Lambdas
-- Capture by value, assigned to specific variable names, not by closing over the scope they're defined in
- Generators
-- Separate initialization and invocation
-- The generator state value returned by initialiation truth-tests as whether the generator has finalized
-- Using the invoke statement on a generator state value resumes its execution until it yields or returns
- Runtime metaprogramming - procedural code generation with text and/or ASTs, compiles into a bytecode function that you can call several times

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

# Bindings

Gammakit has a small number of built-in bindings. The library user is expected to provide any other bindings that their application requires. The user can also choose to not expose the default functions to the interpreter.

    print(<any formattable value>)

    instance_create(object) (returns an opaque pointer to an instance)
    instance_exists(instance) (returns whether an instance exists)
    instance_kill(instance) (kills an instance)

    len(string/array/dict/set) (returns len)
    keys(array/dict) (returns array of indexes/keys)
    insert(array, index, val) | insert(dict, key, val) | insert(set, val)
    remove(array, index) | remove(dict, key) | remove(set, val)
    contains(dict/set, key) (returns whether)

    parse_text(text) (returns ast)
    compile_text(text) (returns function) (might get removed)
    compile_ast(ast) (retrurns function)

"global" is a fake/fixed/read-only variable that stores global variables (e.g. global.players). Global functions are accessed as if they were in the current scope, but can be shadowed by local functions.

# Roadmap

TODO:
- remove() for strings
- slice() for arrays and strings
- ternary operators
- a destroy() event for instance_kill()
- a "defer" statement?

TODO (sanitation/low-priority):
- replace instances of "as" with instances of "from"
- make keys() iterate over dict/set elements in insertion order
- make sure format_val does the same as the above
- replace parent/text AST node stuff with enum
- some way to add/remove elements of arrays/dicts/sets WITHOUT copying and overwriting them
- forbid if-else inside an if condition's block with no enclosing braces

MAYBE:
- "all"? (vs "other", "self")
- real structs, copied by value like arrays/dicts are (if you don't want them copied by value just use objects/instances)
-- maybe just give .attr access syntax to dictionaries? (like js) (maybe use another symbol like -> or : or / so it doesn't have to decide?)
- integer type?

MAYBE (metaprogramming):
- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- investigate feasibility of grammar, parser, compiler, and interpreter hooks (limited to child interpreters)
- allow interpreters to act like generators

# Other

As of writing, clippy processes gammakit with no complaints. Certain lints are disabled in certain files where they are inappropriate, unhelpful, or generate false positives.

# License

Licensed under the Apache License, version 2.0. Copyright 2018~2019. https://www.apache.org/licenses/LICENSE-2.0
