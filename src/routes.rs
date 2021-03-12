use warp::filters::BoxedFilter;

use crate::*;

pub fn create_routes<'a>(app: Arc<App>) -> BoxedFilter<(impl Reply + 'a,)> {
	//routes
	let static_pages = warp::fs::dir("www/static");
	
	fn with_app(app: Arc<App>) -> impl Filter<Extract = (Arc<App>,), Error = Infallible> + Clone {
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

async fn handle_landing(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
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
	let template = content.ramhorns.get("discord.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
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
	let template = content.ramhorns.get("post.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		post: &'a Post,
		settings: &'a Settings
	}
	
	let templating_context = TemplatingContext {
		post: content.posts.get_by_slug(&post_name).ok_or(PostRouteErr::NoPost(post_name))?,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_post_index(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("post_index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
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
	let template = content.ramhorns.get("tag.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
	#[derive(Content)]
	struct TemplatingContext<'a> {
		posts: &'a Vec<&'a Post>,
		count: usize,
		many: bool,
		tag: &'a String,
		settings: &'a Settings
	}
	
	let tagged_posts = content.posts.get_by_tag(&tag);
	
	let templating_context = TemplatingContext {
		posts: &tagged_posts,
		count: tagged_posts.len(),
		many: tagged_posts.len() > 1,
		tag: &tag,
		settings: &app.settings
	};
	
	let rendered = template.render(&templating_context);
	Ok(warp::reply::html(rendered))
}

async fn handle_tag_index(app: Arc<App>) -> Result<impl Reply, Rejection> {
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("tag_index.template.html").ok_or(PostRouteErr::NoTemplate)?;
	
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
enum PostRouteErr {
	NoPost(String),
	NoTemplate
}

impl warp::reject::Reject for PostRouteErr {}