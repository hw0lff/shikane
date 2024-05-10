use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::{SingleSearch, SingleSearchResult};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MultiSearchResult {
    searches: Vec<SingleSearchResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MultiSearch {
    searches: Vec<SingleSearch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MultiQuery<'a> {
    search: MultiSearch,
    description: &'a str,
    model: &'a str,
    name: &'a str,
    serial: &'a str,
    vendor: &'a str,
}

impl MultiSearchResult {
    /// Returns `true` if all inner [`SingleSearchResult::is_ok()`]s return `true` too.
    pub fn is_ok(&self) -> bool {
        self.searches.iter().all(|ssr| ssr.is_ok())
    }

    /// Returns the sum of all inner [`SingleSearchResult::specificity()`].
    pub fn specificity(&self) -> u64 {
        self.searches.iter().map(|ssr| ssr.specificity()).sum()
    }
}

impl MultiSearch {
    pub fn new(searches: Vec<SingleSearch>) -> Self {
        Self { searches }
    }
    pub fn iter(&self) -> impl Iterator<Item = &SingleSearch> {
        self.searches.iter()
    }
    pub fn query<'a>(self) -> MultiQuery<'a> {
        MultiQuery::new(self)
    }
}

impl<'a> MultiQuery<'a> {
    pub fn new(search: MultiSearch) -> Self {
        Self {
            search,
            description: Default::default(),
            model: Default::default(),
            name: Default::default(),
            serial: Default::default(),
            vendor: Default::default(),
        }
    }
    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }
    pub fn model(mut self, model: &'a str) -> Self {
        self.model = model;
        self
    }
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }
    pub fn serial(mut self, serial: &'a str) -> Self {
        self.serial = serial;
        self
    }
    pub fn vendor(mut self, vendor: &'a str) -> Self {
        self.vendor = vendor;
        self
    }
    pub fn run(self) -> MultiSearchResult {
        let mut ssrs: Vec<SingleSearchResult> = vec![];
        for ss in self.search.searches {
            let ssr = ss
                .query()
                .description(self.description)
                .model(self.model)
                .name(self.name)
                .serial(self.serial)
                .vendor(self.vendor)
                .run();
            ssrs.push(ssr);
        }
        MultiSearchResult { searches: ssrs }
    }
}

impl Display for MultiSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, search) in self.searches.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            search.fmt(f)?;
        }
        write!(f, "]")
    }
}
