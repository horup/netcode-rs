use serde::{de::DeserializeOwned, Serialize};

/// Serializable message between `Client` and `Server`
pub trait Message : Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}
impl<T> Message for T where T : Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        
    }
}
