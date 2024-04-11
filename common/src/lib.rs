use serde::{de::DeserializeOwned, Serialize};

/// Serializable message between `Client` and `Server`
pub trait Msg : Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}
impl<T> Msg for T where T : Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}
