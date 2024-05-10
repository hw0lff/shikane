use std::str::FromStr;

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::prelude::*;

use super::{
    CompareMethod, FieldSet, FieldSetError, SearchField, SearchKind, SearchPattern, SingleSearch,
};

#[derive(Debug, PartialEq, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum ParseSingleSearchError {
    FieldSet { source: FieldSetError },
    Regex { source: regex::Error },
    MissingSearchKind,
}

impl FromStr for SingleSearch {
    type Err = ParseSingleSearchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars().peekable();
        let mut fieldset = FieldSet::default();
        let kind = loop {
            if let Some((c, sf)) = chars
                .peeking_take_while(|c| {
                    SearchField::from_char(*c).is_some() || SearchKind::from_char(*c).is_some()
                })
                .next()
                .map(|c| (c, SearchField::from_char(c)))
            {
                if let Some(sf) = sf {
                    fieldset.try_insert(sf).context(FieldSetCtx)?;
                } else if let Some(kind) = SearchKind::from_char(c) {
                    break Some(kind);
                } else {
                    break None;
                };
            } else {
                break None;
            }
        };

        let mut method = CompareMethod::Exact;
        if fieldset.is_empty() {
            fieldset.fill_default();
            method = CompareMethod::AtleastOne;
        }
        let kind = kind.ok_or(ParseSingleSearchError::MissingSearchKind)?;
        let s: String = chars.collect();
        let sp = match kind {
            SearchKind::Regex => SearchPattern::Regex(regex::Regex::new(&s).context(RegexCtx)?),
            SearchKind::Substring => SearchPattern::Substring(s),
            SearchKind::Fulltext => SearchPattern::Fulltext(s),
        };
        Ok(SingleSearch::new(fieldset, kind, sp, method))
    }
}

impl Serialize for SingleSearch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", self);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SingleSearch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ssearch = SingleSearch::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(ssearch)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;

    use crate::search::SearchField as SF;
    use crate::search::SearchKind as SK;
    use crate::search::SingleSearch;
    use SF::Description as D;
    use SF::Model as M;
    use SF::Name as N;
    use SF::Serial as S;
    use SF::Vendor as V;
    use SK::Fulltext as Ft;
    use SK::Regex as Rx;
    use SK::Substring as Sstr;

    #[rstest]
    #[case("=DP-1", [D,N,V,M,S], Ft, 4)] // Ft
    #[case("=ab%cdef", [D,N,V,M,S], Ft, 7)]
    #[case("mvsnd=DP-1", [M,V,S,N,D], Ft, 4)]
    #[case("ms=DP-1", [M,S], Ft, 4)]
    #[case("s=%/DP-1", [S], Ft, 6)]
    #[case("s=/%DP-1", [S], Ft, 6)]
    #[case("%display", [D,N,V,M,S], Sstr, 7)] // Sstr
    #[case("d%display", [D], Sstr, 7)]
    #[case("m%=display", [M], Sstr, 8)]
    #[case("m%/display", [M], Sstr, 8)]
    #[case("m%=/display", [M], Sstr, 9)]
    #[case("/DP", [D,N,V,M,S], Rx, 2)] // Rx
    #[case("mv/company", [M,V], Rx, 7)]
    #[case("m/%=model", [M], Rx, 7)]
    #[case("m/=%model", [M], Rx, 7)]
    fn parse_single_search_from_str_ok(
        #[case] s: &str,
        #[case] fields: impl AsRef<[SF]>,
        #[case] kind: SK,
        #[case] pattern_len: usize,
    ) {
        let ssearch = SingleSearch::from_str(s);
        assert!(ssearch.is_ok());
        let ssearch = ssearch.unwrap();
        assert_eq!(ssearch.kind, kind);
        let sfields: Vec<_> = ssearch.fields.iter().collect();
        assert_eq!(sfields, fields.as_ref());
        assert_eq!(ssearch.pattern.as_str().len(), pattern_len);
    }
}
