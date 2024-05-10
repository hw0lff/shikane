mod convert;
mod mode;

use std::fmt::Display;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::prelude::*;

use crate::search::Search;

pub use self::convert::{ConvertError, Converter, ConverterSettings};
pub use self::mode::Mode;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,
    #[serde(skip)]
    pub index: usize,
    #[serde(rename = "exec")]
    pub commands: Option<Vec<String>>,
    // table must come last in toml
    #[serde(rename = "output")]
    pub outputs: Vec<Output>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Output {
    pub enable: bool,
    #[serde(rename = "search", alias = "match")]
    pub search_pattern: Search,
    #[serde(rename = "exec")]
    pub commands: Option<Vec<String>>,
    pub mode: Option<Mode>,
    pub position: Option<Position>,
    pub scale: Option<f64>,
    pub transform: Option<Transform>,
    pub adaptive_sync: Option<AdaptiveSyncState>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct PhysicalSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptiveSyncState {
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Transform {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "90")]
    _90,
    #[serde(rename = "180")]
    _180,
    #[serde(rename = "270")]
    _270,
    #[serde(rename = "flipped")]
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

impl Profile {
    pub fn new(name: String, outputs: Vec<Output>) -> Self {
        Self {
            name,
            outputs,
            commands: Default::default(),
            index: Default::default(),
        }
    }
}

impl Output {
    pub fn enabled(search_pattern: Search) -> Self {
        Self {
            enable: true,
            search_pattern,
            commands: Default::default(),
            mode: None,
            position: None,
            scale: None,
            transform: None,
            adaptive_sync: None,
        }
    }
    pub fn disabled(search_pattern: Search) -> Self {
        Self {
            enable: false,
            search_pattern,
            commands: Default::default(),
            mode: None,
            position: None,
            scale: None,
            transform: None,
            adaptive_sync: None,
        }
    }
    pub fn mode(&mut self, mode: Mode) {
        self.mode = Some(mode);
    }
    pub fn position(&mut self, position: Position) {
        self.position = Some(position);
    }
    pub fn scale(&mut self, scale: f64) {
        self.scale = Some(scale);
    }
    pub fn transform(&mut self, transform: Option<Transform>) {
        self.transform = transform;
    }
    pub fn adaptive_sync(&mut self, adaptive_sync: AdaptiveSyncState) {
        self.adaptive_sync = Some(adaptive_sync)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

impl Display for AdaptiveSyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdaptiveSyncState::Disabled => write!(f, "disabled"),
            AdaptiveSyncState::Enabled => write!(f, "enabled"),
        }
    }
}

impl Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Transform::Normal => "normal",
            Transform::_90 => "90",
            Transform::_180 => "180",
            Transform::_270 => "270",
            Transform::Flipped => "flipped",
            Transform::Flipped90 => "flipped-90",
            Transform::Flipped180 => "flipped-180",
            Transform::Flipped270 => "flipped-270",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, PartialEq, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum ParsePositionError {
    #[snafu(display("Missing separator"))]
    Separator,
    #[snafu(display("Missing x coordinate"))]
    MissingX,
    #[snafu(display("Missing y coordinate"))]
    MissingY,
    #[snafu(display("Failed to parse x or y to i32"))]
    ParseInt { source: ParseIntError },
}

impl FromStr for Position {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (x, y) = match s.split_once(',') {
            Some(split) => split,
            None => return SeparatorCtx {}.fail(),
        };
        if x.is_empty() {
            return MissingXCtx {}.fail();
        }
        if y.is_empty() {
            return MissingYCtx {}.fail();
        }
        let x: i32 = x.parse().context(ParseIntCtx)?;
        let y: i32 = y.parse().context(ParseIntCtx)?;
        Ok(Self { x, y })
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PositionVisitor;

        #[derive(Serialize, Deserialize)]
        struct PositionToml {
            pub x: i32,
            pub y: i32,
        }

        impl<'de> Visitor<'de> for PositionVisitor {
            type Value = Position;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string or struct")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                FromStr::from_str(value).map_err(serde::de::Error::custom)
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let pos: PositionToml =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(Position { x: pos.x, y: pos.y })
            }
        }

        deserializer.deserialize_any(PositionVisitor)
    }
}

impl Serialize for AdaptiveSyncState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let b = matches!(self, AdaptiveSyncState::Enabled);
        serializer.serialize_bool(b)
    }
}

impl<'de> Deserialize<'de> for AdaptiveSyncState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let a_sync: bool = bool::deserialize(deserializer)?;
        match a_sync {
            true => Ok(AdaptiveSyncState::Enabled),
            false => Ok(AdaptiveSyncState::Disabled),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use ParsePositionError::*;

    fn pos(x: i32, y: i32) -> Position {
        Position { x, y }
    }
    fn int_error(s: &str) -> ParsePositionError {
        ParseInt {
            source: i32::from_str(s).unwrap_err(),
        }
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct SimpleToml {
        pos: Position,
    }

    #[rstest]
    #[case("0,0", pos(0, 0))]
    #[case("1920,1080", pos(1920, 1080))]
    #[case("-1920,-1080", pos(-1920, -1080))]
    #[case("1000000,2", pos(1_000_000, 2))]
    fn serde_serialize_pos_string_ok(#[case] s: &str, #[case] pos: Position) {
        assert_eq!(Ok(format!("\"{}\"", s)), toml::to_string(&pos));
    }

    #[rstest]
    #[case("0,0", pos(0, 0))]
    #[case("1920,1080", pos(1920, 1080))]
    #[case("-1920,-1080", pos(-1920, -1080))]
    #[case("1000000,2", pos(1_000_000, 2))]
    fn serde_deserialize_pos_string_ok(#[case] s: &str, #[case] pos: Position) {
        let toml_str = toml::from_str(&format!("pos = \"{}\"", s));
        assert_eq!(toml_str, Ok(SimpleToml { pos }));
    }

    #[rstest]
    #[case("0,0", pos(0, 0))]
    #[case("1920,1080", pos(1920, 1080))]
    #[case("-1920,-1080", pos(-1920, -1080))]
    #[case("1000000,2", pos(1_000_000, 2))]
    fn serde_deserialize_pos_table_ok(#[case] s: &str, #[case] pos: Position) {
        let v: Vec<&str> = s.split(',').collect();
        let (x, y) = (v[0], v[1]);
        let toml_str = format!("pos = {{ x = {x}, y = {y} }}");
        assert_eq!(toml::from_str(&toml_str), Ok(SimpleToml { pos }));
    }

    #[rstest]
    #[case("0,0", pos(0, 0))]
    #[case("1920,1080", pos(1920, 1080))]
    #[case("-1920,-1080", pos(-1920, -1080))]
    #[case("1000000,2", pos(1_000_000, 2))]
    fn parse_position_from_str_ok(#[case] s: &str, #[case] pos: Position) {
        assert_eq!(Position::from_str(s), Ok(pos))
    }

    #[rstest]
    #[case("", Separator)]
    #[case(",1080", MissingX)]
    #[case("1920,", MissingY)]
    #[case("-19201080", Separator)]
    #[case("50000000000,123", int_error("5000000000"))]
    #[case("-50000000000,123", int_error("-5000000000"))]
    #[case("30.0,123", int_error("30.0"))]
    fn parse_position_from_str_err(#[case] s: &str, #[case] err: ParsePositionError) {
        assert_eq!(Position::from_str(s), Err(err))
    }
}
