use std::fmt::Display;
use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use snafu::{prelude::*, ResultExt};

use crate::wl_backend::WlMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Best,
    Preferred,
    WiHe(i32, i32),
    WiHeRe(i32, i32, i32),
    WiHeReCustom(i32, i32, i32),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModeToml {
    Best,
    Preferred,
    #[serde(untagged)]
    ModeMap(ModeMap),
}
#[derive(Serialize, Deserialize)]
struct ModeMap {
    pub width: i32,
    pub height: i32,
    pub refresh: Option<f32>,
    #[serde(default)]
    pub custom: bool,
}

impl Mode {
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::WiHeReCustom(_, _, _))
    }
    pub fn to_short_hz_string(&self) -> String {
        match self {
            Mode::Best => "best".to_string(),
            Mode::Preferred => "preferred".to_string(),
            Mode::WiHe(w, h) => format!("{w}x{h}"),
            Mode::WiHeRe(w, h, r) => {
                format!("{w}x{h}@{r}Hz", r = freq_milli_hz_to_hz(*r))
            }
            Mode::WiHeReCustom(w, h, r) => {
                format!("!{w}x{h}@{r}Hz", r = freq_milli_hz_to_hz(*r))
            }
        }
    }
    pub fn refresh(&self) -> Option<i32> {
        match self {
            Mode::WiHeRe(_, _, r) => Some(*r),
            Mode::WiHeReCustom(_, _, r) => Some(*r),
            _ => None,
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Best => write!(f, "best"),
            Mode::Preferred => write!(f, "preferred"),
            Mode::WiHe(w, h) => write!(f, "{w}x{h}"),
            Mode::WiHeRe(w, h, r) => write!(f, "{w}x{h}@{r}mHz"),
            Mode::WiHeReCustom(w, h, r) => write!(f, "custom({w}x{h}@{r}mHz)"),
        }
    }
}

impl From<ModeToml> for Mode {
    fn from(value: ModeToml) -> Self {
        match value {
            ModeToml::Best => Self::Best,
            ModeToml::Preferred => Self::Preferred,
            ModeToml::ModeMap(m) => {
                if let Some(refresh) = m.refresh {
                    if m.custom {
                        Self::WiHeReCustom(m.width, m.height, freq_hz_to_milli_hz(refresh))
                    } else {
                        Self::WiHeRe(m.width, m.height, freq_hz_to_milli_hz(refresh))
                    }
                } else {
                    Self::WiHe(m.width, m.height)
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Snafu)]
#[snafu(context(suffix(Ctx)))]
pub enum ParseModeError {
    #[snafu(display("Missing separator"))]
    MissingSeparator,
    #[snafu(display("Missing width"))]
    MissingWidth,
    #[snafu(display("Missing height"))]
    MissingHeight,
    #[snafu(display("Missing refresh rate"))]
    MissingRefresh,
    #[snafu(display("Failed to parse height or width into i32"))]
    ParseInt { source: ParseIntError },
    #[snafu(display("Failed to parse refresh rate into f32"))]
    ParseFloat { source: ParseFloatError },
    #[snafu(display("Value higher than 0 expected: {value}"))]
    LessOrEqualZero { value: i32 },
}

impl FromStr for Mode {
    type Err = ParseModeError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s == "best" {
            return Ok(Self::Best);
        }
        if s == "preferred" {
            return Ok(Self::Preferred);
        }
        let custom = s.starts_with('!');
        if custom {
            s = &s[1..];
        }
        let (w, hr) = match s.split_once('x') {
            Some(split) => split,
            None => return MissingSeparatorCtx {}.fail(),
        };

        if w.is_empty() {
            return MissingWidthCtx {}.fail();
        }
        if hr.is_empty() {
            return MissingHeightCtx {}.fail();
        }

        let (h, r) = if hr.contains('@') {
            let (h, mut r) = hr.split_once('@').unwrap();
            if r.ends_with("Hz") {
                r = r.strip_suffix("Hz").unwrap();
            }
            (h, Some(r))
        } else {
            (hr, None)
        };

        let w: i32 = w.parse().context(ParseIntCtx)?;
        let h: i32 = h.parse().context(ParseIntCtx)?;
        if w <= 0 {
            return LessOrEqualZeroCtx { value: w }.fail();
        }
        if h <= 0 {
            return LessOrEqualZeroCtx { value: h }.fail();
        }
        let r: Option<i32> = if let Some(r) = r {
            let r: f32 = r.parse().context(ParseFloatCtx)?;
            let r = freq_hz_to_milli_hz(r);
            if r <= 0 {
                return LessOrEqualZeroCtx { value: r }.fail();
            }
            Some(r)
        } else {
            None
        };

        if custom {
            if let Some(r) = r {
                return Ok(Self::WiHeReCustom(w, h, r));
            } else {
                return MissingRefreshCtx {}.fail();
            }
        }
        if let Some(r) = r {
            Ok(Self::WiHeRe(w, h, r))
        } else {
            Ok(Self::WiHe(w, h))
        }
    }
}

impl From<Mode> for ModeToml {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Best => Self::Best,
            Mode::Preferred => Self::Preferred,
            Mode::WiHe(width, height) => Self::ModeMap(ModeMap {
                width,
                height,
                refresh: None,
                custom: false,
            }),
            Mode::WiHeRe(width, height, refresh) => Self::ModeMap(ModeMap {
                width,
                height,
                refresh: Some(freq_milli_hz_to_hz(refresh)),
                custom: false,
            }),
            Mode::WiHeReCustom(width, height, refresh) => Self::ModeMap(ModeMap {
                width,
                height,
                refresh: Some(freq_milli_hz_to_hz(refresh)),
                custom: true,
            }),
        }
    }
}

