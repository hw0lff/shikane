use crate::backend::ShikaneBackend;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Debug, Default)]
pub(crate) struct ShikaneState {
    pub(crate) backend: ShikaneBackend,
}
