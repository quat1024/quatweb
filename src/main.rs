#[allow(unused_imports)]

mod post;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, Mutex}};
use post::{Post, PostMap};
use ramhorns::{Ramhorns, Content};
use tokio::{runtime::Runtime};
use warp::{Filter, http::StatusCode};

struct App {
	ramhorns: Ramhorns,
	posts: PostMap
}

fn main() {
	//server setup
	let rt = Runtime::new().unwrap();
	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80);
	
	rt.block_on(async {
		//template engine
		let app = Arc::new(init_app().await.expect("failed to initialize app"));
		
		//routes
		let static_pages = warp::fs::dir("www/static");
		let post = post_route(app.clone());
		
		let routes = warp::get().and(
			post.or(static_pages)
		);
	
		//letsa go
		warp::serve(routes).bind(addr).await;
	});
}

async fn init_app() -> Result<App, Box<dyn std::error::Error>> {
	Ok(App {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: Post::all_in_dir("www/post").await?
	})
}

fn with_app(app: Arc<App>) -> impl Filter<Extract = (Arc<App>,), Error = Infallible> + Clone {
	warp::any().map(move || app.clone())
}

fn post_route(app: Arc<App>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
	warp::path!("post" / String).and(with_app(app)).and_then(render_post)
}

async fn render_post(post_name: String, app: Arc<App>) -> Result<impl warp::Reply, Infallible> {
	let post = app.posts.get(&post_name);
	if post.is_none() {
		return Ok(warp::reply::with_status(warp::reply::html("not found".into()), StatusCode::NOT_FOUND));
	}
	let post = post.unwrap();
	
	let post_contents = post.read_contents().await;
	if post_contents.is_err() {
		return Ok(warp::reply::with_status(warp::reply::html("problem reading post".into()), StatusCode::INTERNAL_SERVER_ERROR));
	}
	let post_contents = post_contents.unwrap();
	
	#[derive(Content)]
	struct FormattedPost<'a> {
		slug: &'a str,
		title: &'a str,
		description: Option<&'a str>,
		created_date: &'a str,
		modified_date: Option<&'a str>,
		#[md]
		post_contents: String
	}
	
	let post = FormattedPost {
		slug: &post.slug,
		title: &post.title,
		description: post.description.as_ref().map(|s| s.as_ref()),
		created_date: &post.created_date,
		modified_date: post.modified_date.as_ref().map(|s| s.as_ref()),
		post_contents
	};
	
	let post_template = app.ramhorns.get("post.html");
	if post_template.is_none() {
		return Ok(warp::reply::with_status(warp::reply::html("could not read post template".into()), StatusCode::INTERNAL_SERVER_ERROR));
	}
	let post_template = post_template.unwrap();
	
	let rendered_post = post_template.render(&post);
	return Ok(warp::reply::with_status(warp::reply::html(rendered_post), StatusCode::OK));
}