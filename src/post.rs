use std::{collections::HashMap, path::{Path, PathBuf}};
use tokio::{fs::File, io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader, Lines}};
use ramhorns::Content;
use crate::ext::*;
use crate::*;

#[derive(Content)]
pub struct Post {
	#[ramhorns(skip)]
	pub path: PathBuf,
	#[ramhorns(flatten)]
	pub meta: PostMetadata,
	pub content: String
}

#[derive(Content)]
pub struct PostMetadata {
	pub slug: String,
	pub author: String,
	#[ramhorns(rename = "post_title")]
	pub title: String,
	pub description: Option<String>,
	pub created_date: MyNaiveDate,
	pub modified_date: Option<MyNaiveDate>,
	pub tags: Vec<Tag>
}

static FRONT_MATTER_DELIMITER: &str = "---";

impl Post {
	pub async fn from_file(path: impl AsRef<Path>) -> Result<Post, PostErr> {
		let path: &Path = path.as_ref();
		
		let reader = BufReader::new(File::open(path).await?);
		let mut lines = reader.lines();
		
		let meta = Self::parse_metadata(&mut lines).await?;
		let content = Self::parse_content(lines).await?;
		
		Ok(Post{
			path: path.to_owned(),
			meta,
			content
		})
	}
	
	async fn parse_metadata<T: AsyncBufRead + Unpin>(line_reader: &mut Lines<T>) -> Result<PostMetadata, PostErr> {
		let mut kv: HashMap<String, String> = HashMap::new();
		
		while let Some(line) = line_reader.next_line().await? {
			if line.starts_with(FRONT_MATTER_DELIMITER) {
				//consume it
				break;
			}
			
			if line.trim().is_empty() || matches!(line.chars().next(), Some('#')) {
				continue;
			}
			
			let eq = line.find('=').ok_or(PostErr::FrontMatterSyntax)?;
			let key = &line[..eq];
			let value = &line[eq + 1..].trim();
			kv.insert(key.to_owned(), value.to_string());
		}
		
		//kinda a sticky situation that i couldn't find my way out of with the ? operator
		//basically None is fine and Some(Ok) is fine but Some(Err) is not fine
		let modified_date: Option<Result<MyNaiveDate, _>> = kv.remove("modified_date").map(|x| x.parse());
		if let Some(Err(e)) = modified_date {
			return Err(PostErr::DateParse(e));
		}
		let modified_date = modified_date.map(|x| x.unwrap());
		
		Ok(PostMetadata {
			slug: kv.remove("slug").ok_or(PostErr::NoSlug)?,
			author: kv.remove("author").ok_or(PostErr::NoAuthor)?,
			title: kv.remove("title").ok_or(PostErr::NoTitle)?,
			description: kv.remove("description"),
			created_date: kv.remove("created_date").ok_or(PostErr::NoDate)?.parse().map_err(PostErr::DateParse)?,
			modified_date,
			tags: kv.remove("tags").unwrap_or_else(|| "".into()).split(',').map(|x| Tag(x.trim().to_owned())).collect()
		})
	}
	
	async fn parse_content<T: AsyncBufRead + Unpin>(line_reader: Lines<T>) -> Result<String, PostErr> {	
		//collect the rest of the file
		let mut contents = String::new();
		line_reader.into_inner().read_to_string(&mut contents).await?;
		
		//pipe it through a markdown parser (nb: ramhorns does have its own md parser, but it's internally pulldown-cmark and is not configurable)
		use pulldown_cmark::Options as O;
		let md = pulldown_cmark::Parser::new_ext(&contents, O::ENABLE_FOOTNOTES | O::ENABLE_STRIKETHROUGH | O::ENABLE_TABLES | O::ENABLE_TASKLISTS);
		
		let mut html = String::new();
		pulldown_cmark::html::push_html(&mut html, md);
		
		Ok(html)
	}
}

//self-referential structs are Heck in rust atm
//here, the Vec owns all the posts, and other fields are indices into the Vec
#[derive(Content)]
pub struct PostCollection {
	#[ramhorns(rename = "post_collection")]
	pub all_posts: Vec<Post>,
	#[ramhorns(skip)]
	pub posts_by_slug: HashMap<String, usize>,
	#[ramhorns(skip)]
	pub posts_by_tag: HashMap<Tag, Vec<usize>>
}

impl PostCollection {
	pub async fn from_folder(path: impl AsRef<Path>) -> Result<PostCollection, PostErr> {
		let mut all_posts = Vec::new();
		
		let mut reader = tokio::fs::read_dir(path).await?;
		while let Some(entry) = reader.next_entry().await? {
			if entry.file_type().await?.is_file() {
				all_posts.push(Post::from_file(entry.path()).await?);
			}
		}
		
		all_posts.sort_by(|a, b| b.meta.created_date.cmp(&a.meta.created_date));
		
		let mut posts_by_slug = HashMap::new();
		let mut posts_by_tag: HashMap<_, Vec<_>> = HashMap::new();
		
		for (idx, post) in all_posts.iter().enumerate() {
			if posts_by_slug.insert(post.meta.slug.clone(), idx).is_some() {
				return Err(PostErr::DuplicateSlug(post.meta.slug.clone()));
			}
			
			for tag in post.meta.tags.iter() {
				(*posts_by_tag.entry(tag.clone()).or_default()).push(idx);
			}
		}
		
		Ok(PostCollection {
			all_posts,
			posts_by_slug,
			posts_by_tag
		})
	}
	
	pub fn get_by_slug(&self, slug: &str) -> Option<&Post> {
		self.posts_by_slug.get(slug).map(|&index| &self.all_posts[index])
	}
	
	pub fn get_by_tag(&self, tag: impl Into<Tag>) -> Vec<&Post> {
		match self.posts_by_tag.get(&tag.into()) {
			None => Vec::new(),
			Some(poasts) => {
				let mut result = Vec::new();
				for &i in poasts {
					result.push(&self.all_posts[i]);
				}
				result
			}
		}
	}
}

#[derive(Debug, Error)]
pub enum PostErr {
	#[error("IO error")]
	Io(#[from] std::io::Error),
	#[error("No post slug provided")]
	NoSlug,
	#[error("No post author provided")]
	NoAuthor,
	#[error("No title provided")]
	NoTitle,
	#[error("No creation date provided")]
	NoDate,
	#[error("Error parsing date")]
	DateParse(#[from] chrono::ParseError),
	#[error("Two posts with the same slug: {0}")]
	DuplicateSlug(String),
	#[error("Error parsing front-matter")]
	FrontMatterSyntax
}