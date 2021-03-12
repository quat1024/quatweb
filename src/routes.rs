use warp::{filters::BoxedFilter, hyper::StatusCode, reply};

use crate::*;

pub fn create_routes<'a>(app: Arc<App>) -> BoxedFilter<(impl Reply + 'a,)> {
	//routes
	let static_pages = warp::fs::dir("www/static");
	
	fn with_app(app: Arc<App>) -> impl Filter<Extract = (Arc<App>,), Error = Infallible> + Clone {
		warp::any().map(move || app.clone())
	}
	
	//TODO: none of these routes actually *need* the async keyword on them, they never await
	
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
	
	warp::get().and(static_pages
		.or(landing_route)
		.or(discord_route)
		.or(post_index_route)
		.or(post_route)
		.or(tag_index_route)
		.or(tag_route)
	).recover(move |rej| recover(rej, app.clone())).boxed()
}

async fn recover(rej: Rejection, app: Arc<App>) -> Result<impl Reply, Infallible> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("error.template.html");
	if template.is_none() {
		//Not much else to do
		let message = format!("Encountered an error, but couldn't load the fancy error page. {:?}", rej);
		let html = reply::html(message);
		return Ok(reply::with_status(html, StatusCode::INTERNAL_SERVER_ERROR));
	}
	let template = template.unwrap();
	
	let code: StatusCode;
	let message: String;
	
	if rej.is_not_found() {
		code = StatusCode::NOT_FOUND;
		message = "Not Found.".into();
	} else if let Some(RouteErr::NoTemplate) = rej.find() {
		code = StatusCode::INTERNAL_SERVER_ERROR;
		message = "Missing template.".into();
	} else if let Some(RouteErr::NoPost(post)) = rej.find() {
		code = StatusCode::NOT_FOUND;
		message = format!("No post at {}.", post);
	} else if let Some(RouteErr::NoTag(tag)) = rej.find() {
		code = StatusCode::NOT_FOUND;
		message = format!("No tag named {}.", tag);
	} else {
		code = StatusCode::INTERNAL_SERVER_ERROR;
		message = format!("Unhandled: {:?}", rej);
	}
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		errno: u16,
		message: String,
		include_error_css: bool,
		settings: &'a Settings
	}
	
	let rendered = template.render(&TemplatingContext {
		errno: code.into(),
		message,
		include_error_css: true,
		settings: &app.settings
	});
	
	Ok(reply::with_status(reply::html(rendered), code))
}

async fn handle_landing(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("index.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		settings: &'a Settings
	}
	
	let templating_context = TemplatingContext {
		posts: &content.posts.all_posts.iter().take(5).collect::<Vec<_>>(),
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_discord(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("discord.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		settings: &'a Settings
	}
	
	let templating_context = TemplatingContext {
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post(post_name: String, app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("post.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		post: &'a Post,
		settings: &'a Settings
	}
	
	let templating_context = TemplatingContext {
		post: content.posts.get_by_slug(&post_name).ok_or(RouteErr::NoPost(post_name))?,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post_index(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("post_index.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<Post>,
		count: usize,
		many: bool,
		settings: &'a Settings
	}
	
	let posts = &content.posts.all_posts;
	let count = posts.len();
	
	let templating_context = TemplatingContext {
		posts,
		count,
		many: count > 1,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_tag(tag: String, app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("tag.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		count: usize,
		many: bool,
		tag: &'a String,
		settings: &'a Settings
	}
	
	let tagged_posts = content.posts.get_by_tag(&tag);
	let count = tagged_posts.len();
	
	if count == 0 {
		return Err(RouteErr::NoTag(tag).into());
	}
	
	let templating_context = TemplatingContext {
		posts: &tagged_posts,
		count,
		many: count > 1,
		tag: &tag,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_tag_index(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("tag_index.template.html").ok_or(RouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		tags: Vec<&'a Tag>,
		count: usize,
		many: bool,
		settings: &'a Settings
	}
	
	let mut tags = content.posts.posts_by_tag.keys().collect::<Vec<_>>();
	tags.sort();
	let count = tags.len();
	
	let templating_context = TemplatingContext {
		tags,
		count,
		many: count > 1,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum RouteErr {
	NoTemplate,
	NoPost(String),
	NoTag(String)
}

impl warp::reject::Reject for RouteErr {}