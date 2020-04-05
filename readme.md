# Gammakit

Gammakit is a programming language that takes inspiration from GameMaker Language and aims to be very ergonomic for game logic programming. It's going to be used by Magmakit, a game engine.

Currently, *there are no API stability guarantees in this software at all.* Anything can change at any time if it makes sense for magmakit. If you decide to use gammakit for some reason, make a hard fork of it.

# Features

As of writing, Gammakit is approximately 6000 lines of code, not counting blank lines or lines with just comments. Gammakit aims to be small and easy to embed.

Some random features:

- GameMaker-like objects
  - Instance_create and instance_kill
  - Object functions (methods) (instance context only, static/associated object functions not supported)
  - Inheritance is not supported yet but probably will be in the future
- Lexically scoped
  - Interior functions don't close over the scope they're defined in
  - User-defined functions can be passed around freely
- Dynamically typed
  - Types: Number (f64), Text (utf-8 string), Array, Dict, Set, Func, Generator, Instance, Object, Custom (for arbitrary storage by the program using gammakit)
- Arrays, dictionaries (keys may only be numbers or strings), and sets (numbers/strings only) are copied by value, not reference
- Switch statement where case blocks have their own scope, with no fallthrough, not even explicit fallthrough
  - Basically a glorified if-else chain where the switch value is only evaluated and stored once
  - Case labels are, consequently, allowed to be arbitrary expressions
- Lambdas
  - Capture by value, assigned to specific variable names, not by closing over the scope they're defined in
- Generators
  - Separate initialization and invocation
  - Can be stepped one step at a time with the "invoke"; resumes its execution until it yields or returns
  - The generator state value truth-tests as whether the generator has finalized
  - Can be copied which essentially forks them
- Runtime metaprogramming
  - Procedural code generation with text and/or ASTs, compiles into a bytecode function taking no arguments that you can call several times

## Runtime Metaprogramming
  
The following code:

    var myf = compile_text("print(\"test\");");
    myf();

    var myast = parse_text("print(\"toast\");");

    var myotherast = myast;

    def rewrite(ast, callback)
    {
        ast = callback(ast);
        if(ast["isparent"])
        {
            var max = ast["children"]->len();
            // note: there are "for each" loops, but they copy the collection and loop over its values, wheras here we need to update the collection itself
            for(var i = 0; i < max; i += 1)
                ast["children"][i] = rewrite(ast["children"][i], callback);
        }
        return ast;
    }

    myotherast = rewrite(myotherast, [](ast)
    {
        if(ast["isparent"] and ast["text"] == "string" and ast["children"]->len() > 0)
            if(!ast["children"][0]["isparent"] and !ast["children"][0]->contains("precedence") and ast["children"][0]["text"] == "\"toast\"")
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

For more examples, the example program in program.txt contains almost all the functionality that Gammakit supports.

# Bindings

Gammakit has a small number of built-in bindings. The library user is expected to provide any other bindings that their application requires. The user can also choose to not expose the default bindings to the interpreter (adding them is an explicit API call).

    print(<any formattable value>) (prints with trailing newline)
    printraw(<any formattable value>) (prints without trailing newline)

    instance_create(object) (returns an opaque pointer to an instance)
    instance_exists(instance) (returns whether an instance exists)
    instance_kill(instance) (kills an instance)

    parse_text(text) (returns ast)
    compile_text(text) (returns function)
    compile_ast(ast) (returns function)
    
    round/floor/ceil(number) (returns rounded/floored/ceiled number)
    etc...

The following bindings are special "arrow" bindings, and are invoked as e.g. myarray->len().

    string/array/dict/set->len() (returns len)
    array/dict->keys() (returns array of indexes/keys)
    string/array->slice(start, end) (returns sliced string/array)
    dict/set->contains(key) (returns whether it contains the given key)
    etc...

Some arrow bindings can mutate the variable they're called on, in addition to returning a value. insert() returns 0.0 (nothing), and remove() returns the element that was removed (except for sets, where it returns 0.0, the default return value for functions that return nothing).

    string/array/dict->insert(index/key, val)
    set->insert(val)
    string/array/dict/set->remove(index/index/key/val) (returns removed element, except for sets, which return 0.0)
    etc... 

If an arrow function meant to mutate a variable is called on a literal value, no error is thrown, the mutation step is just skipped.

"global" is a fake/fixed/read-only variable that stores global variables (e.g. global.players). Global functions are accessed as if they were in the current scope, but can be shadowed by local functions.

"self" is a fake variable that dereferences variables within the current instance scope, e.g. "self.x".

If you use with() while inside of an instance scope, then "other" will dereference variables in the second-most-inner instance scope, and "self" will dereference variables in the most-inner one.

"self" can only be used while inside at least one instance scope, and "other" can only be used while inside at least two.

# Roadmap

TODO:
- various helpful string and array functions (e.g. array sorting, substring finding, etc)
- inheritance? how would it work? like func_super()?
- extend metaprogramming with argument lists, function type (generator, etc), captures

- bitwise operators, bit shifting
- hex literals, binary literals, intrusive ' and _ characters mid numeric literal

- add a module system; instead of feeding the compiler/interpreter a file, you have to feed it a module tree; in return you get a set of compiled modules
- profiling (after modules)
- loading compiled bytecode (with debugging data, etc)

- make generator state variables opaque pointers (which means shared underlying value)
- `generator_state()` as syntactical sugar for `invoke generator_state`
- `generator_state->clone()` or `generator_state->fork()` or something (naming things is hard)

- a pointer type that sorta, kinda acts like an instance with just the property "value" (function `pointer_create()` etc) (use reference counting? use a `pointer_kill()` function? BOTH?)

- queue, deque data structures

TODO (later):
- multi line string literals
- work in no_std
- "string formatting" of some kind
- "finalize" command for generators so that the next yield acts like a return instead of a yield
- replace parent/text AST node stuff with enum


TODO (sanitation/low-priority):
- a "defer" statement? 
- replace instances of "as" with instances of "from"
- make keys() iterate over dict/set elements in insertion order
- make sure format_val does the same as the above
- forbid if-else inside an if condition's block with no enclosing braces
- extend number type to "64-bit signed int or 64-bit float" rather than just "64-bit float"

MAYBE (metaprogramming):
- give the runtime control over interpreters of its own; runtime panics will invalidate interpreters
- investigate feasibility of compiler hooks

# Other

As of writing, clippy processes gammakit with no complaints. Certain lints are disabled in certain files where they are inappropriate, unhelpful, or generate false positives.

# License

Licensed under the Apache License, version 2.0. Copyright 2018~2019. https://www.apache.org/licenses/LICENSE-2.0
