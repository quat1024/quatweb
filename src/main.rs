mod post;
mod time;
mod routes;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}, thread};
use post::{Post, PostCollection, PostErr, Tag};
use ramhorns::{Content, Ramhorns};
use tokio::{runtime::Runtime, sync::{mpsc::{self, UnboundedReceiver}, oneshot}};
use warp::{Filter, Rejection, Reply};

pub struct App {
	pub settings: Settings,
	pub content: RwLock<DynamicContent>
}

#[derive(Content)]
pub struct Settings {
	pub hostname: String,
	pub tls: bool,
	#[ramhorns(skip)]
	pub addr: SocketAddr,
	pub title: String,
}

pub struct DynamicContent {
	pub ramhorns: Ramhorns,
	pub posts: PostCollection
}

fn main() {
	let settings = Settings {
		hostname: "jiji.srv.highlysuspect.agency:12345".into(),
		addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80),
		tls: false,
		title: "Highly Suspect Agency".into()
	};
	
	//server setup
	let rt = Runtime::new().unwrap();
	
	rt.block_on(async {
		//first time app startup
		let app = Arc::new(App {
			settings,
			content: RwLock::new(init_content().await.expect("failed to initialize app"))
		});
		
		//shutdown trigger
		let (tx, rx) = oneshot::channel::<()>();
		
		//setup control console
		rt.spawn(control(app.clone(), tx, stdin_thread()));
		
		//setup server
		if app.settings.tls {
			let(_, server) = warp::serve(routes::create_routes(app.clone()))
				.tls()
				.cert_path("www/keys/cert.pem")
				.key_path("www/keys/key.rsa")
				.bind_with_graceful_shutdown(app.settings.addr, async { rx.await.ok().unwrap() });
			
			server.await;
		} else {
			let(_, server) = warp::serve(routes::create_routes(app.clone())).bind_with_graceful_shutdown(app.settings.addr, async { rx.await.ok().unwrap() });
			
			server.await;
		}
	});
}

/// Spawns a thread (an OS thread, not a future) that reads lines from stdin and passes them to the returned Receiver.
fn stdin_thread() -> UnboundedReceiver<String> {
	let (tx, rx) = mpsc::unbounded_channel();
	thread::Builder::new().name("Standard input reading thread".into()).spawn(move || loop {
		let mut buf = String::new();
		std::io::stdin().read_line(&mut buf).unwrap();
		tx.send(buf).expect("couldn't send stdin line");
	}).expect("Failed to spawn standard input thread");
	rx
}

/// Parses control commands from the stdin thread.
async fn control(app: Arc<App>, shutdown_tx: oneshot::Sender<()>, mut stdin: UnboundedReceiver<String>) {
	while let Some(line) = stdin.recv().await {
		match line.trim().as_ref() {
			"reload" => {
				match init_content().await {
					Ok(new_content) => {
						*app.content.write().unwrap() = new_content;
						println!("Reloaded contents");
					},
					Err(e) => {
						eprintln!("error reloading contents: {}", e);
					}
				}
			},
			"quit" => {
				eprintln!("Manually triggered shutdown");
				shutdown_tx.send(()).unwrap();
				return;
			}
			other => eprintln!("unknown command: {}", other)
		}
	}
}

async fn init_content() -> Result<DynamicContent, InitContentErr> {
	Ok(DynamicContent {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: PostCollection::from_folder("www/post").await?
	})
}

#[derive(Debug)]
enum InitContentErr {
	Ramhorns(ramhorns::Error),
	Post(PostErr)
}

impl From<ramhorns::Error> for InitContentErr {
    fn from(er: ramhorns::Error) -> Self {
        InitContentErr::Ramhorns(er)
    }
}

impl From<PostErr> for InitContentErr {
    fn from(er: PostErr) -> Self {
        InitContentErr::Post(er)
    }
}

impl std::fmt::Display for InitContentErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			InitContentErr::Ramhorns(e) => write!(f, "ramhorns error: {}", e),
			InitContentErr::Post(e) => write!(f, "post error: {}", e)
		}
    }
}

impl warp::reject::Reject for InitContentErr {}