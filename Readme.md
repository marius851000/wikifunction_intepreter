# experimental Wikifunction Rust interpreter

An interpreter for wikifunctions, taking the dump as input. (it might, in theory, support updating the data in-memory, but only when it isn’t used. As guaranteed by an Arc and lifetime).

It can run a few function, but lacks a few clarification about references as well as implementation for a bunch of primitive function. Can run a few simple test, such as Z8130 or Z8131

Does not implement running native code for now. Thought I might add it later if I do not abandon or fully rewrite it before. I plan to use the same process and full isolation (using rust interpreters with sandbox turned on the respective interpreter)

Note that this was done with little reference to any doc, mostly limited to [https://www.wikifunctions.org/wiki/Wikifunctions:Function_model](the function model page)
