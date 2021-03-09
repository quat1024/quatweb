#[allow(unused_imports)]

mod post;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}};
use post::{Post, PostMap};
use ramhorns::{Ramhorns, Content};
use tokio::{runtime::Runtime};
use warp::{Filter, Rejection, Reply};

struct App {
	ramhorns: Ramhorns,
	posts: PostMap
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
		
		let post_base = || warp::path("post").and(with_rwlock_app(app.clone()));
		let post_index_route = post_base().and(warp::path::end()).and_then(handle_post_index);
		let post_route = post_base().and(warp::path::param()).and_then(handle_post);
		
		let reload_route = reload_route(app.clone());
		
		let routes = warp::get().and(static_pages
			.or(post_route)
			.or(post_index_route)
			.or(reload_route)
		);
	
		//letsa go!
		warp::serve(routes).bind(addr).await;
	});
}

async fn init_app() -> Result<App, Box<dyn std::error::Error>> {
	Ok(App {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: Post::all_in_dir("www/post").await?
	})
}

fn with_rwlock_app(app: Arc<RwLock<Arc<App>>>) -> impl Filter<Extract = (Arc<RwLock<Arc<App>>>,), Error = Infallible> + Clone {
	warp::any().map(move || app.clone())
}

async fn handle_post(app: Arc<RwLock<Arc<App>>>, post_name: String) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	let template = app.ramhorns.get("post.html").ok_or(PostRouteErr::RamhornsErr)?;
	
	let post = app.posts.get(&post_name).ok_or(PostRouteErr::NoPost(post_name))?;
	let contents = post.read_contents().await.map_err(PostRouteErr::ContentReadErr)?;
	
	#[derive(Content)]
	struct FormattedPost<'a> {
		slug: &'a str,
		title: &'a str,
		description: Option<&'a str>,
		created_date: &'a str,
		modified_date: Option<&'a str>,
		#[md]
		contents: String
	}
		
	let formatted_post = FormattedPost {
		slug: &post.slug,
		title: &post.title,
		description: post.description.as_ref().map(|s| s.as_ref()),
		created_date: &post.created_date,
		modified_date: post.modified_date.as_ref().map(|s| s.as_ref()),
		contents
	};
	
	let rendered = template.render(&formatted_post);
	
	Ok(warp::reply::html(rendered))
}

#[derive(Debug)]
enum PostRouteErr {
	NoPost(String),
	ContentReadErr(post::PostErr),
	RamhornsErr
}

impl warp::reject::Reject for PostRouteErr {}

async fn handle_post_index(app: Arc<RwLock<Arc<App>>>) -> Result<impl Reply, Rejection> {
	let app = app.read().unwrap().clone();
	
	//todo use a template for this
	let mut resp = String::new();
	app.posts.values().for_each(|post| {
		resp.push_str("post slug: ");
		resp.push_str(&post.slug);
		resp.push_str(" - title: ");
		resp.push_str(&post.title);
		resp.push_str("<br/>");
	});
	
	Ok(warp::reply::html(resp))
}

fn reload_route(app: Arc<RwLock<Arc<App>>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
	warp::path("refresh").and(with_rwlock_app(app)).and_then(|app: Arc<RwLock<Arc<App>>>| async move {
		let new_app = {
			let x = init_app().await;
			if x.is_err() {
				return Ok(format!("did not refresh app: {}", x.err().unwrap()));
			}
			x.unwrap()
		};
		
		let mut a = app.write().unwrap();
		*a = Arc::new(new_app);
		
		Ok("reloaded".into()) as Result<_, Infallible>
	})
}