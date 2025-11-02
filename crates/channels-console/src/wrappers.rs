#[cfg(feature = "futures")]
pub(crate) mod futures;
pub(crate) mod std;
#[cfg(feature = "tokio")]
pub(crate) mod tokio;
