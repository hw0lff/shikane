use serde::Deserialize;

#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Output {
    pub enable: bool,
    pub r#match: String,
    pub mode: Mode,
    pub position: Position,
}
#[derive(Clone, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    #[serde(rename = "output")]
    pub outputs: Vec<Output>,
    pub exec: Option<Vec<String>>,
}
