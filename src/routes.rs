use warp::{filters::BoxedFilter, hyper::StatusCode, reply};

use crate::*;

pub struct DynamicContent {
	pub ramhorns: Ramhorns,
	pub posts: PostCollection
}

impl DynamicContent {
	pub async fn init() -> Result<DynamicContent, InitContentErr> {
		Ok(DynamicContent {
			ramhorns: Ramhorns::from_folder("www/content/templates")?,
			posts: PostCollection::from_folder("www/content/posts").await?
		})
	}
	
	pub fn get_template(&self, tmpl: &'static str) -> Result<&Template, RouteErr> {
		self.ramhorns.get(tmpl).ok_or_else(|| RouteErr::NoTemplate(tmpl.to_string()))
	}
}

#[derive(Debug, Error)]
pub enum InitContentErr {
	#[error("Error loading templates")]
	Ramhorns(#[from] ramhorns::Error),
	#[error("Error parsing posts")]
	Post(#[from] PostErr)
}

impl warp::reject::Reject for InitContentErr {}

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
	).recover(move |rej| recover(rej, app.clone())).with(warp::log("quatweb::routes")).boxed()
}

async fn recover(rej: Rejection, app: Arc<App>) -> Result<impl Reply, Infallible> {
	error!("Attempting to recover from rejection: {:?}", rej);
	
	let content: &DynamicContent = &app.content.read().unwrap();
	let template = content.ramhorns.get("error.template.html");
	if template.is_none() {
		//Not much else to do
		let message = format!("Encountered an error, but couldn't load the fancy error page. {:?}", rej);
		error!("Could not load the error page!");
		let html = reply::html(message);
		return Ok(reply::with_status(html, StatusCode::INTERNAL_SERVER_ERROR));
	}
	let template = template.unwrap();
	
	let code: StatusCode;
	let message: String;
	
	//TODO there has to be a cuter way to do this, right
	
	if rej.is_not_found() {
		code = StatusCode::NOT_FOUND;
		message = "Not Found".into();
	} else if let Some(RouteErr::NoTemplate(tmpl)) = rej.find() {
		code = StatusCode::INTERNAL_SERVER_ERROR;
		message = format!("{}", RouteErr::NoTemplate(tmpl.clone()));
	} else if let Some(RouteErr::NoPost(post)) = rej.find() {
		code = StatusCode::NOT_FOUND;
		message = format!("{}", RouteErr::NoPost(post.clone()));
	} else if let Some(RouteErr::NoTag(tag)) = rej.find() {
		code = StatusCode::NOT_FOUND;
		message = format!("{}", RouteErr::NoTag(tag.clone()));
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
	let template = content.get_template("index.template.html")?;
	
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
	let template = content.get_template("discord.template.html")?;
	
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
	let template = content.get_template("post.template.html")?;
	
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
	let template = content.get_template("post_index.template.html")?;
	
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
	let template = content.get_template("tag.template.html")?;
	
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
	let template = content.get_template("tag_index.template.html")?;
	
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

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum RouteErr {
	#[error("Could not load template {0}")]
	NoTemplate(String),
	#[error("No post with slug {0}")]
	NoPost(String),
	#[error("No tag {0}")]
	NoTag(String)
}

impl warp::reject::Reject for RouteErr {}