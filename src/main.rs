mod post;
mod time;
mod routes;

use std::{convert::Infallible, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::{Arc, RwLock}, thread};
use post::{Post, PostCollection, PostErr, Tag};
use ramhorns::{Content, Ramhorns};
use tokio::{runtime::Runtime, sync::{mpsc::{self, UnboundedReceiver}, oneshot}};
use warp::{Filter, Rejection, Reply};

pub struct App {
	pub ramhorns: Ramhorns,
	pub posts: PostCollection,
	pub context: Context
}

#[derive(Content)]
pub struct Context {
	pub hostname: String,
	pub title: String
}

fn main() {
	//server setup
	let rt = Arc::new(Runtime::new().unwrap());
	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80);
	
	rt.block_on(async {
		//first time app startup
		let app = Arc::new(RwLock::new(Arc::new(init_app().await.expect("failed to initialize app"))));
		
		//shutdown trigger
		let (tx, rx) = oneshot::channel::<()>();
		
		//setup control console
		rt.spawn(control(app.clone(), tx, stdin_thread()));
		
		//setup server
		let (_, server) = warp::serve(routes::create_routes(app)).bind_with_graceful_shutdown(addr, async { rx.await.ok().unwrap() });
		
		//and let's go!
		server.await;
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
async fn control(app: Arc<RwLock<Arc<App>>>, shutdown_tx: oneshot::Sender<()>, mut stdin: UnboundedReceiver<String>) {
	while let Some(line) = stdin.recv().await {
		match line.trim().as_ref() {
			"reload" => {
				match init_app().await {
					Ok(new_app) => {
						*app.write().unwrap() = Arc::new(new_app);
						println!("Reloaded app");
					},
					Err(e) => {
						eprintln!("error reloading app: {}", e);
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

async fn init_app() -> Result<App, InitErr> {
	Ok(App {
		ramhorns: Ramhorns::from_folder("www/template")?,
		posts: PostCollection::from_folder("www/post").await?,
		context: Context { //todo find a better spot to put this.
			hostname: "localhost".into(),
			title: "Highly Suspect Agency".into()
		}
	})
}

#[derive(Debug)]
enum InitErr {
	Ramhorns(ramhorns::Error),
	Post(PostErr)
}

impl From<ramhorns::Error> for InitErr {
    fn from(er: ramhorns::Error) -> Self {
        InitErr::Ramhorns(er)
    }
}

impl From<PostErr> for InitErr {
    fn from(er: PostErr) -> Self {
        InitErr::Post(er)
    }
}

impl std::fmt::Display for InitErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			InitErr::Ramhorns(e) => write!(f, "ramhorns error: {}", e),
			InitErr::Post(e) => write!(f, "post error: {}", e)
		}
    }
}

impl warp::reject::Reject for InitErr {}