mod post;
mod routes;
mod settings;
mod ext;

use std::{convert::Infallible, net::SocketAddr, sync::{Arc, RwLock}, thread};
use log::*;
use post::{Post, PostCollection, PostErr};
use routes::{DynamicContent, InitContentErr};
use ext::Tag;
use ramhorns::{Content, Ramhorns, Template};
use settings::Settings;
use tokio::{runtime::Runtime, sync::{mpsc::{self, UnboundedReceiver}, oneshot}};
use warp::{Filter, Rejection, Reply};
use serde::Deserialize;
use thiserror::Error;

pub struct App {
	pub settings: Settings,
	pub content: RwLock<DynamicContent>
}

fn main() {
	//Annoyingly pretty-env-logger doesn't use the `Env` system from env-logger. That's annoying.
	//So I actually have to set the environment variable myself. https://github.com/seanmonstar/pretty-env-logger/issues/41
	if std::env::var_os("RUST_LOG").is_none() {
		std::env::set_var("RUST_LOG", "info");
	}
	
	pretty_env_logger::init_timed();
	info!("🐉 dragn time");
	
	//parse settings from environment variables
	let settings = match envy::prefixed("QUAT_").from_env::<Settings>() {
		Ok(settings) => settings,
		Err(e) => panic!("error parsing environment variables: {}", e)
	};
	
	//server setup
	let rt = Arc::new(Runtime::new().unwrap());
	
	rt.block_on(async {
		//first time app startup
		let app = Arc::new(App {
			settings,
			content: RwLock::new(DynamicContent::init().await.expect("failed to initialize app"))
		});
		
		//shutdown trigger
		let (shut_tx, shut_rx) = oneshot::channel::<()>();
		
		//setup control console
		rt.spawn(control(app.clone(), shut_tx, stdin_thread()));
		
		//setup server
		if app.settings.tls {
			let(_, server) = warp::serve(routes::create_routes(app.clone()))
				.tls()
				.cert_path("www/keys/cert.pem")
				.key_path("www/keys/key.rsa")
				.bind_with_graceful_shutdown(app.settings.addr, async { shut_rx.await.ok().unwrap() });
				
			info!("Server started with TLS.");
			server.await;
		} else {
			let(_, server) = warp::serve(routes::create_routes(app.clone())).bind_with_graceful_shutdown(app.settings.addr, async { shut_rx.await.ok().unwrap() });
			
			warn!("Server started without any TLS!");
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
	}).expect("Failed to spawn standard input reading thread");
	rx
}

/// Parses control commands from the stdin thread.
async fn control(app: Arc<App>, shutdown_tx: oneshot::Sender<()>, mut stdin: UnboundedReceiver<String>) {
	while let Some(line) = stdin.recv().await {
		match line.trim().as_ref() {
			"reload" => {
				match rebuild_dynamic_content(&app).await {
					Ok(()) => {
						info!("Reloaded contents");
					},
					Err(e) => {
						error!("error reloading contents: {}", e);
					}
				}
			},
			"quit" => {
				error!("Manually triggered shutdown");
				shutdown_tx.send(()).unwrap();
				return;
			}
			other => error!("unknown command: {}", other)
		}
	}
}

async fn rebuild_dynamic_content(app: &Arc<App>) -> Result<(), InitContentErr> {
	DynamicContent::init().await.map(|new_content| {
		*app.content.write().unwrap() = new_content;
	})
}