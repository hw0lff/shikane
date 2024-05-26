use std::fmt::Display;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

use super::{CompareMethod, FieldSet, SearchKind, SearchPattern};

use super::{SearchField, SingleQuery};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SingleSearchResult {
    search: SingleSearch,
    satisfied_fields: Vec<(SearchField, u64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SingleSearch {
    pub(crate) fields: FieldSet,
    pub(crate) kind: SearchKind,
    pub(crate) pattern: SearchPattern,
    pub(crate) method: CompareMethod,
}

impl SingleSearchResult {
    pub fn new(search: SingleSearch, satisfied_fields: Vec<(SearchField, u64)>) -> Self {
        Self {
            search,
            satisfied_fields,
        }
    }

    pub fn is_ok(&self) -> bool {
        let ssfields: Vec<SearchField> = self.search.fields.iter().collect();
        let satisfied_fields: Vec<SearchField> =
            self.satisfied_fields.iter().map(|f| f.0).collect();
        trace!(
            "search_result.is_ok={}\nsearch_fields: {:?}\nsatisfied:     {:?}",
            ssfields == satisfied_fields,
            ssfields,
            satisfied_fields,
        );
        match self.search.method {
            CompareMethod::AtleastOne => !self.satisfied_fields.is_empty(),
            CompareMethod::Exact => ssfields == satisfied_fields,
        }
    }

    /// Return how specific the [`SingleSearch`] matched to its input from a [`SingleQuery`].
    ///
    /// Each [`SearchField`] rests at a certain index. `(`[`FieldSet::N`]` - 1 - index)` is taken
    /// to the power of 2 and then multiplied by the weight the [`SearchPattern::matches`] function
    /// returned.
    pub fn specificity(&self) -> u64 {
        self.satisfied_fields
            .iter()
            .enumerate()
            .map(|(idx, (_sf, weight))| weight * 2u64.pow((FieldSet::N - 1 - idx) as u32))
            .sum()
    }
}

impl SingleSearch {
    pub fn new(
        fields: FieldSet,
        kind: SearchKind,
        pattern: SearchPattern,
        method: CompareMethod,
    ) -> Self {
        Self {
            fields,
            kind,
            pattern,
            method,
        }
    }

    fn matches_field(&self, text: &str, field: SearchField) -> (bool, u64) {
        if !self.fields.contains(field) {
            return (false, 0);
        }
        self.pattern.matches(text)
    }
    pub fn matches_description(&self, text: &str) -> (bool, u64) {
        self.matches_field(text, SearchField::Description)
    }
    pub fn matches_model(&self, text: &str) -> (bool, u64) {
        self.matches_field(text, SearchField::Model)
    }
    pub fn matches_name(&self, text: &str) -> (bool, u64) {
        self.matches_field(text, SearchField::Name)
    }
    pub fn matches_serial(&self, text: &str) -> (bool, u64) {
        self.matches_field(text, SearchField::Serial)
    }
    pub fn matches_vendor(&self, text: &str) -> (bool, u64) {
        self.matches_field(text, SearchField::Vendor)
    }
    pub fn query<'a>(self) -> SingleQuery<'a> {
        SingleQuery::new(self)
    }
}

impl Display for SingleSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.method {
            CompareMethod::AtleastOne => {}
            CompareMethod::Exact => write!(f, "{}", self.fields)?,
        }
        write!(f, "{}", self.kind.as_char())?;
        write!(f, "{}", self.pattern.as_str())
    }
}
