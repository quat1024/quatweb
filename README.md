quatweb
=======

Welcome to the quatweb project. This is the sources to what *will* be my new public-facing website. Not even close to finished, at the moment, so scram

(Imagine I put an under_construction.gif here)

It's kinda a static site, but also kinda not? It puzzles the templates together on every request. There's also a reload endpoint to make it reparse templates from disk (the error handling on this is... not great, I'll look into eyre or something)

Uses Ramhorns for HTML templating and pulldown-cmark for markdown parsing (but i'll probably switch to comrak so i can add codeblock syntax highlighting)

Uses the [Warp](https://github.com/seanmonstar/warp) web server framework also I cribbed a bit from the sources of the world-famous [Christine Website](https://github.com/Xe/site).

### License

Oi mate, you got a loicense for that code?

(I don't care, do what you want. No fascists.)