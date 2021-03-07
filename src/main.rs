use warp::Filter;
use std::{convert::Infallible, time::Duration};
use tokio::{runtime::Runtime};

fn main() {
	let rt = Runtime::new().unwrap();
	
	let static_files = warp::fs::dir("www/static");
	
	let seepy_cosy = warp::path("seepy").and_then(|| async {
		tokio::time::sleep(Duration::from_secs(5)).await;
		Ok("all done") as Result<_, Infallible>
	});
	
	let memedown = warp::path("md").and(warp::fs::dir("www/test")).and_then(|file: warp::fs::File| async move {
		let file_contents = tokio::fs::read_to_string(file.path()).await;
		
		match file_contents {
			Ok(file_contents) => {
				let mark = pulldown_cmark::Parser::new(&file_contents);
				let mut out = String::new();
				pulldown_cmark::html::push_html(&mut out, mark);
		
				Ok(warp::reply::html(out))
			},
			Err(_) => {
				Err(warp::reject())
			}
		}
	});
	
	let routes = warp::get().and(static_files
		.or(seepy_cosy)
		.or(memedown)
	);
	
	rt.block_on(async {
		warp::serve(routes).bind(([127, 0, 0, 1], 80)).await
	});
}