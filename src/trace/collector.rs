#[cfg(not(feature = "multi_threaded"))]
use super::single_threaded_collector::SingleThreadedCollector;
#[cfg(feature = "multi_threaded")]
use super::multi_threaded_collector::MultiThreadedCollector;

/// A lock guard for yielding in multi-threaded mode. In single-threaded mode this is a unit type.
#[cfg(feature = "multi_threaded")]
pub type YieldLockGuard<'a> = std::sync::RwLockReadGuard<'a, ()>;

#[cfg(not(feature = "multi_threaded"))]
pub type YieldLockGuard<'a> = ();

/// The garbage collector type - aliases to either SingleThreadedCollector or MultiThreadedCollector
/// based on the `multi_threaded` feature flag.
#[cfg(not(feature = "multi_threaded"))]
pub type Collector = SingleThreadedCollector;

#[cfg(feature = "multi_threaded")]
pub type Collector = MultiThreadedCollector;
