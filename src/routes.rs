use warp::filters::BoxedFilter;

use crate::*;

pub fn create_routes(app: Arc<RwLock<Arc<App>>>) -> BoxedFilter<(impl Reply,)> {
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
		.and(with_app(app))
		.and_then(handle_tag);
	
	warp::get().and(static_pages
		.or(landing_route)
		.or(discord_route)
		.or(post_index_route)
		.or(post_route)
		.or(tag_index_route)
		.or(tag_route)
	).boxed()
}

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