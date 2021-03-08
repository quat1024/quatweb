#[allow(unused_imports)]

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, sync::{Arc, Mutex}};
use ramhorns::{Ramhorns, Content};
use tokio::{runtime::Runtime};
use warp::{Filter, http::StatusCode};

type Horns = Arc<Mutex<Ramhorns>>;

fn main() {
	//server setup
	let rt = Runtime::new().unwrap();
	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80);
	
	//template engine
	let horns = Arc::new(Mutex::new(init_ramhorns()));
	
	//routes
	let static_pages = warp::fs::dir("www/static");
	let post = post_route(horns.clone());
	let reload = reload_ramhorns_route(horns);
	
	let routes = warp::get().and(
		post.or(static_pages).or(reload)
	);
	
	//let's go!
	rt.block_on(async {
		warp::serve(routes).bind(addr).await;
	});
}

fn init_ramhorns() -> Ramhorns {
	Ramhorns::from_folder("www/template").expect("failed to init ramhorns")
}

fn with_horns(horns: Horns) -> impl Filter<Extract = (Horns,), Error = Infallible> + Clone {
	warp::any().map(move || horns.clone())
}

fn reload_ramhorns_route(horns: Horns) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
	warp::path("reload_ramhorns").and(with_horns(horns)).map(|horns: Horns| {
		let mut asdf = horns.lock().unwrap();
		*asdf = init_ramhorns();
		
		"reloaded".to_string()
	})
}

fn post_route(horns: Horns) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
	warp::path!("post" / String).and(with_horns(horns)).and_then(render_post)
}

async fn render_post(post_name: String, horns: Horns) -> Result<impl warp::Reply, Infallible> {
	let mut path: PathBuf = "www/post".into();
	path.push(post_name);
	path.set_extension("md");
	
	//todo parse some front matter as well
	let post_markdown = tokio::fs::read_to_string(&path).await;
	if post_markdown.is_err() {
		return Ok(warp::reply::with_status("not found".into(), StatusCode::NOT_FOUND));
	}
	let post_markdown = post_markdown.unwrap();
	
	//let date: SystemTime = tokio::fs::metadata(&path).await.expect("could not read metadata").created().expect("could not read created time");
	
	#[derive(Content)]
	struct Post {
		title: String,
		#[md]
		post_contents: String
	}
	
	let post = Post {
		title: "post title lmao".into(),
		post_contents: post_markdown
	};
	
	let horns = horns.lock().unwrap();
	let post_template = horns.get("post.html").expect("failed to get post template");
	
	let rendered_post = post_template.render(&post);
	
	return Ok(warp::reply::with_status(rendered_post, StatusCode::OK));
}