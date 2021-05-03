use std::net::{IpAddr, Ipv4Addr};

use crate::*;

#[derive(Content, Deserialize, Debug)]
pub struct Settings {
	#[serde(default="default_hostname")]
	pub hostname: String,
	
	#[ramhorns(skip)]
	#[serde(default="default_addr")]
	pub addr: SocketAddr,
	
	#[serde(default="default_title")]
	pub title: String,
}

fn default_hostname() -> String {
	"http://highlysuspect.agency".into()
}

fn default_addr() -> SocketAddr {
	SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80)
}

fn default_title() -> String {
	"Highly Suspect Agency".into()
}