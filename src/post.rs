use std::{collections::HashMap, error::Error, fmt::{self, Display, Formatter}, path::{Path, PathBuf}};
use tokio::{fs::{File}, io::{AsyncBufReadExt, BufReader}};

pub struct Post {
	path: PathBuf,
	pub slug: String,
	pub title: String,
	pub description: Option<String>,
	pub created_date: String,
	pub modified_date: Option<String>
}

pub type PostMap = HashMap<String, Post>;

static FRONT_MATTER_DELIMITER: &str = "---\n";

impl Post {
	pub async fn from_file(path: impl AsRef<Path>) -> Result<Post, PostErr> {
		let path: &Path = path.as_ref();
		
		let reader = BufReader::new(File::open(path).await?);
		
		let mut slug: Option<String> = None;
		let mut title: Option<String> = None;
		let mut description: Option<String> = None;
		let mut created_date: Option<String> = None;
		let mut modified_date: Option<String> = None;
		
		let mut lines = reader.lines();
		while let Some(line) = lines.next_line().await? {
			if line == FRONT_MATTER_DELIMITER {
				break;
			}
			
			if line.trim().is_empty() || matches!(line.chars().next(), Some('#')) {
				continue;
			}
			
			let eq = line.find('=').ok_or(PostErr::FrontMatterSyntax)?;
			let key = &line[..eq];
			let value = &line[eq + 1..];
			
			match key {
				"slug" => slug = Some(value.to_owned()),
				"title" => title = Some(value.to_owned()),
				"description" => description = Some(value.to_owned()),
				"created_date" => created_date = Some(value.to_owned()),
				"modified_date" => modified_date = Some(value.to_owned()),
				_ => ()
			}
		}
		
		Ok(Post {
			path: path.to_owned(),
			slug: slug.ok_or(PostErr::NoSlug)?,
			title: title.ok_or(PostErr::NoTitle)?,
			description,
			created_date: created_date.ok_or(PostErr::NoDate)?,
			modified_date
		})
	}
	
	pub async fn all_in_dir(path: impl AsRef<Path>) -> Result<PostMap, PostErr> {
		let mut map = HashMap::new();
		
		let mut reader = tokio::fs::read_dir(path).await?;
		while let Some(entry) = reader.next_entry().await? {
			if entry.file_type().await?.is_file() {
				let post = Post::from_file(entry.path()).await?;
				map.insert(post.slug.clone(), post);
			}
		}
		
		Ok(map)
	}
	
	pub async fn read_contents(&self) -> Result<String, PostErr> {
		let reader = BufReader::new(File::open(&self.path).await?);
		let mut lines = reader.lines();
		
		//TODO this sucks lmao
		while let Some(line) = lines.next_line().await? {
			if line != FRONT_MATTER_DELIMITER {
				continue;
			}
		}
		
		let mut contents = Vec::new();
		
		while let Some(line) = lines.next_line().await? {
			contents.push(line);
		}
		
		return Ok(contents.join("\n"));
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