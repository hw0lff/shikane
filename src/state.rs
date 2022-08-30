use crate::backend::ShikaneBackend;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Default, Debug)]
pub(crate) struct ShikaneState {
    pub(crate) backend: ShikaneBackend,
}
