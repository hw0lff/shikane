use super::{CompareMethod, SearchField, SingleSearch, SingleSearchResult};

#[derive(Clone, Debug, PartialEq)]
pub struct SingleQuery<'a> {
    search: SingleSearch,
    description: &'a str,
    model: &'a str,
    name: &'a str,
    serial: &'a str,
    vendor: &'a str,
}

impl<'a> SingleQuery<'a> {
    pub fn new(search: SingleSearch) -> Self {
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
    pub fn run(self) -> SingleSearchResult {
        let mut matches = true;
        let mut satisfied_fields: Vec<(SearchField, u64)> = vec![];

        let operation: fn(bool, bool) -> bool = match self.search.method {
            // match against at least 1 field, field has to be the left parameter
            CompareMethod::AtleastOne => |_, b| b,
            // match against all given fields
            CompareMethod::Exact => |a, b| a && b,
        };
        for field in self.search.fields.iter() {
            let (b, weight) = match field {
                SearchField::Description => self.search.matches_description(self.description),
                SearchField::Model => self.search.matches_model(self.model),
                SearchField::Name => self.search.matches_name(self.name),
                SearchField::Serial => self.search.matches_serial(self.serial),
                SearchField::Vendor => self.search.matches_vendor(self.vendor),
            };
            matches = operation(matches, b);
            if matches {
                // We can unwrap here because our input comes from another FieldSet
                satisfied_fields.push((field, weight));
            }
        }

        SingleSearchResult::new(self.search, satisfied_fields)
    }
}

#[cfg(test)]
mod test {
    use crate::search::{
        CompareMethod, FieldSet, SearchField, SearchKind, SearchPattern, SingleSearch,
    };

    use super::SingleQuery;

    #[test]
    fn single_search() {
        let ssearch = SingleSearch {
            fields: FieldSet::new(SearchField::Name),
            kind: SearchKind::Fulltext,
            pattern: SearchPattern::Fulltext(String::from("DP-1")),
            method: CompareMethod::Exact,
        };
        let ssr = SingleQuery::new(ssearch)
            .name("DP-1")
            .model("generic model")
            .vendor("generic vendor")
            .run();

        assert!(ssr.is_ok());
    }
}
