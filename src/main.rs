#[allow(unused_imports)]

mod post;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}};
use post::{Post, PostErr, PostMap};
use ramhorns::{Ramhorns, Content};
use tokio::runtime::Runtime;
use warp::{Filter, Rejection, Reply};

struct App {
	ramhorns: Ramhorns,
	posts: PostMap,
	context: Context
}

#[derive(Content)]
struct Context {
	hostname: String,
	title: String
}

fn main() {
	//server setup
	let rt = Runtime::new().unwrap();
	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80);
	
	rt.block_on(async {
		//first time app startup
		let app = Arc::new(RwLock::new(Arc::new(init_app().await.expect("failed to initialize app"))));
		
		//routes
		let static_pages = warp::fs::dir("www/static");
		
		fn with_app(app: Arc<RwLock<Arc<App>>>) -> impl Filter<Extract = (Arc<RwLock<Arc<App>>>,), Error = Infallible> + Clone {
			warp::any().map(move || app.clone())
		}
		
		let post_index_route = warp::path("post")  .and(with_app(app.clone())).and(warp::path::end())  .and_then(handle_post_index);
		let post_route       = warp::path("post")  .and(with_app(app.clone())).and(warp::path::param()).and_then(handle_post);
		let reload_route     = warp::path("reload").and(with_app(app.clone()))                         .and_then(handle_reload);
		
		let routes = warp::get().and(static_pages
			.or(post_index_route)
			.or(post_route)
			.or(reload_route)
		);
	
		//letsa go!
		warp::serve(routes).bind(addr).await;
	});
}

async fn init_app() -> Result<App, InitError> {
	Ok(App {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: Post::all_in_dir("www/post").await?,
		context: Context { //todo find a better spot to put this.
			hostname: "localhost".into(),
			title: "Highly Suspect Agency".into()
		}
	})
}

#[derive(Debug)]
enum InitError {
	Ramhorns(ramhorns::Error),
	Post(PostErr)
}

impl From<ramhorns::Error> for InitError {
    fn from(er: ramhorns::Error) -> Self {
        InitError::Ramhorns(er)
    }
}

impl From<PostErr> for InitError {
    fn from(er: PostErr) -> Self {
        InitError::Post(er)
    }
}

impl warp::reject::Reject for InitError {}

async fn handle_post(app: Arc<RwLock<Arc<App>>>, post_name: String) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("post.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		post: &'a Post,
		context: &'a Context
	}
	
	let templating_context = TemplatingContext {
		post: app.posts.get(&post_name).ok_or(PostRouteErr::NoPost(post_name))?,
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post_index(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("post_index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	//Is this possible to do without copying?
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		context: &'a Context
	}
	
	//TODO: It's unsorted!
	let templating_context = TemplatingContext {
		posts: &app.posts.values().collect::<Vec<_>>(),
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

#[derive(Debug)]
enum PostRouteErr {
	NoPost(String),
	NoTemplate
}

impl warp::reject::Reject for PostRouteErr {}

async fn handle_reload(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let new_app = init_app().await?;
	
	let mut a = app.write().unwrap();
	*a = Arc::new(new_app);
	
	Ok(warp::reply::html("reloaded"))
}