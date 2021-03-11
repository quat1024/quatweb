mod post;
mod time;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}};
use post::{Post, PostCollection, PostErr, Tag};
use ramhorns::{Content, Ramhorns};
use tokio::runtime::Runtime;
use warp::{Filter, Rejection, Reply};

struct App {
	ramhorns: Ramhorns,
	posts: PostCollection,
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
		
		let landing_route = warp::path::end()
			.and(with_app(app.clone()))
			.and_then(handle_landing);
			
		let discord_route = warp::path!("discord")
			.and(with_app(app.clone()))
			.and_then(handle_discord);
			
		let post_index_route = warp::path!("posts")
			.and(with_app(app.clone()))
			.and_then(handle_post_index);
		
		let post_route = warp::path!("posts" / String)
			.and(with_app(app.clone()))
			.and_then(handle_post);
		
		let tag_index_route = warp::path!("tags")
			.and(with_app(app.clone()))
			.and_then(handle_tag_index);
		
		let tag_route = warp::path!("tags" / String)
			.and(with_app(app.clone()))
			.and_then(handle_tag);
		
		//todo guard this behind a cookie or something LOL
		let reload_route = warp::path!("reload")
			.and(with_app(app.clone()))
			.and_then(handle_reload);
		
		let routes = warp::get().and(static_pages
			.or(landing_route)
			.or(discord_route)
			.or(post_index_route)
			.or(post_route)
			.or(tag_index_route)
			.or(tag_route)
			.or(reload_route)
		);
	
		//letsa go!
		warp::serve(routes).bind(addr).await;
	});
}

async fn init_app() -> Result<App, InitError> {
	Ok(App {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: PostCollection::from_folder("www/post").await?,
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

async fn handle_landing(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		context: &'a Context
	}
	
	let templating_context = TemplatingContext {
		posts: &app.posts.all_posts.iter().take(5).collect::<Vec<_>>(),
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_discord(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("discord.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		context: &'a Context
	}
	
	let templating_context = TemplatingContext {
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post(post_name: String, app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("post.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		post: &'a Post,
		context: &'a Context
	}
	
	let templating_context = TemplatingContext {
		post: app.posts.get_by_slug(&post_name).ok_or(PostRouteErr::NoPost(post_name))?,
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post_index(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("post_index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<Post>,
		count: usize,
		many: bool,
		context: &'a Context
	}
	
	let count = app.posts.all_posts.len();
	
	let templating_context = TemplatingContext {
		posts: &app.posts.all_posts,
		count,
		many: count > 1,
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_tag(tag: String, app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("tag.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		count: usize,
		many: bool,
		tag: &'a String,
		context: &'a Context
	}
	
	let tagged_posts = app.posts.get_by_tag(&tag);
	
	let templating_context = TemplatingContext {
		posts: &tagged_posts,
		count: tagged_posts.len(),
		many: tagged_posts.len() > 1,
		tag: &tag,
		context: &app.context
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_tag_index(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("tag_index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		tags: Vec<&'a Tag>,
		count: usize,
		many: bool,
		context: &'a Context
	}
	
	let mut tags = app.posts.posts_by_tag.keys().collect::<Vec<_>>();
	tags.sort();
	let count = tags.len();
	
	let templating_context = TemplatingContext {
		tags,
		count,
		many: count > 1,
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