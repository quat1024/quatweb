use std::ops::Deref;
use std::str::FromStr;

use chrono::NaiveDate;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use crate::*;

//TODO: I can't figure out how to get Ramhorns to render the *item* inside a vec
//For example if "tags" is a Vec<String> with "a", "b", "c", I can wrap something in {{#tags}}M{{/tags}} to get MMM.
//But I don't know how to actually render "abc".
//At least with this funny tuple struct I can use {{#tags}}{{0}}{{/tags}}.
#[derive(Content, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct Tag(pub String);

impl From<&String> for Tag {
	fn from(s: &String) -> Self {
		Tag(s.clone())
	}
}

///A wrapper around [chrono::NaiveDate] that serializes and deserializes to Ramhorns and Serde using my favorite date format.
#[serde_as]
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub struct MyNaiveDate {
	#[serde_as(as = "DisplayFromStr")]
	inner: NaiveDate,
}

static EPIC_DATE_FORMAT: &str = "%b %d, %Y";

impl FromStr for MyNaiveDate {
	type Err = chrono::ParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(NaiveDate::parse_from_str(s, EPIC_DATE_FORMAT)?.into())
	}
}

impl From<NaiveDate> for MyNaiveDate {
	fn from(n: NaiveDate) -> Self {
		MyNaiveDate { inner: n }
	}
}

impl Deref for MyNaiveDate {
	type Target = NaiveDate;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl ToString for MyNaiveDate {
	fn to_string(&self) -> String {
		self.format(EPIC_DATE_FORMAT).to_string()
	}
}

impl Content for MyNaiveDate {
	fn render_escaped<E: ramhorns::encoding::Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
		encoder.write_unescaped(&self.to_string())
	}
}
