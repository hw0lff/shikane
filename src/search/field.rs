use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSet {
    index: usize,
    set: [Option<SearchField>; Self::N],
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchField {
    Description,
    Name,
    Vendor,
    Model,
    Serial,
}

impl FieldSet {
    pub const N: usize = 5;

    pub fn new(field: SearchField) -> Self {
        let mut set: [_; Self::N] = Default::default();
        set[0] = Some(field);
        Self { index: 1, set }
    }

    // fn acc(mut self, field: SearchField) -> Self {
    //     if self.index < Self::N {
    //         self.set[self.index] = Some(field);
    //         self.index += 1;
    //     }
    //     self
    // }
    fn try_acc(mut self, field: SearchField) -> Result<Self, FieldSetError> {
        self.try_insert(field)?;
        Ok(self)
    }

    fn check_insert(&self, field: SearchField) -> Result<(), FieldSetError> {
        if self.index >= Self::N {
            return Err(FieldSetError::Full);
        }
        let new = Some(field);
        if self.set.contains(&new) {
            return Err(FieldSetError::AlreadyInside(field));
        }
        Ok(())
    }

    pub fn try_insert(&mut self, field: SearchField) -> Result<(), FieldSetError> {
        self.check_insert(field)?;
        self.set[self.index] = Some(field);
        self.index += 1;
        Ok(())
    }

    pub fn weight(&self, field: SearchField) -> Option<u16> {
        if let Some(exp) = self.set.iter().position(|sf| *sf == Some(field)) {
            Some(2_u16.pow(exp as u32))
        } else if self.set.is_empty() {
            Some(2_u16.pow(field as u32))
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = SearchField> + '_ {
        self.set.iter().filter_map(|f| *f)
    }
    pub fn contains(&self, field: SearchField) -> bool {
        self.set.contains(&Some(field))
    }
    pub fn is_empty(&self) -> bool {
        self.set.iter().all(|f| f.is_none())
    }
    pub fn len(&self) -> usize {
        self.set.iter().filter(|f| f.is_some()).count()
    }
    pub fn fill_default(&mut self) {
        self.set[0] = Some(SearchField::Description);
        self.set[1] = Some(SearchField::Name);
        self.set[2] = Some(SearchField::Vendor);
        self.set[3] = Some(SearchField::Model);
        self.set[4] = Some(SearchField::Serial);
    }
}

impl SearchField {
    pub fn as_char(&self) -> char {
        match self {
            SearchField::Description => 'd',
            SearchField::Model => 'm',
            SearchField::Name => 'n',
            SearchField::Serial => 's',
            SearchField::Vendor => 'v',
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchField::Description => "Description",
            SearchField::Model => "Model",
            SearchField::Name => "Name",
            SearchField::Serial => "Serial",
            SearchField::Vendor => "Vendor",
        }
    }
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'd' => Some(SearchField::Description),
            'm' => Some(SearchField::Model),
            'n' => Some(SearchField::Name),
            's' => Some(SearchField::Serial),
            'v' => Some(SearchField::Vendor),
            _ => None,
        }
    }
}

impl TryFrom<[Option<SearchField>; Self::N]> for FieldSet {
    type Error = FieldSetError;

    fn try_from(value: [Option<SearchField>; Self::N]) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .flatten()
            .try_fold(Self::default(), |fs, field| fs.try_acc(field))
    }
}

impl TryFrom<Vec<SearchField>> for FieldSet {
    type Error = FieldSetError;

    fn try_from(value: Vec<SearchField>) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .try_fold(Self::default(), |fs, field| fs.try_acc(field))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldSetError {
    /// The given [SearchField] has already been inserted
    AlreadyInside(SearchField),
    /// There is no more space inside this [FieldSet] left
    Full,
}

impl std::error::Error for FieldSetError {}

impl Display for FieldSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldSetError::AlreadyInside(sf) => write!(f, "{sf} was already specified"),
            FieldSetError::Full => {
                write!(f, "Cannot specify more than {} search fields", FieldSet::N)
            }
        }
    }
}

impl Display for SearchField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for FieldSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for field in self.set.into_iter().flatten() {
            write!(f, "{}", field.as_char())?;
        }
        Ok(())
    }
}
