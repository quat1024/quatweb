quatweb
=======

Welcome to the quatweb project. This is the sources to what *will* be my new public-facing website. Not even close to finished, at the moment, so scram

(Imagine I put an under_construction.gif here)

It's kinda a static site, but also kinda not? It puzzles the templates together on every request. The server also reads from stdin. Command "reload" reparses markdown posts and the html templates, and command "quit" issues a graceful shutdown.

Uses Ramhorns for HTML templating and pulldown-cmark for markdown parsing, for now. I'll probably end up switching.

Uses the [Warp](https://github.com/seanmonstar/warp) web server framework also I cribbed a bit from the sources of the world-famous [Christine Website](https://github.com/Xe/site).

### License

Oi mate, you got a loicense for that code?

(I don't care, do what you want. No fascists.)

### Stuff

Recursively downloading a website: `wget --convert-links -r -p -E localhost:80`

* `--convert-links`: make all links relative
* `-r`: recursive
* `-p`: get all prerequisites like css and images
* `-E`: fix file extensions