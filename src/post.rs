use std::{collections::HashMap, error::Error, fmt::{self, Display, Formatter}, ops::Deref, path::{Path, PathBuf}};
use tokio::{fs::File, io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader, Lines}};
use ramhorns::Content;
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
	#[ramhorns(rename = "post_title")]
	pub title: String,
	pub description: Option<String>,
	pub created_date: String,
	pub modified_date: Option<String>
}

pub type PostMap = HashMap<String, Post>;

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
	
	pub async fn all_in_dir(path: impl AsRef<Path>) -> Result<PostMap, PostErr> {
		let mut map = HashMap::new();
		
		let mut reader = tokio::fs::read_dir(path).await?;
		while let Some(entry) = reader.next_entry().await? {
			if entry.file_type().await?.is_file() {
				let post = Post::from_file(entry.path()).await?;
				map.insert(post.meta.slug.clone(), post);
			}
		}
		
		Ok(map)
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
			let value = &line[eq + 1..];
			kv.insert(key.to_owned(), value.to_owned());
		}
		
		Ok(PostMetadata {
			slug: kv.remove("slug").ok_or(PostErr::NoSlug)?,
			title: kv.remove("title").ok_or(PostErr::NoTitle)?,
			description: kv.remove("description"),
			created_date: kv.remove("created_date").ok_or(PostErr::NoDate)?,
			modified_date: kv.remove("modified_date"),
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

#[derive(Debug)]
pub enum PostErr {
	Io(std::io::Error),
	NoSlug,
	NoTitle,
	NoDate,
	FrontMatterSyntax
}

impl From<std::io::Error> for PostErr {
    fn from(er: std::io::Error) -> Self {
        PostErr::Io(er)
    }
}

impl Display for PostErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
			PostErr::Io(e) => write!(f, "io error: {}", e),
			PostErr::NoSlug => write!(f, "no post slug specified"),
		    PostErr::NoTitle => write!(f, "no post title specified"),
            PostErr::NoDate => write!(f, "no creation date specified"),
            PostErr::FrontMatterSyntax => write!(f, "syntax error parsing the front-matter"),
        }
    }
}

impl Error for PostErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
			PostErr::Io(e) => Some(e),
			_ => None
		}
    }
}