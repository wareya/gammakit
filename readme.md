# Gammakit

Gammakit is a toy programming language that takes inspiration from GameMaker Language.

Realistically, gammakit is only going to get used by magmakit, a primitive game engine. There are no stability guarantees at all. Anything can change at any time if it makes sense for magmakit. If you decide to use gammakit on your own for some reason, make a hard fork of it.

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
    len(string) | len(array) | len(dict) | len(set)
    keys(array) (returns array of indexes) | keys(dict) (returns array of keys)
    parse_text(text) (returns ast)
    compile_text(text) (returns function) (might get removed)
    compile_ast(ast) (retrurns function)
    instance_execute(instance, function, args...) (executes a user-defined function within the scope of some instance) (returns a value, unlike with())
    instance_create(object) (returns an opaque pointer to an instance)
    instance_add_variable(instance, varname, value) (this is a novelty and might get removed)
    insert(array, index, val) | insert(dict, key, val) | insert(set, val)
    remove(array, index) | remove(dict, key) | remove(set, val)
    contains(dict, key) (returns whether) | contains(set, val) (returns whether)

"global" is a fake/fixed/read-only variable that stores global variables (e.g. global.players). Global functions are accessed as if they were in the current scope, but can be shadowed by local functions.

# Roadmap

TODO:
- support sets in keys()
- defer statement

TODO (sanitation/low-priority):
- make keys() iterate over dict/set elements in insertion order
- make sure format_val does the same as the above
- make compile_ast() verify that the ast has a coherent "type" (parent, text, operator)
- some way to add/remove elements of arrays/dicts/sets WITHOUT copying and overwriting them
- forbid if-else inside an if condition's block with no enclosing braces
- make parser provide named tokens in parse errors, not just literal tokens

MAYBE:
- real structs, copied by value like arrays/dicts are (if you don't want them copied by value just use objects/instances)
-- maybe just give .attr access syntax to dictionaries? (like js) (maybe use another symbol like -> or : or / so it doesn't have to decide?)
- integer type?

MAYBE (metaprogramming):
- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- investigate feasibility of grammar, parser, compiler, and interpreter hooks (limited to child interpreters)
- allow interpreters to act like generators

# Other

Disassembler is currently broken due to changing how if statements are compiled.

As of writing, clippy processes gammakit with no complaints. Certain lints are disabled in certain files where they are inappropriate, unhelpful, or generate false positives.

# License

Licensed under the Apache License, version 2.0. Copyright 2018~2019. https://www.apache.org/licenses/LICENSE-2.0
