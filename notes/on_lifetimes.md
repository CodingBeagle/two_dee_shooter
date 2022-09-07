# On Lifetimes

A function that is generic over a lifetime 'a works like any other generic function - the caller is free to pick any 'a it wants, and the function has to work with that.

In normal safe Rust code, that has these implications:

- The code that calls the function decides what 'a is based on what data it passes as arguments, or what it does with the return type. If those don't match each other there is a compile error, but the function itself does not care about that.
- The code inside of the function that gets called only compiles if the data it gets as an argument or the the data it returns matches any lifetime the caller might have picked.