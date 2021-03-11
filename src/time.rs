use std::{fmt::{Display, Formatter}, num::ParseIntError, str::FromStr};

use ramhorns::{Content, Template};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)] //field order matters for sorting
pub struct Date {
	pub year: u16, //is this future proof enough???1/?!?1/!?1
	pub month: Month,
	pub day: u8
}

impl Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:02}, {}", self.month, self.day, self.year)
    }
}

//Parses dates of the format "Jan 3, 2120" which is the format i prefer to write dates in lol
impl FromStr for Date {
    type Err = DateErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //Filter non a-zA-Z0-9 and space characters real quick
		let clean: String = s.chars().filter(|&c|
			('a'..='z').contains(&c) ||
			('A'..='Z').contains(&c) ||
			('0'..='9').contains(&c) ||
			c.is_ascii_whitespace()
		).collect();
		
		//Break it into three parts
		let split = clean.split_ascii_whitespace().collect::<Vec<_>>();
		if split.len() != 3 {
			return Err(DateErr::Not3Parts);
		}
		
		Ok(Date{
			year: split[2].parse::<u16>().map_err(DateErr::YearParse)?,
			month: split[0].parse::<Month>()?,
			day: split[1].parse::<u8>().map_err(DateErr::DayParse)?
		})
    }
}
#[derive(Debug)]
pub enum DateErr {
	Month(MonthErr),
	Not3Parts,
	DayParse(ParseIntError),
	YearParse(ParseIntError)
}

impl From<MonthErr> for DateErr {
    fn from(er: MonthErr) -> Self {
        DateErr::Month(er)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] //field order matters
pub enum Month {
	January,
	February,
	March,
	April,
	May,
	June,
	July,
	August,
	September,
	October,
	November,
	December,
}

impl Display for Month {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Month::January => "Jan",
            Month::February => "Feb",
            Month::March => "Mar",
            Month::April => "Apr",
            Month::May => "May",
            Month::June => "Jun",
            Month::July => "Jul",
            Month::August => "Aug",
            Month::September => "Sep",
            Month::October => "Oct",
            Month::November => "Nov",
            Month::December => "Dec"
        })
    }
}

impl FromStr for Month {
    type Err = MonthErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
		//this is lazy
		let pog = s.chars().map(|c| c.to_ascii_lowercase()).take(3).collect::<String>();
        match pog.as_ref() {
			"jan" => Ok(Month::January),
			"feb" => Ok(Month::February),
			"mar" => Ok(Month::March),
			"apr" => Ok(Month::April),
			"may" => Ok(Month::May),
			"jun" => Ok(Month::June),
			"jul" => Ok(Month::July),
			"aug" => Ok(Month::August),
			"sep" => Ok(Month::September),
			"oct" => Ok(Month::October),
			"nov" => Ok(Month::November),
			"dec" => Ok(Month::December),
			_ => Err(MonthErr::NotMonth(pog))
		}
    }
}

#[derive(Debug)]
pub enum MonthErr {
	NotMonth(String)
}

impl Content for Date {
	fn capacity_hint(&self, _tpl: &Template) -> usize {
		12 //3 letter month, space, worst-case 2 character day, comma, space, 4 character year
	}
	
    fn render_escaped<E: ramhorns::encoding::Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
		//Escaping is never needed so just call write_unescaped, it's faster, heyo
		encoder.write_unescaped(&self.to_string())
    }
}