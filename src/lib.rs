#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use zmaxion_dylib;
pub use zmaxion_internal::*;

// TODO: consumer groups
// TODO: synchronization: last writer sends data to first reader that has written
// TODO: error routing, pipes that return error can be piped to specific topic
// TODO: locals, memory that is private to pipes
// TODO: states, persistent memory
// TODO: backoff strategy
// TODO: pipe definition, require only one field
