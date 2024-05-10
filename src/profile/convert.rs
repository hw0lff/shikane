use std::collections::VecDeque;
use std::str::FromStr;

use itertools::Itertools;
use snafu::{prelude::*, Location};

use crate::profile::{Output, Profile};
use crate::search::{MultiSearch, ParseSingleSearchError, SearchField, SingleSearch};
use crate::settings::SettingsToml;
use crate::wl_backend::WlHead;

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum ConvertError {
    #[snafu(display("[{location}] Failed to create search from WlHead {head:?}"))]
    SingleSearch {
        source: ParseSingleSearchError,
        location: Location,
        head: Box<WlHead>,
    },
    #[snafu(display("[{location}] Cannot serialize Settings to TOML"))]
    TomlSerialize {
        source: toml::ser::Error,
        location: Location,
    },
}

#[derive(Clone, Debug, Default)]
pub struct ConverterSettings {
    profile_name: String,
    included_search_fields: Vec<SearchField>,
}

pub struct Converter {
    settings: ConverterSettings,
}

impl ConverterSettings {
    pub fn profile_name(mut self, name: String) -> Self {
        self.profile_name = name;
        self
    }
    pub fn include_search_fields(mut self, fields: Vec<SearchField>) -> Self {
        self.included_search_fields = fields;
        self
    }
    pub fn converter(self) -> Converter {
        Converter { settings: self }
    }
}
impl Converter {
    pub fn run(&self, heads: VecDeque<WlHead>) -> Result<String, ConvertError> {
        let mut outputs: Vec<Output> = vec![];
        for head in heads {
            outputs.push(self.convert_head_to_output(head)?);
        }

        let p = Profile::new(self.settings.profile_name.clone(), outputs);
        let sc = SettingsToml {
            timeout: None,
            profiles: vec![p].into(),
        };
        let settings_string = toml::to_string(&sc).context(TomlSerializeCtx)?;
        Ok(settings_string)
    }
    fn convert_head_to_output(&self, head: WlHead) -> Result<Output, ConvertError> {
        let ms = self
            .multi_search_from_head(&head)
            .context(SingleSearchCtx { head: head.clone() })?;
        let search_pattern = ms.into();

        if !head.enabled() {
            return Ok(Output::disabled(search_pattern));
        }

        let mut output = Output::enabled(search_pattern);
        if let Some(m) = head.current_mode().clone() {
            let mode = m.into();
            output.mode(mode);
        }
        output.position(head.position());
        output.transform(head.transform());
        if head.scale() != 0.0 {
            output.scale(head.scale());
        }
        if let Some(adaptive_sync) = head.adaptive_sync() {
            output.adaptive_sync(adaptive_sync);
        }
        Ok(output)
    }

    fn multi_search_from_head(&self, head: &WlHead) -> Result<MultiSearch, ParseSingleSearchError> {
        let searches: Result<Vec<SingleSearch>, ParseSingleSearchError> = self
            .settings
            .included_search_fields
            .iter()
            .unique()
            .map(|field| match field {
                SearchField::Description => {
                    SingleSearch::from_str(&format!("d={}", head.description()))
                }
                SearchField::Name => SingleSearch::from_str(&format!("n={}", head.name())),
                SearchField::Vendor => SingleSearch::from_str(&format!("v={}", head.make())),
                SearchField::Model => SingleSearch::from_str(&format!("m={}", head.model())),
                SearchField::Serial => {
                    SingleSearch::from_str(&format!("s={}", head.serial_number()))
                }
            })
            .collect();
        Ok(MultiSearch::new(searches?))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;

    use crate::profile::{Mode, Output, Profile};
    use crate::search::MultiSearch;
    use crate::search::SingleSearch;

    #[rstest]
    fn ser_multi_search() {
        let ss_serial = SingleSearch::from_str("=ab").unwrap();
        let ss_model = SingleSearch::from_str("=cd").unwrap();
        let ss_vendor = SingleSearch::from_str("=ef").unwrap();
        let ss_description = SingleSearch::from_str("=gh").unwrap();
        let ms = MultiSearch::new(vec![ss_serial, ss_model, ss_vendor, ss_description]);

        let output = Output {
            enable: true,
            search_pattern: ms.into(),
            commands: Default::default(),
            mode: Some(Mode::Best),
            position: None,
            scale: None,
            transform: None,
            adaptive_sync: None,
        };

        let profile = Profile::new("foo".into(), vec![output]);
        let s = toml::to_string(&profile).unwrap();
        let test = r#"name = "foo"

[[output]]
enable = true
search = ["=ab", "=cd", "=ef", "=gh"]
mode = "best"
"#;
        println!("{}", s);
        assert_eq!(test, s);
    }
}
