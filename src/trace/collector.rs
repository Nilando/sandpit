#[cfg(feature = "multi_threaded")]
use super::multi_threaded_collector::MultiThreadedCollector;
#[cfg(not(feature = "multi_threaded"))]
use super::single_threaded_collector::SingleThreadedCollector;

/// The garbage collector type - aliases to either SingleThreadedCollector or MultiThreadedCollector
/// based on the `multi_threaded` feature flag.
#[cfg(not(feature = "multi_threaded"))]
pub type Collector = SingleThreadedCollector;

#[cfg(feature = "multi_threaded")]
pub type Collector = MultiThreadedCollector;
