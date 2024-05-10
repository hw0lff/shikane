mod field;
mod multi;
mod parser;
mod query;
mod single;

use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub use self::field::{FieldSet, FieldSetError, SearchField};
pub use self::multi::{MultiQuery, MultiSearch, MultiSearchResult};
pub use self::parser::ParseSingleSearchError;
pub use self::query::SingleQuery;
pub use self::single::{SingleSearch, SingleSearchResult};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SearchResult {
    Single(SingleSearchResult),
    Multi(MultiSearchResult),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Search {
    Single(SingleSearch),
    Multi(MultiSearch),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Query<'a> {
    Single(SingleQuery<'a>),
    Multi(MultiQuery<'a>),
}

#[derive(Clone, Copy, Debug, Default, PartialOrd, Ord, PartialEq, Eq)]
pub enum SearchKind {
    Regex,
    Substring,
    #[default]
    Fulltext,
}

#[derive(Clone, Debug)]
pub enum SearchPattern {
    Regex(regex::Regex),
    Substring(String),
    Fulltext(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompareMethod {
    // Try to match against at least one field
    #[default]
    AtleastOne,
    // The search has to match all given fields
    Exact,
}

impl Search {
    pub fn query<'a>(self) -> Query<'a> {
        match self {
            Search::Single(s) => s.query().into(),
            Search::Multi(s) => s.query().into(),
        }
    }
}

impl SearchResult {
    pub fn is_ok(&self) -> bool {
        match self {
            SearchResult::Single(sr) => sr.is_ok(),
            SearchResult::Multi(sr) => sr.is_ok(),
        }
    }
    pub fn specificity(&self) -> u64 {
        match self {
            SearchResult::Single(sr) => sr.specificity(),
            SearchResult::Multi(sr) => sr.specificity(),
        }
    }
}

impl<'a> Query<'a> {
    pub fn new(search: Search) -> Self {
        match search {
            Search::Single(search) => search.query().into(),
            Search::Multi(search) => search.query().into(),
        }
    }
    pub fn description(self, description: &'a str) -> Self {
        match self {
            Query::Single(q) => q.description(description).into(),
            Query::Multi(q) => q.description(description).into(),
        }
    }
    pub fn model(self, model: &'a str) -> Self {
        match self {
            Query::Single(q) => q.model(model).into(),
            Query::Multi(q) => q.model(model).into(),
        }
    }
    pub fn name(self, name: &'a str) -> Self {
        match self {
            Query::Single(q) => q.name(name).into(),
            Query::Multi(q) => q.name(name).into(),
        }
    }
    pub fn serial(self, serial: &'a str) -> Self {
        match self {
            Query::Single(q) => q.serial(serial).into(),
            Query::Multi(q) => q.serial(serial).into(),
        }
    }
    pub fn vendor(self, vendor: &'a str) -> Self {
        match self {
            Query::Single(q) => q.vendor(vendor).into(),
            Query::Multi(q) => q.vendor(vendor).into(),
        }
    }
    pub fn run(self) -> SearchResult {
        match self {
            Query::Single(q) => q.run().into(),
            Query::Multi(q) => q.run().into(),
        }
    }
}

impl SearchPattern {
    /// Compares the provided string with its contained pattern.
    ///
    /// Returns true if they match and how good they match.
    /// Higher is better.
    pub fn matches(&self, text: &str) -> (bool, u64) {
        let (is_matched, calculated_weight) = match self {
            SearchPattern::Regex(re) => {
                let b = re.is_match(text);
                (b, 1.0)
            }
            SearchPattern::Substring(s) => {
                let b = text.contains(s);
                (b, 1024.0 * (s.len() as f64 / text.len() as f64))
            }
            SearchPattern::Fulltext(s) => {
                let b = text == s;
                (b, 1024.0)
            }
        };
        if !is_matched {
            return (false, 0);
        }
        // scale the weight up by 1000 to remove the need for a float.
        // most relevant for substring matching.
        let weight = (calculated_weight * 1000.0).trunc() as u64;
        (is_matched, weight)
    }
}

impl SearchKind {
    pub fn as_char(&self) -> char {
        match self {
            SearchKind::Regex => '/',
            SearchKind::Substring => '%',
            SearchKind::Fulltext => '=',
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchKind::Regex => "Regex",
            SearchKind::Substring => "Substring",
            SearchKind::Fulltext => "Fulltext",
        }
    }
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '/' => Some(SearchKind::Regex),
            '%' => Some(SearchKind::Substring),
            '=' => Some(SearchKind::Fulltext),
            _ => None,
        }
    }
}

impl SearchPattern {
    fn as_str(&self) -> &str {
        match self {
            SearchPattern::Regex(re) => re.as_str(),
            SearchPattern::Substring(s) => s,
            SearchPattern::Fulltext(s) => s,
        }
    }
}

impl PartialEq for SearchPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SearchPattern::Regex(s), SearchPattern::Regex(o)) => s.as_str() == o.as_str(),
            (SearchPattern::Substring(s), SearchPattern::Substring(o)) => s == o,
            (SearchPattern::Fulltext(s), SearchPattern::Fulltext(o)) => s == o,
            (_, _) => false,
        }
    }
}

impl From<regex::Regex> for SearchPattern {
    fn from(r: regex::Regex) -> Self {
        Self::Regex(r)
    }
}

impl From<SingleSearchResult> for SearchResult {
    fn from(q: SingleSearchResult) -> Self {
        Self::Single(q)
    }
}

impl From<MultiSearchResult> for SearchResult {
    fn from(q: MultiSearchResult) -> Self {
        Self::Multi(q)
    }
}

impl From<SingleSearch> for Search {
    fn from(q: SingleSearch) -> Self {
        Self::Single(q)
    }
}

impl From<MultiSearch> for Search {
    fn from(q: MultiSearch) -> Self {
        Self::Multi(q)
    }
}

impl<'a> From<SingleQuery<'a>> for Query<'a> {
    fn from(q: SingleQuery<'a>) -> Self {
        Self::Single(q)
    }
}

impl<'a> From<MultiQuery<'a>> for Query<'a> {
    fn from(q: MultiQuery<'a>) -> Self {
        Self::Multi(q)
    }
}

impl Display for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Search::Single(s) => s.fmt(f),
            Search::Multi(s) => s.fmt(f),
        }
    }
}