impl From<WlMode> for Mode {
    fn from(value: WlMode) -> Self {
        let m = value.wl_base_mode();
        Self::WiHeRe(m.width, m.height, m.refresh)
    }
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_short_hz_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModeVisitor;
        impl<'de> Visitor<'de> for ModeVisitor {
            type Value = Mode;

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
                let mode: ModeToml =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(mode.into())
            }
        }

        deserializer.deserialize_any(ModeVisitor)
    }
}

/// Converts Hz to mHz, truncating remaining precision.
///
/// frequency    = 59.992_345f32  Hz
/// (_) * 1000.0 = 59_992.345f32 mHz
/// (_).trunc()  = 59_992.0f32   mHz
/// (_) as i32   = 59_992i32     mHz
fn freq_hz_to_milli_hz(frequency: f32) -> i32 {
    (frequency * 1000.0).trunc() as i32
}

/// Converts mHz to Hz.
fn freq_milli_hz_to_hz(frequency: i32) -> f32 {
    frequency as f32 / 1000.0
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;
    use serde::{Deserialize, Serialize};

    use super::ParseModeError::*;
    use super::{Mode, ParseModeError};

    fn mwh(w: i32, h: i32) -> Mode {
        Mode::WiHe(w, h)
    }
    fn mwhr(w: i32, h: i32, r: i32) -> Mode {
        Mode::WiHeRe(w, h, r)
    }
    fn mwhrc(w: i32, h: i32, r: i32) -> Mode {
        Mode::WiHeReCustom(w, h, r)
    }
    fn parse_float_error(s: &str) -> ParseModeError {
        ParseFloat {
            source: f32::from_str(s).unwrap_err(),
        }
    }
    fn less_or_equal_zero_error(value: i32) -> ParseModeError {
        LessOrEqualZero { value }
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct SimpleToml {
        mode: Mode,
    }

    #[rstest]
    #[case("best", Mode::Best)]
    #[case("preferred", Mode::Preferred)]
    #[case("123x456", mwh(123, 456))]
    #[case("123x456@30Hz", mwhr(123, 456, 30_000))]
    #[case("123x456@30.123Hz", mwhr(123, 456, 30_123))]
    #[case("123x456@30.001Hz", mwhr(123, 456, 30_001))]
    #[case("!123x456@30Hz", mwhrc(123, 456, 30_000))]
    #[case("!123x456@30.123Hz", mwhrc(123, 456, 30_123))]
    #[case("!123x456@30.001Hz", mwhrc(123, 456, 30_001))]
    fn serde_serialize_mode_string_ok(#[case] s: &str, #[case] mode: Mode) {
        assert_eq!(Ok(format!("\"{}\"", s)), toml::to_string(&mode));
    }

    #[rstest]
    #[case("best", Mode::Best)]
    #[case("preferred", Mode::Preferred)]
    #[case("123x456", mwh(123, 456))]
    #[case("123x456@30Hz", mwhr(123, 456, 30_000))]
    #[case("123x456@30.123Hz", mwhr(123, 456, 30_123))]
    #[case("123x456@30.001Hz", mwhr(123, 456, 30_001))]
    #[case("!123x456@30Hz", mwhrc(123, 456, 30_000))]
    #[case("!123x456@30.123Hz", mwhrc(123, 456, 30_123))]
    #[case("!123x456@30.001Hz", mwhrc(123, 456, 30_001))]
    fn serde_deserialize_mode_string_ok(#[case] s: &str, #[case] mode: Mode) {
        let toml_str = toml::from_str(&format!("mode = \"{}\"", s));
        assert_eq!(toml_str, Ok(SimpleToml { mode }));
    }

    #[rstest]
    #[case("123,456", mwh(123, 456))]
    #[case("123,456,30", mwhr(123, 456, 30_000))]
    #[case("123,456,30.123", mwhr(123, 456, 30_123))]
    #[case("123,456,30.001", mwhr(123, 456, 30_001))]
    #[case("123,456,30", mwhrc(123, 456, 30_000))]
    #[case("123,456,30.0", mwhrc(123, 456, 30_000))]
    #[case("123,456,30.123", mwhrc(123, 456, 30_123))]
    #[case("123,456,30.001", mwhrc(123, 456, 30_001))]
    #[case("123,456,30.000001", mwhrc(123, 456, 30_000))]
    fn serde_deserialize_mode_table_ok(#[case] s: &str, #[case] mode: Mode) {
        let v: Vec<&str> = s.split(',').collect();
        let (w, h) = (v[0], v[1]);
        let toml_str = if mode.is_custom() {
            let r = v[2];
            format!("mode = {{ width = {w}, height = {h}, refresh = {r}, custom = true }}")
        } else if let Some(r) = v.get(2) {
            format!("mode = {{ width = {w}, height = {h}, refresh = {r} }}")
        } else {
            format!("mode = {{ width = {w}, height = {h} }}")
        };
        assert_eq!(toml::from_str(&toml_str), Ok(SimpleToml { mode }));
    }

    #[rstest]
    #[case("b", MissingSeparator)]
    #[case("p", MissingSeparator)]
    #[case("!", MissingSeparator)]
    #[case("x", MissingWidth)]
    #[case("@", MissingSeparator)]
    #[case("Hz", MissingSeparator)]
    #[case("!1920x1080", MissingRefresh)]
    #[case("!1920x1080@", parse_float_error(""))]
    #[case("!1920x1080@word", parse_float_error("word"))]
    #[case("!1920x1080@wordHz", parse_float_error("word"))]
    #[case("!1920x1080@Hz", parse_float_error(""))]
    #[case("!1920x1080@0Hz", less_or_equal_zero_error(0))]
    #[case("!1920x1080@0", less_or_equal_zero_error(0))]
    #[case("!0x1080@10", less_or_equal_zero_error(0))]
    #[case("!1920x0@10", less_or_equal_zero_error(0))]
    #[case("1920x1080@", parse_float_error(""))]
    #[case("1920x1080@word", parse_float_error("word"))]
    #[case("1920x1080@wordHz", parse_float_error("word"))]
    #[case("1920x1080@Hz", parse_float_error(""))]
    #[case("1920x1080@0Hz", less_or_equal_zero_error(0))]
    #[case("1920x1080@0", less_or_equal_zero_error(0))]
    #[case("0x1080@10", less_or_equal_zero_error(0))]
    #[case("1920x0@10", less_or_equal_zero_error(0))]
    fn parse_mode_from_str_err(#[case] s: &str, #[case] mode: ParseModeError) {
        assert_eq!(Mode::from_str(s), Err(mode));
    }

    #[rstest]
    #[case("best", Mode::Best)]
    #[case("preferred", Mode::Preferred)]
    #[case("123x456", mwh(123, 456))]
    #[case("123x456@30", mwhr(123, 456, 30_000))]
    #[case("123x456@30.123", mwhr(123, 456, 30_123))]
    #[case("123x456@30.123456789", mwhr(123, 456, 30_123))]
    #[case("123x456@30Hz", mwhr(123, 456, 30_000))]
    #[case("123x456@30.123Hz", mwhr(123, 456, 30_123))]
    #[case("123x456@30.123456789Hz", mwhr(123, 456, 30_123))]
    #[case("!123x456@30", mwhrc(123, 456, 30_000))]
    #[case("!123x456@30.123", mwhrc(123, 456, 30_123))]
    #[case("!123x456@30.123456789", mwhrc(123, 456, 30_123))]
    #[case("!123x456@30Hz", mwhrc(123, 456, 30_000))]
    #[case("!123x456@30.123Hz", mwhrc(123, 456, 30_123))]
    #[case("!123x456@30.123456789Hz", mwhrc(123, 456, 30_123))]
    fn parse_mode_from_str_ok(#[case] s: &str, #[case] mode: Mode) {
        assert_eq!(Mode::from_str(s), Ok(mode));
    }
}
