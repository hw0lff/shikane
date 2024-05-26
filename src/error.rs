#[must_use]
pub(crate) fn report(error: &dyn snafu::Error) -> String {
    let sources = snafu::ChainCompat::new(error);
    let sources: Vec<&dyn snafu::Error> = sources.collect();
    let sources = sources.iter().rev();
    let mut s = String::new();
    for (i, source) in sources.enumerate() {
        s = match i {
            0 => format!("{source}"),
            _ => format!("{source} ({s})"),
        }
    }
    s
}
