quatweb
=======

Welcome to the quatweb project. This is the sources to what *will* be my new public-facing website. Not even close to finished, at the moment, so scram

(Imagine I put an under_construction.gif here)

It's kinda a static site, but also kinda not? It doesn't store the contents of posts in-memory, only some information about their frontmatter and their file paths, and that information is reloadable without restarting the server. Brought to you by: this nasty `Arc<RwLock<Arc<App>>>` type. I will probably change that and make it store the contents of posts in-memory, I don't think this is actually a very good idea.

Currently uses Ramhorns for HTML templating and (incindentally) markdown parsing but I'll probably switch to something else. If there's any markdown solutions out there that do syntax highlighting in code-blocks I'd be very interested to hear about them btw

Uses the [Warp](https://github.com/seanmonstar/warp) web server framework - rather poorly, unfortunately. Also I cribbed a bit from the sources of the world-famous [Christine Website](https://github.com/Xe/site).

### License

Oi mate, you got a loicense for that code?

I don't care, do what you want. No facists.