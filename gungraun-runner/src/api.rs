//! The api contains all elements which the `runner` can understand

use std::ffi::OsString;
use std::fmt::Display;
#[cfg(feature = "runner")]
use std::fs::File;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
#[cfg(feature = "runner")]
use std::process::{Child, Command as StdCommand, Stdio as StdStdio};
#[cfg(feature = "runner")]
use std::str::FromStr;
use std::time::Duration;

#[cfg(feature = "runner")]
use anyhow::anyhow;
#[cfg(feature = "runner")]
use indexmap::indexset;
#[cfg(feature = "runner")]
use indexmap::IndexSet;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "runner")]
use strum::{EnumIter, IntoEnumIterator};

#[cfg(feature = "runner")]
use crate::runner;
#[cfg(feature = "runner")]
use crate::runner::metrics::Summarize;
#[cfg(feature = "runner")]
use crate::runner::metrics::TypeChecker;

/// All metrics which cachegrind produces and additionally some derived events
///
/// Depending on the options passed to Cachegrind, these are the events that Cachegrind can produce.
/// See the [Cachegrind
/// documentation](https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts) for details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "runner", derive(EnumIter))]
pub enum CachegrindMetric {
    /// The default event. I cache reads (which equals the number of instructions executed)
    Ir,
    /// D Cache reads (which equals the number of memory reads) (--cache-sim=yes)
    Dr,
    /// D Cache writes (which equals the number of memory writes) (--cache-sim=yes)
    Dw,
    /// I1 cache read misses (--cache-sim=yes)
    I1mr,
    /// D1 cache read misses (--cache-sim=yes)
    D1mr,
    /// D1 cache write misses (--cache-sim=yes)
    D1mw,
    /// LL cache instruction read misses (--cache-sim=yes)
    ILmr,
    /// LL cache data read misses (--cache-sim=yes)
    DLmr,
    /// LL cache data write misses (--cache-sim=yes)
    DLmw,
    /// I1 cache miss rate (--cache-sim=yes)
    I1MissRate,
    /// LL/L2 instructions cache miss rate (--cache-sim=yes)
    LLiMissRate,
    /// D1 cache miss rate (--cache-sim=yes)
    D1MissRate,
    /// LL/L2 data cache miss rate (--cache-sim=yes)
    LLdMissRate,
    /// LL/L2 cache miss rate (--cache-sim=yes)
    LLMissRate,
    /// Derived event showing the L1 hits (--cache-sim=yes)
    L1hits,
    /// Derived event showing the LL hits (--cache-sim=yes)
    LLhits,
    /// Derived event showing the RAM hits (--cache-sim=yes)
    RamHits,
    /// L1 cache hit rate (--cache-sim=yes)
    L1HitRate,
    /// LL/L2 cache hit rate (--cache-sim=yes)
    LLHitRate,
    /// RAM hit rate (--cache-sim=yes)
    RamHitRate,
    /// Derived event showing the total amount of cache reads and writes (--cache-sim=yes)
    TotalRW,
    /// Derived event showing estimated CPU cycles (--cache-sim=yes)
    EstimatedCycles,
    /// Conditional branches executed (--branch-sim=yes)
    Bc,
    /// Conditional branches mispredicted (--branch-sim=yes)
    Bcm,
    /// Indirect branches executed (--branch-sim=yes)
    Bi,
    /// Indirect branches mispredicted (--branch-sim=yes)
    Bim,
}

/// A collection of groups of [`CachegrindMetric`]s
///
/// The members of each group are fully documented in the docs of each variant of this enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CachegrindMetrics {
    /// The default group contains all metrics except the [`CachegrindMetrics::CacheMisses`],
    /// [`CachegrindMetrics::CacheMissRates`], [`CachegrindMetrics::CacheHitRates`] and
    /// [`EventKind::Dr`], [`EventKind::Dw`]. More specifically, the following event kinds and
    /// groups in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetrics, CachegrindMetric};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::Ir.into(),
    ///     CachegrindMetrics::CacheHits,
    ///     CachegrindMetric::TotalRW.into(),
    ///     CachegrindMetric::EstimatedCycles.into(),
    ///     CachegrindMetrics::BranchSim,
    /// ];
    /// ```
    #[default]
    Default,

    /// The `CacheMisses` produced by `--cache-sim=yes` contain the following [`CachegrindMetric`]s
    /// in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::I1mr.into(),
    ///     CachegrindMetric::D1mr.into(),
    ///     CachegrindMetric::D1mw.into(),
    ///     CachegrindMetric::ILmr.into(),
    ///     CachegrindMetric::DLmr.into(),
    ///     CachegrindMetric::DLmw.into(),
    /// ];
    /// ```
    CacheMisses,

    /// The cache miss rates calculated from the [`CallgrindMetrics::CacheMisses`] produced by
    /// `--cache-sim`:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::I1MissRate.into(),
    ///     CachegrindMetric::LLiMissRate.into(),
    ///     CachegrindMetric::D1MissRate.into(),
    ///     CachegrindMetric::LLdMissRate.into(),
    ///     CachegrindMetric::LLMissRate.into(),
    /// ];
    /// ```
    CacheMissRates,

    /// `CacheHits` are iai-callgrind specific and calculated from the metrics produced by
    /// `--cache-sim=yes` in this order:
    ///
    /// ```
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::L1hits.into(),
    ///     CachegrindMetric::LLhits.into(),
    ///     CachegrindMetric::RamHits.into(),
    /// ];
    /// ```
    CacheHits,

    /// The cache hit rates calculated from the [`CachegrindMetrics::CacheHits`]:
    ///
    /// ```
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::L1HitRate.into(),
    ///     CachegrindMetric::LLHitRate.into(),
    ///     CachegrindMetric::RamHitRate.into(),
    /// ];
    /// ```
    CacheHitRates,

    /// All metrics produced by `--cache-sim=yes` including the iai-callgrind specific metrics
    /// [`CachegrindMetric::L1hits`], [`CachegrindMetric::LLhits`], [`CachegrindMetric::RamHits`],
    /// [`CachegrindMetric::TotalRW`], [`CachegrindMetric::EstimatedCycles`],
    /// [`CachegrindMetrics::CacheMissRates`] and [`CachegrindMetrics::CacheHitRates`] in this
    /// order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::Dr.into(),
    ///     CachegrindMetric::Dw.into(),
    ///     CachegrindMetrics::CacheMisses,
    ///     CachegrindMetrics::CacheMissRates,
    ///     CachegrindMetrics::CacheHits,
    ///     CachegrindMetrics::CacheHitRates,
    ///     CachegrindMetric::TotalRW.into(),
    ///     CachegrindMetric::EstimatedCycles.into(),
    /// ];
    /// ```
    CacheSim,

    /// The metrics produced by `--branch-sim=yes` in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::Bc.into(),
    ///     CachegrindMetric::Bcm.into(),
    ///     CachegrindMetric::Bi.into(),
    ///     CachegrindMetric::Bim.into(),
    /// ];
    /// ```
    BranchSim,

    /// All possible [`CachegrindMetric`]s in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// let metrics: Vec<CachegrindMetrics> = vec![
    ///     CachegrindMetric::Ir.into(),
    ///     CachegrindMetrics::CacheSim,
    ///     CachegrindMetrics::BranchSim,
    /// ];
    /// ```
    All,

    /// Selection of no [`CachegrindMetric`] at all
    None,

    /// Specify a single [`CachegrindMetric`].
    ///
    /// Note that [`CachegrindMetric`] implements the necessary traits to convert to the
    /// `CachegrindMetrics::SingleEvent` variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CachegrindMetric, CachegrindMetrics};
    /// # }
    /// use iai_callgrind::{CachegrindMetric, CachegrindMetrics};
    ///
    /// assert_eq!(
    ///     CachegrindMetrics::SingleEvent(CachegrindMetric::Ir),
    ///     CachegrindMetric::Ir.into()
    /// );
    /// ```
    SingleEvent(CachegrindMetric),
}

/// A collection of groups of [`EventKind`]s
///
/// `Callgrind` supports a large amount of metrics and their collection can be enabled with various
/// command-line flags. [`CallgrindMetrics`] groups these metrics to make it less cumbersome to
/// specify multiple [`EventKind`]s at once if necessary.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CallgrindMetrics {
    /// The default group contains all event kinds except the [`CallgrindMetrics::CacheMisses`],
    /// [`CallgrindMetrics::CacheMissRates`], [`CallgrindMetrics::CacheHitRates`] and
    /// [`EventKind::Dr`], [`EventKind::Dw`]. More specifically, the following event kinds and
    /// groups in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::Ir.into(),
    ///     CallgrindMetrics::CacheHits,
    ///     EventKind::TotalRW.into(),
    ///     EventKind::EstimatedCycles.into(),
    ///     CallgrindMetrics::SystemCalls,
    ///     EventKind::Ge.into(),
    ///     CallgrindMetrics::BranchSim,
    ///     CallgrindMetrics::WriteBackBehaviour,
    ///     CallgrindMetrics::CacheUse,
    /// ];
    /// ```
    #[default]
    Default,

    /// The `CacheMisses` produced by `--cache-sim=yes` contain the following [`EventKind`]s in
    /// this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::I1mr.into(),
    ///     EventKind::D1mr.into(),
    ///     EventKind::D1mw.into(),
    ///     EventKind::ILmr.into(),
    ///     EventKind::DLmr.into(),
    ///     EventKind::DLmw.into(),
    /// ];
    /// ```
    CacheMisses,

    /// The cache miss rates calculated from the [`CallgrindMetrics::CacheMisses`] produced by
    /// `--cache-sim`:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::I1MissRate.into(),
    ///     EventKind::D1MissRate.into(),
    ///     EventKind::LLiMissRate.into(),
    ///     EventKind::LLdMissRate.into(),
    ///     EventKind::LLMissRate.into(),
    /// ];
    /// ```
    CacheMissRates,

    /// `CacheHits` are iai-callgrind specific and calculated from the metrics produced by
    /// `--cache-sim=yes` in this order:
    ///
    /// ```
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::L1hits.into(),
    ///     EventKind::LLhits.into(),
    ///     EventKind::RamHits.into(),
    /// ];
    /// ```
    CacheHits,

    /// The cache hit rates calculated from the [`CallgrindMetrics::CacheHits`]:
    ///
    /// ```
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::L1HitRate.into(),
    ///     EventKind::LLHitRate.into(),
    ///     EventKind::RamHitRate.into(),
    /// ];
    /// ```
    CacheHitRates,

    /// All metrics produced by `--cache-sim=yes` including the iai-callgrind specific metrics
    /// [`EventKind::L1hits`], [`EventKind::LLhits`], [`EventKind::RamHits`],
    /// [`EventKind::TotalRW`], [`EventKind::EstimatedCycles`] and miss/hit rates in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::Dr.into(),
    ///     EventKind::Dw.into(),
    ///     CallgrindMetrics::CacheMisses,
    ///     CallgrindMetrics::CacheMissRates,
    ///     CallgrindMetrics::CacheHits,
    ///     EventKind::TotalRW.into(),
    ///     CallgrindMetrics::CacheHitRates,
    ///     EventKind::EstimatedCycles.into(),
    /// ];
    /// ```
    CacheSim,

    /// The metrics produced by `--cacheuse=yes` in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::AcCost1.into(),
    ///     EventKind::AcCost2.into(),
    ///     EventKind::SpLoss1.into(),
    ///     EventKind::SpLoss2.into(),
    /// ];
    /// ```
    CacheUse,

    /// `SystemCalls` are events of the `--collect-systime=yes` option in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::SysCount.into(),
    ///     EventKind::SysTime.into(),
    ///     EventKind::SysCpuTime.into(),
    /// ];
    /// ```
    SystemCalls,

    /// The metrics produced by `--branch-sim=yes` in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::Bc.into(),
    ///     EventKind::Bcm.into(),
    ///     EventKind::Bi.into(),
    ///     EventKind::Bim.into(),
    /// ];
    /// ```
    BranchSim,

    /// All metrics of `--simulate-wb=yes` in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::ILdmr.into(),
    ///     EventKind::DLdmr.into(),
    ///     EventKind::DLdmw.into(),
    /// ];
    /// ```
    WriteBackBehaviour,

    /// All possible [`EventKind`]s in this order:
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// let metrics: Vec<CallgrindMetrics> = vec![
    ///     EventKind::Ir.into(),
    ///     CallgrindMetrics::CacheSim,
    ///     CallgrindMetrics::SystemCalls,
    ///     EventKind::Ge.into(),
    ///     CallgrindMetrics::BranchSim,
    ///     CallgrindMetrics::WriteBackBehaviour,
    ///     CallgrindMetrics::CacheUse,
    /// ];
    /// ```
    All,

    /// Selection of no [`EventKind`] at all
    None,

    /// Specify a single [`EventKind`].
    ///
    /// Note that [`EventKind`] implements the necessary traits to convert to the
    /// `CallgrindMetrics::SingleEvent` variant which is shorter to write.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{CallgrindMetrics, EventKind};
    /// # }
    /// use iai_callgrind::{CallgrindMetrics, EventKind};
    ///
    /// assert_eq!(
    ///     CallgrindMetrics::SingleEvent(EventKind::Ir),
    ///     EventKind::Ir.into()
    /// );
    /// ```
    SingleEvent(EventKind),
}

/// The kind of `Delay`
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelayKind {
    /// Delay the `Command` for a fixed [`Duration`]
    DurationElapse(Duration),
    /// Delay the `Command` until a successful tcp connection can be established
    TcpConnect(SocketAddr),
    /// Delay the `Command` until a successful udp response was received
    UdpResponse(SocketAddr, Vec<u8>),
    /// Delay the `Command` until the specified path exists
    PathExists(PathBuf),
}

/// The metrics collected by DHAT
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "runner", derive(EnumIter))]
pub enum DhatMetric {
    /// In ad-hoc mode, Total units measured over the entire execution
    TotalUnits,
    /// Total ad-hoc events over the entire execution
    TotalEvents,
    /// Total bytes allocated over the entire execution
    TotalBytes,
    /// Total heap blocks allocated over the entire execution
    TotalBlocks,
    /// The bytes alive at t-gmax, the time when the heap size reached its global maximum
    AtTGmaxBytes,
    /// The blocks alive at t-gmax
    AtTGmaxBlocks,
    /// The amount of bytes at the end of the execution.
    ///
    /// This is the amount of bytes which were not explicitly freed.
    AtTEndBytes,
    /// The amount of blocks at the end of the execution.
    ///
    /// This is the amount of heap blocks which were not explicitly freed.
    AtTEndBlocks,
    /// The amount of bytes read during the entire execution
    ReadsBytes,
    /// The amount of bytes written during the entire execution
    WritesBytes,
    /// The total lifetimes of all heap blocks allocated
    TotalLifetimes,
    /// The maximum amount of bytes
    MaximumBytes,
    /// The maximum amount of heap blocks
    MaximumBlocks,
}

/// A collection of groups of [`DhatMetric`]s
///
/// The members of each group are fully documented in the docs of each variant of this enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhatMetrics {
    /// The default group in this order
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{DhatMetrics, DhatMetric};
    /// # }
    /// use iai_callgrind::{DhatMetric, DhatMetrics};
    ///
    /// let metrics: Vec<DhatMetrics> = vec![
    ///     DhatMetric::TotalUnits.into(),
    ///     DhatMetric::TotalEvents.into(),
    ///     DhatMetric::TotalBytes.into(),
    ///     DhatMetric::TotalBlocks.into(),
    ///     DhatMetric::AtTGmaxBytes.into(),
    ///     DhatMetric::AtTGmaxBlocks.into(),
    ///     DhatMetric::AtTEndBytes.into(),
    ///     DhatMetric::AtTEndBlocks.into(),
    ///     DhatMetric::ReadsBytes.into(),
    ///     DhatMetric::WritesBytes.into(),
    /// ];
    /// ```
    #[default]
    Default,

    /// All [`DhatMetric`]s in this order
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{DhatMetrics, DhatMetric};
    /// # }
    /// use iai_callgrind::{DhatMetric, DhatMetrics};
    ///
    /// let metrics: Vec<DhatMetrics> = vec![
    ///     DhatMetrics::Default,
    ///     DhatMetric::TotalLifetimes.into(),
    ///     DhatMetric::MaximumBytes.into(),
    ///     DhatMetric::MaximumBlocks.into(),
    /// ];
    /// ```
    All,

    /// A single [`DhatMetric`]
    ///
    /// ```rust
    /// # pub mod iai_callgrind {
    /// # pub use gungraun_runner::api::{DhatMetrics, DhatMetric};
    /// # }
    /// use iai_callgrind::{DhatMetric, DhatMetrics};
    ///
    /// assert_eq!(
    ///     DhatMetrics::SingleMetric(DhatMetric::TotalBytes),
    ///     DhatMetric::TotalBytes.into()
    /// );
    /// ```
    SingleMetric(DhatMetric),
}

/// The `Direction` in which the flamegraph should grow.
///
/// The default is `TopToBottom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Grow from top to bottom with the highest event costs at the top
    TopToBottom,
    /// Grow from bottom to top with the highest event costs at the bottom
    BottomToTop,
}

/// The `EntryPoint` of a benchmark
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryPoint {
    /// Disable the entry point
    None,
    /// The default entry point is the benchmark function
    #[default]
    Default,
    /// A custom entry point. The argument allows the same glob patterns as the
    /// [`--toggle-collect`](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)
    /// argument of callgrind. These are the wildcards `*` (match any amount of arbitrary
    /// characters) and `?` (match a single arbitrary character)
    Custom(String),
}

/// The error metrics from a tool which reports errors
///
/// The tools which report only errors are `helgrind`, `drd` and `memcheck`. The order in which the
/// variants are defined in this enum determines the order of the metrics in the benchmark terminal
/// output.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "runner", derive(EnumIter))]
pub enum ErrorMetric {
    /// The amount of detected unsuppressed errors
    Errors,
    /// The amount of detected unsuppressed error contexts
    Contexts,
    /// The amount of suppressed errors
    SuppressedErrors,
    /// The amount of suppressed error contexts
    SuppressedContexts,
}

/// All `EventKind`s callgrind produces and additionally some derived events
///
/// Depending on the options passed to Callgrind, these are the events that Callgrind can produce.
/// See the [Callgrind
/// documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "runner", derive(EnumIter))]
pub enum EventKind {
    /// The default event. I cache reads (which equals the number of instructions executed)
    Ir,
    /// D Cache reads (which equals the number of memory reads) (--cache-sim=yes)
    Dr,
    /// D Cache writes (which equals the number of memory writes) (--cache-sim=yes)
    Dw,
    /// I1 cache read misses (--cache-sim=yes)
    I1mr,
    /// D1 cache read misses (--cache-sim=yes)
    D1mr,
    /// D1 cache write misses (--cache-sim=yes)
    D1mw,
    /// LL cache instruction read misses (--cache-sim=yes)
    ILmr,
    /// LL cache data read misses (--cache-sim=yes)
    DLmr,
    /// LL cache data write misses (--cache-sim=yes)
    DLmw,
    /// I1 cache miss rate (--cache-sim=yes)
    I1MissRate,
    /// LL/L2 instructions cache miss rate (--cache-sim=yes)
    LLiMissRate,
    /// D1 cache miss rate (--cache-sim=yes)
    D1MissRate,
    /// LL/L2 data cache miss rate (--cache-sim=yes)
    LLdMissRate,
    /// LL/L2 cache miss rate (--cache-sim=yes)
    LLMissRate,
    /// Derived event showing the L1 hits (--cache-sim=yes)
    L1hits,
    /// Derived event showing the LL hits (--cache-sim=yes)
    LLhits,
    /// Derived event showing the RAM hits (--cache-sim=yes)
    RamHits,
    /// L1 cache hit rate (--cache-sim=yes)
    L1HitRate,
    /// LL/L2 cache hit rate (--cache-sim=yes)
    LLHitRate,
    /// RAM hit rate (--cache-sim=yes)
    RamHitRate,
    /// Derived event showing the total amount of cache reads and writes (--cache-sim=yes)
    TotalRW,
    /// Derived event showing estimated CPU cycles (--cache-sim=yes)
    EstimatedCycles,
    /// The number of system calls done (--collect-systime=yes)
    SysCount,
    /// The elapsed time spent in system calls (--collect-systime=yes)
    SysTime,
    /// The cpu time spent during system calls (--collect-systime=nsec)
    SysCpuTime,
    /// The number of global bus events (--collect-bus=yes)
    Ge,
    /// Conditional branches executed (--branch-sim=yes)
    Bc,
    /// Conditional branches mispredicted (--branch-sim=yes)
    Bcm,
    /// Indirect branches executed (--branch-sim=yes)
    Bi,
    /// Indirect branches mispredicted (--branch-sim=yes)
    Bim,
    /// Dirty miss because of instruction read (--simulate-wb=yes)
    ILdmr,
    /// Dirty miss because of data read (--simulate-wb=yes)
    DLdmr,
    /// Dirty miss because of data write (--simulate-wb=yes)
    DLdmw,
    /// Counter showing bad temporal locality for L1 caches (--cachuse=yes)
    AcCost1,
    /// Counter showing bad temporal locality for LL caches (--cachuse=yes)
    AcCost2,
    /// Counter showing bad spatial locality for L1 caches (--cachuse=yes)
    SpLoss1,
    /// Counter showing bad spatial locality for LL caches (--cachuse=yes)
    SpLoss2,
}

/// Set the expected exit status of a binary benchmark
///
/// Per default, the benchmarked binary is expected to succeed, but if a benchmark is expected to
/// fail, setting this option is required.
///
/// # Examples
///
/// ```rust,ignore
/// use iai_callgrind::{main, BinaryBenchmarkConfig, ExitWith};
///
/// # fn main() {
/// main!(
///     config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Code(1));
///     binary_benchmark_groups = my_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitWith {
    /// Exit with success is similar to `ExitCode(0)`
    Success,
    /// Exit with failure is similar to setting the `ExitCode` to something different from `0`
    /// without having to rely on a specific exit code
    Failure,
    /// The exact `ExitCode` of the benchmark run
    Code(i32),
}

/// The kind of `Flamegraph` which is going to be constructed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlamegraphKind {
    /// The regular flamegraph for the new callgrind run
    Regular,
    /// A differential flamegraph showing the differences between the new and old callgrind run
    Differential,
    /// All flamegraph kinds that can be constructed (`Regular` and `Differential`). This
    /// is the default.
    All,
    /// Do not produce any flamegraphs
    None,
}

/// A `Limit` which can be either an integer or a float
///
/// Depending on the metric the type of the hard limit is a float or an integer. For example
/// [`EventKind::Ir`] is an integer and [`EventKind::L1HitRate`] is a percentage and therefore a
/// float.
///
/// The type of the metric can be seen in the terminal output of Gungraun: Floats always
/// contain a `.` and integers do not.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Limit {
    /// An integer `Limit`. For example [`EventKind::Ir`]
    Int(u64),
    /// A float `Limit`. For example [`EventKind::L1HitRate`] or [`EventKind::I1MissRate`]
    Float(f64),
}

/// Configure the `Stream` which should be used as pipe in [`Stdin::Setup`]
///
/// The default is [`Pipe::Stdout`]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pipe {
    /// The `Stdout` default `Stream`
    #[default]
    Stdout,
    /// The `Stderr` error `Stream`
    Stderr,
}

/// This is a special `Stdio` for the stdin method of [`Command`]
///
/// Contains all the standard [`Stdio`] options and the [`Stdin::Setup`] option
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stdin {
    /// Using this in [`Command::stdin`] pipes the stream specified with [`Pipe`] of the `setup`
    /// function into the `Stdin` of the [`Command`]. In this case the `setup` and [`Command`] are
    /// executed in parallel instead of sequentially. See [`Command::stdin`] for more details.
    Setup(Pipe),
    #[default]
    /// See [`Stdio::Inherit`]
    Inherit,
    /// See [`Stdio::Null`]
    Null,
    /// See [`Stdio::File`]
    File(PathBuf),
    /// See [`Stdio::Pipe`]
    Pipe,
}

/// Configure the `Stdio` of `Stdin`, `Stdout` and `Stderr`
///
/// Describes what to do with a standard I/O stream for the [`Command`] when passed to the stdin,
/// stdout, and stderr methods of [`Command`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stdio {
    /// The [`Command`]'s `Stream` inherits from the benchmark runner.
    #[default]
    Inherit,
    /// This stream will be ignored. This is the equivalent of attaching the stream to `/dev/null`
    Null,
    /// Redirect the content of a file into this `Stream`. This is equivalent to a redirection in a
    /// shell for example for the `Stdout` of `my-command`: `my-command > some_file`
    File(PathBuf),
    /// A new pipe should be arranged to connect the benchmark runner and the [`Command`]
    Pipe,
}

/// We use this enum only internally in the benchmark runner
#[cfg(feature = "runner")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    Stdin,
    Stderr,
    Stdout,
}

/// The tool specific flamegraph configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFlamegraphConfig {
    /// The callgrind configuration
    Callgrind(FlamegraphConfig),
    /// The option for tools which can't create flamegraphs
    None,
}

/// The tool specific metrics to show in the terminal output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolOutputFormat {
    /// The Callgrind configuration
    Callgrind(Vec<CallgrindMetrics>),
    /// The Cachegrind configuration
    Cachegrind(Vec<CachegrindMetrics>),
    /// The DHAT configuration
    DHAT(Vec<DhatMetric>),
    /// The Memcheck configuration
    Memcheck(Vec<ErrorMetric>),
    /// The Helgrind configuration
    Helgrind(Vec<ErrorMetric>),
    /// The DRD configuration
    DRD(Vec<ErrorMetric>),
    /// If there is no configuration
    None,
}

/// The tool specific regression check configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolRegressionConfig {
    /// The cachegrind configuration
    Cachegrind(CachegrindRegressionConfig),
    /// The callgrind configuration
    Callgrind(CallgrindRegressionConfig),
    /// The dhat configuration
    Dhat(DhatRegressionConfig),
    /// The option for tools which don't perform regression checks
    None,
}

/// The valgrind tools which can be run
///
/// Note the default changes from `Callgrind` to `Cachegrind` if the `cachegrind` feature is
/// selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ValgrindTool {
    /// [Callgrind: a call-graph generating cache and branch prediction profiler](https://valgrind.org/docs/manual/cl-manual.html)
    Callgrind,
    /// [Cachegrind: a high-precision tracing profiler](https://valgrind.org/docs/manual/cg-manual.html)
    Cachegrind,
    /// [DHAT: a dynamic heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html)
    DHAT,
    /// [Memcheck: a memory error detector](https://valgrind.org/docs/manual/mc-manual.html)
    Memcheck,
    /// [Helgrind: a thread error detector](https://valgrind.org/docs/manual/hg-manual.html)
    Helgrind,
    /// [DRD: a thread error detector](https://valgrind.org/docs/manual/drd-manual.html)
    DRD,
    /// [Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html)
    Massif,
    /// [BBV: an experimental basic block vector generation tool](https://valgrind.org/docs/manual/bbv-manual.html)
    BBV,
}

/// The model for the `#[binary_benchmark]` attribute or the equivalent from the low level api
///
/// For internal use only
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmark {
    /// The extracted binary benchmarks
    pub benches: Vec<BinaryBenchmarkBench>,
    /// The configuration at `#[binary_benchmark]` level
    pub config: Option<BinaryBenchmarkConfig>,
}

/// For internal use only: Used to differentiate between the `iter` and other `#[benches]` arguments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandKind {
    /// The default mode when `iter` was not used
    Default(Box<Command>),
    /// The mode when `iter` was used
    Iter(Vec<Command>),
}

/// The model for the `#[bench]` attribute or the low level equivalent
///
/// For internal use only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryBenchmarkBench {
    /// The arguments to the function
    pub args: Option<String>,
    /// The returned [`Command`]
    pub command: CommandKind,
    /// The configuration at `#[bench]` or `#[benches]` level
    pub config: Option<BinaryBenchmarkConfig>,
    /// The name of the annotated function
    pub function_name: String,
    /// True if there is a `setup` function
    pub has_setup: bool,
    /// True if there is a `teardown` function
    pub has_teardown: bool,
    /// The `id` of the benchmark as in `#[bench::id]`
    pub id: Option<String>,
}

/// The model for the configuration in binary benchmarks
///
/// This is the configuration which is built from the configuration of the UI and for internal use
/// only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BinaryBenchmarkConfig {
    /// If some, set the the working directory of the benchmarked binary to this path
    pub current_dir: Option<PathBuf>,
    /// The valgrind tool to run instead of the default callgrind
    pub default_tool: Option<ValgrindTool>,
    /// True if the environment variables should be cleared
    pub env_clear: Option<bool>,
    /// The environment variables to set or pass through to the binary
    pub envs: Vec<(OsString, Option<OsString>)>,
    /// The [`ExitWith`] to set the expected exit code/signal of the benchmarked binary
    pub exit_with: Option<ExitWith>,
    /// The configuration of the output format
    pub output_format: Option<OutputFormat>,
    /// Run the benchmarked binary in a [`Sandbox`] or not
    pub sandbox: Option<Sandbox>,
    /// Run the `setup` function parallel to the benchmarked binary
    pub setup_parallel: Option<bool>,
    /// The valgrind tools to run in addition to the default tool
    pub tools: Tools,
    /// The tool override at this configuration level
    pub tools_override: Option<Tools>,
    /// The arguments to pass to all tools
    pub valgrind_args: RawArgs,
}

/// The model for the `binary_benchmark_group` macro
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroup {
    /// The actual data and the benchmarks of this group
    pub binary_benchmarks: Vec<BinaryBenchmark>,
    /// If true compare the benchmarks in this group
    pub compare_by_id: Option<bool>,
    /// The configuration at this level
    pub config: Option<BinaryBenchmarkConfig>,
    /// True if there is a `setup` function
    pub has_setup: bool,
    /// True if there is a `teardown` function
    pub has_teardown: bool,
    /// The name or id of the `binary_benchmark_group!`
    pub id: String,
}

/// The model for the main! macro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroups {
    /// The command line arguments as we receive them from `cargo bench`
    pub command_line_args: Vec<String>,
    /// The configuration of this level
    pub config: BinaryBenchmarkConfig,
    /// The default tool changed by the `cachegrind` feature
    pub default_tool: ValgrindTool,
    /// All groups of this benchmark
    pub groups: Vec<BinaryBenchmarkGroup>,
    /// True if there is a `setup` function
    pub has_setup: bool,
    /// True if there is a `teardown` function
    pub has_teardown: bool,
}

/// The model for the regression check configuration of Cachegrind
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CachegrindRegressionConfig {
    /// True if the benchmarks should fail on the first occurrence of a regression
    pub fail_fast: Option<bool>,
    /// The hard limits
    pub hard_limits: Vec<(CachegrindMetrics, Limit)>,
    /// The soft limits
    pub soft_limits: Vec<(CachegrindMetrics, f64)>,
}

/// The model for the regression check configuration of Callgrind
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CallgrindRegressionConfig {
    /// True if the benchmarks should fail on the first occurrence of a regression
    pub fail_fast: Option<bool>,
    /// The hard limits
    pub hard_limits: Vec<(CallgrindMetrics, Limit)>,
    /// The soft limits
    pub soft_limits: Vec<(CallgrindMetrics, f64)>,
}

/// The model for the command returned by the binary benchmark function
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Command {
    /// The arguments for the executable
    pub args: Vec<OsString>,
    /// The configuration at this level
    pub config: BinaryBenchmarkConfig,
    /// If present the command is delayed as configured in [`Delay`]
    pub delay: Option<Delay>,
    /// The path to the executable
    pub path: PathBuf,
    /// The command's stderr
    pub stderr: Option<Stdio>,
    /// The command's stdin
    pub stdin: Option<Stdin>,
    /// The command's stdout
    pub stdout: Option<Stdio>,
}

/// The delay of the [`Command`]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delay {
    /// The kind of delay
    pub kind: DelayKind,
    /// The polling time to check the delay condition
    pub poll: Option<Duration>,
    /// The timeout for the delay
    pub timeout: Option<Duration>,
}

/// The model for the regression check configuration of DHAT
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DhatRegressionConfig {
    /// True if the benchmarks should fail on the first occurrence of a regression
    pub fail_fast: Option<bool>,
    /// The hard limits
    pub hard_limits: Vec<(DhatMetrics, Limit)>,
    /// The soft limits
    pub soft_limits: Vec<(DhatMetrics, f64)>,
}

/// The fixtures to copy into the [`Sandbox`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixtures {
    /// If true, follow symlinks
    pub follow_symlinks: bool,
    /// The path to the fixtures
    pub path: PathBuf,
}

/// The model for the configuration of flamegraphs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FlamegraphConfig {
    /// The direction of the flamegraph. Top to bottom or vice versa
    pub direction: Option<Direction>,
    /// The event kinds for which a flamegraph should be generated
    pub event_kinds: Option<Vec<EventKind>>,
    /// The flamegraph kind
    pub kind: Option<FlamegraphKind>,
    /// The minimum width which should be displayed
    pub min_width: Option<f64>,
    /// If true, negate a differential flamegraph
    pub negate_differential: Option<bool>,
    /// If true, normalize a differential flamegraph
    pub normalize_differential: Option<bool>,
    /// The subtitle to use for the flamegraphs
    pub subtitle: Option<String>,
    /// The title to use for the flamegraphs
    pub title: Option<String>,
}

/// The model for the `#[library_benchmark]` attribute
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmark {
    /// The extracted benchmarks of the annotated function
    pub benches: Vec<LibraryBenchmarkBench>,
    /// The configuration at this level
    pub config: Option<LibraryBenchmarkConfig>,
}

/// The model for the `#[bench]` attribute in a `#[library_benchmark]`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkBench {
    /// The arguments for the function
    pub args: Option<String>,
    /// The configuration at this level
    pub config: Option<LibraryBenchmarkConfig>,
    /// The name of the function
    pub function_name: String,
    /// The id of the attribute as in `#[bench::id]`
    pub id: Option<String>,
    /// The amount of elements in the iterator of the `#[benches::id(iter = ITERATOR)]` if present
    pub iter_count: Option<usize>,
}

/// The model for the configuration in library benchmarks
///
/// This is the configuration which is built from the configuration of the UI and for internal use
/// only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkConfig {
    /// The valgrind tool to run instead of the default callgrind
    pub default_tool: Option<ValgrindTool>,
    /// True if the environment variables should be cleared
    pub env_clear: Option<bool>,
    /// The environment variables to set or pass through to the binary
    pub envs: Vec<(OsString, Option<OsString>)>,
    /// The configuration of the output format
    pub output_format: Option<OutputFormat>,
    /// The valgrind tools to run in addition to the default tool
    pub tools: Tools,
    /// The tool override at this configuration level
    pub tools_override: Option<Tools>,
    /// The arguments to pass to all tools
    pub valgrind_args: RawArgs,
}

/// The model for the `library_benchmark_group` macro
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroup {
    /// If true compare the benchmarks in this group
    pub compare_by_id: Option<bool>,
    /// The configuration at this level
    pub config: Option<LibraryBenchmarkConfig>,
    /// True if there is a `setup` function
    pub has_setup: bool,
    /// True if there is a `teardown` function
    pub has_teardown: bool,
    /// The name or id of the `library_benchmark_group!`
    pub id: String,
    /// The actual data and the benchmarks of this group
    pub library_benchmarks: Vec<LibraryBenchmark>,
}

/// The model for the `main` macro
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroups {
    /// The command line args as we receive them from `cargo bench`
    pub command_line_args: Vec<String>,
    /// The configuration of this level
    pub config: LibraryBenchmarkConfig,
    /// The default tool changed by the `cachegrind` feature
    pub default_tool: ValgrindTool,
    /// All groups of this benchmark
    pub groups: Vec<LibraryBenchmarkGroup>,
    /// True if there is a `setup` function
    pub has_setup: bool,
    /// True if there is a `teardown` function
    pub has_teardown: bool,
}

/// The configuration values for the output format
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OutputFormat {
    /// Show a grid instead of spaces in the terminal output
    pub show_grid: Option<bool>,
    /// Show intermediate results, for example in benchmarks for multi-threaded applications
    pub show_intermediate: Option<bool>,
    /// Don't show differences within the tolerance margin
    pub tolerance: Option<f64>,
    /// If set, truncate the description
    pub truncate_description: Option<Option<usize>>,
}

/// The raw arguments to pass to a valgrind tool
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawArgs(pub Vec<String>);

/// The sandbox to run the benchmarks in
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sandbox {
    /// If this sandbox is enabled or not
    pub enabled: Option<bool>,
    /// The fixtures to copy into the sandbox
    pub fixtures: Vec<PathBuf>,
    /// If true follow symlinks when copying the fixtures
    pub follow_symlinks: Option<bool>,
}

/// The tool configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// If true the tool is run. Ignored for the default tool which always runs
    pub enable: Option<bool>,
    /// The entry point for the tool
    pub entry_point: Option<EntryPoint>,
    /// The configuration for flamegraphs
    pub flamegraph_config: Option<ToolFlamegraphConfig>,
    /// Any frames in the call stack which should be considered in addition to the entry point
    pub frames: Option<Vec<String>>,
    /// The valgrind tool this configuration is for
    pub kind: ValgrindTool,
    /// The configuration of the output format
    pub output_format: Option<ToolOutputFormat>,
    /// The arguments to pass to the tool
    pub raw_args: RawArgs,
    /// The configuration for regression checks of tools which perform regression checks
    pub regression_config: Option<ToolRegressionConfig>,
    /// If true show the logging output of Valgrind (not Gungraun)
    pub show_log: Option<bool>,
}

/// The configurations of all tools to run in addition to the default tool
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tools(pub Vec<Tool>);

impl BinaryBenchmarkConfig {
    /// Update this configuration with all other configurations in the given order
    #[must_use]
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.default_tool = update_option(&self.default_tool, &other.default_tool);
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.current_dir = update_option(&self.current_dir, &other.current_dir);
            self.exit_with = update_option(&self.exit_with, &other.exit_with);

            self.valgrind_args
                .extend_ignore_flag(other.valgrind_args.0.iter());

            self.envs.extend_from_slice(&other.envs);

            if let Some(other_tools) = &other.tools_override {
                self.tools = other_tools.clone();
            } else if !other.tools.is_empty() {
                self.tools.update_from_other(&other.tools);
            } else {
                // do nothing
            }

            self.sandbox = update_option(&self.sandbox, &other.sandbox);
            self.setup_parallel = update_option(&self.setup_parallel, &other.setup_parallel);
            self.output_format = update_option(&self.output_format, &other.output_format);
        }
        self
    }

    /// Resolve the environment variables and create key, value pairs out of them
    ///
    /// This is done especially for pass-through environment variables which have a `None` value at
    /// first.
    pub fn resolve_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, value)| {
                value.as_ref().map_or_else(
                    || std::env::var_os(key).map(|value| (key.clone(), value)),
                    |value| Some((key.clone(), value.clone())),
                )
            })
            .collect()
    }

    /// Collect all environment variables which don't have a `None` value
    ///
    /// Pass-through variables have a `None` value.
    pub fn collect_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, option)| option.as_ref().map(|value| (key.clone(), value.clone())))
            .collect()
    }
}

impl CachegrindMetric {
    /// Return true if this `EventKind` is a derived event
    ///
    /// Derived events are calculated from Cachegrind's native event types the same ways as for
    /// callgrind's [`EventKind`]
    ///
    /// * [`CachegrindMetric::L1hits`]
    /// * [`CachegrindMetric::LLhits`]
    /// * [`CachegrindMetric::RamHits`]
    /// * [`CachegrindMetric::TotalRW`]
    /// * [`CachegrindMetric::EstimatedCycles`]
    /// * [`CachegrindMetric::I1MissRate`]
    /// * [`CachegrindMetric::D1MissRate`]
    /// * [`CachegrindMetric::LLiMissRate`]
    /// * [`CachegrindMetric::LLdMissRate`]
    /// * [`CachegrindMetric::LLMissRate`]
    /// * [`CachegrindMetric::L1HitRate`]
    /// * [`CachegrindMetric::LLHitRate`]
    /// * [`CachegrindMetric::RamHitRate`]
    pub fn is_derived(&self) -> bool {
        matches!(
            self,
            Self::L1hits
                | Self::LLhits
                | Self::RamHits
                | Self::TotalRW
                | Self::EstimatedCycles
                | Self::I1MissRate
                | Self::D1MissRate
                | Self::LLiMissRate
                | Self::LLdMissRate
                | Self::LLMissRate
                | Self::L1HitRate
                | Self::LLHitRate
                | Self::RamHitRate
        )
    }

    /// Return the name of the metric which is the exact name of the enum variant
    pub fn to_name(&self) -> String {
        format!("{:?}", *self)
    }
}

impl Display for CachegrindMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            key @ (Self::Ir
            | Self::L1hits
            | Self::LLhits
            | Self::RamHits
            | Self::TotalRW
            | Self::EstimatedCycles
            | Self::I1MissRate
            | Self::D1MissRate
            | Self::LLiMissRate
            | Self::LLdMissRate
            | Self::LLMissRate
            | Self::L1HitRate
            | Self::LLHitRate
            | Self::RamHitRate) => write!(f, "{}", EventKind::from(*key)),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[cfg(feature = "runner")]
impl FromStr for CachegrindMetric {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        let metric = match lower.as_str() {
            "instructions" | "ir" => Self::Ir,
            "dr" => Self::Dr,
            "dw" => Self::Dw,
            "i1mr" => Self::I1mr,
            "ilmr" => Self::ILmr,
            "d1mr" => Self::D1mr,
            "dlmr" => Self::DLmr,
            "d1mw" => Self::D1mw,
            "dlmw" => Self::DLmw,
            "bc" => Self::Bc,
            "bcm" => Self::Bcm,
            "bi" => Self::Bi,
            "bim" => Self::Bim,
            "l1hits" => Self::L1hits,
            "llhits" => Self::LLhits,
            "ramhits" => Self::RamHits,
            "totalrw" => Self::TotalRW,
            "estimatedcycles" => Self::EstimatedCycles,
            "i1missrate" => Self::I1MissRate,
            "d1missrate" => Self::D1MissRate,
            "llimissrate" => Self::LLiMissRate,
            "lldmissrate" => Self::LLdMissRate,
            "llmissrate" => Self::LLMissRate,
            "l1hitrate" => Self::L1HitRate,
            "llhitrate" => Self::LLHitRate,
            "ramhitrate" => Self::RamHitRate,
            _ => return Err(anyhow!("Unknown cachegrind metric: '{string}'")),
        };

        Ok(metric)
    }
}

#[cfg(feature = "runner")]
impl TypeChecker for CachegrindMetric {
    fn is_int(&self) -> bool {
        match self {
            Self::Ir
            | Self::Dr
            | Self::Dw
            | Self::I1mr
            | Self::D1mr
            | Self::D1mw
            | Self::ILmr
            | Self::DLmr
            | Self::DLmw
            | Self::L1hits
            | Self::LLhits
            | Self::RamHits
            | Self::TotalRW
            | Self::EstimatedCycles
            | Self::Bc
            | Self::Bcm
            | Self::Bi
            | Self::Bim => true,
            Self::I1MissRate
            | Self::LLiMissRate
            | Self::D1MissRate
            | Self::LLdMissRate
            | Self::LLMissRate
            | Self::L1HitRate
            | Self::LLHitRate
            | Self::RamHitRate => false,
        }
    }

    fn is_float(&self) -> bool {
        !self.is_int()
    }
}

impl From<CachegrindMetric> for CachegrindMetrics {
    fn from(value: CachegrindMetric) -> Self {
        Self::SingleEvent(value)
    }
}

#[cfg(feature = "runner")]
impl FromStr for CachegrindMetrics {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        match lower.as_str().strip_prefix('@') {
            Some(suffix) => match suffix {
                "default" | "def" => Ok(Self::Default),
                "all" => Ok(Self::All),
                "cachemisses" | "misses" | "ms" => Ok(Self::CacheMisses),
                "cachemissrates" | "missrates" | "mr" => Ok(Self::CacheMissRates),
                "cachehits" | "hits" | "hs" => Ok(Self::CacheHits),
                "cachehitrates" | "hitrates" | "hr" => Ok(Self::CacheHitRates),
                "cachesim" | "cs" => Ok(Self::CacheSim),
                "branchsim" | "bs" => Ok(Self::BranchSim),
                _ => Err(anyhow!("Invalid cachegrind metric group: '{string}")),
            },
            // Use `string` instead of `lower` for the correct error message
            None => CachegrindMetric::from_str(string).map(Self::SingleEvent),
        }
    }
}

impl From<EventKind> for CallgrindMetrics {
    fn from(value: EventKind) -> Self {
        Self::SingleEvent(value)
    }
}

#[cfg(feature = "runner")]
impl FromStr for CallgrindMetrics {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        match lower.as_str().strip_prefix('@') {
            Some(suffix) => match suffix {
                "default" | "def" => Ok(Self::Default),
                "all" => Ok(Self::All),
                "cachemisses" | "misses" | "ms" => Ok(Self::CacheMisses),
                "cachemissrates" | "missrates" | "mr" => Ok(Self::CacheMissRates),
                "cachehits" | "hits" | "hs" => Ok(Self::CacheHits),
                "cachehitrates" | "hitrates" | "hr" => Ok(Self::CacheHitRates),
                "cachesim" | "cs" => Ok(Self::CacheSim),
                "cacheuse" | "cu" => Ok(Self::CacheUse),
                "systemcalls" | "syscalls" | "sc" => Ok(Self::SystemCalls),
                "branchsim" | "bs" => Ok(Self::BranchSim),
                "writebackbehaviour" | "writeback" | "wb" => Ok(Self::WriteBackBehaviour),
                _ => Err(anyhow!("Invalid event group: '{string}")),
            },
            // Keep the `string` instead of the more efficient `lower` to produce the correct error
            // message in `EventKind::from_str`
            None => EventKind::from_str(string).map(Self::SingleEvent),
        }
    }
}

impl Default for DelayKind {
    fn default() -> Self {
        Self::DurationElapse(Duration::from_secs(60))
    }
}

impl Display for DhatMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TotalUnits => f.write_str("Total units"),
            Self::TotalEvents => f.write_str("Total events"),
            Self::TotalBytes => f.write_str("Total bytes"),
            Self::TotalBlocks => f.write_str("Total blocks"),
            Self::AtTGmaxBytes => f.write_str("At t-gmax bytes"),
            Self::AtTGmaxBlocks => f.write_str("At t-gmax blocks"),
            Self::AtTEndBytes => f.write_str("At t-end bytes"),
            Self::AtTEndBlocks => f.write_str("At t-end blocks"),
            Self::ReadsBytes => f.write_str("Reads bytes"),
            Self::WritesBytes => f.write_str("Writes bytes"),
            Self::TotalLifetimes => f.write_str("Total lifetimes"),
            Self::MaximumBytes => f.write_str("Maximum bytes"),
            Self::MaximumBlocks => f.write_str("Maximum blocks"),
        }
    }
}

#[cfg(feature = "runner")]
impl FromStr for DhatMetric {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        let metric = match lower.as_str() {
            "totalunits" | "tun" => Self::TotalUnits,
            "totalevents" | "tev" => Self::TotalEvents,
            "totalbytes" | "tb" => Self::TotalBytes,
            "totalblocks" | "tbk" => Self::TotalBlocks,
            "attgmaxbytes" | "gb" => Self::AtTGmaxBytes,
            "attgmaxblocks" | "gbk" => Self::AtTGmaxBlocks,
            "attendbytes" | "eb" => Self::AtTEndBytes,
            "attendblocks" | "ebk" => Self::AtTEndBlocks,
            "readsbytes" | "rb" => Self::ReadsBytes,
            "writesbytes" | "wb" => Self::WritesBytes,
            "totallifetimes" | "tl" => Self::TotalLifetimes,
            "maximumbytes" | "mb" => Self::MaximumBytes,
            "maximumblocks" | "mbk" => Self::MaximumBlocks,
            _ => return Err(anyhow!("Unknown dhat metric: '{string}'")),
        };

        Ok(metric)
    }
}

#[cfg(feature = "runner")]
impl Summarize for DhatMetric {}

#[cfg(feature = "runner")]
impl TypeChecker for DhatMetric {
    fn is_int(&self) -> bool {
        true
    }

    fn is_float(&self) -> bool {
        false
    }
}

impl From<DhatMetric> for DhatMetrics {
    fn from(value: DhatMetric) -> Self {
        Self::SingleMetric(value)
    }
}

#[cfg(feature = "runner")]
impl FromStr for DhatMetrics {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        match lower.as_str().strip_prefix('@') {
            Some(suffix) => match suffix {
                "default" | "def" => Ok(Self::Default),
                "all" => Ok(Self::All),
                _ => Err(anyhow!("Invalid dhat metrics group: '{string}")),
            },
            // Use `string` instead of `lower` for the correct error message
            None => DhatMetric::from_str(string).map(Self::SingleMetric),
        }
    }
}

impl Default for Direction {
    fn default() -> Self {
        Self::BottomToTop
    }
}

impl<T> From<T> for EntryPoint
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self::Custom(value.into())
    }
}

impl Display for ErrorMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Errors => f.write_str("Errors"),
            Self::Contexts => f.write_str("Contexts"),
            Self::SuppressedErrors => f.write_str("Suppressed Errors"),
            Self::SuppressedContexts => f.write_str("Suppressed Contexts"),
        }
    }
}

#[cfg(feature = "runner")]
impl FromStr for ErrorMetric {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        let metric = match lower.as_str() {
            "errors" | "err" => Self::Errors,
            "contexts" | "ctx" => Self::Contexts,
            "suppressederrors" | "serr" => Self::SuppressedErrors,
            "suppressedcontexts" | "sctx" => Self::SuppressedContexts,
            _ => return Err(anyhow!("Unknown error metric: '{string}'")),
        };

        Ok(metric)
    }
}

#[cfg(feature = "runner")]
impl Summarize for ErrorMetric {}

impl EventKind {
    /// Return true if this `EventKind` is a derived event
    ///
    /// Derived events are calculated from Callgrind's native event types. See also
    /// [`crate::runner::callgrind::model::Metrics::make_summary`]. Currently all derived events
    /// are:
    ///
    /// * [`EventKind::L1hits`]
    /// * [`EventKind::LLhits`]
    /// * [`EventKind::RamHits`]
    /// * [`EventKind::TotalRW`]
    /// * [`EventKind::EstimatedCycles`]
    /// * [`EventKind::I1MissRate`]
    /// * [`EventKind::D1MissRate`]
    /// * [`EventKind::LLiMissRate`]
    /// * [`EventKind::LLdMissRate`]
    /// * [`EventKind::LLMissRate`]
    /// * [`EventKind::L1HitRate`]
    /// * [`EventKind::LLHitRate`]
    /// * [`EventKind::RamHitRate`]
    pub fn is_derived(&self) -> bool {
        matches!(
            self,
            Self::L1hits
                | Self::LLhits
                | Self::RamHits
                | Self::TotalRW
                | Self::EstimatedCycles
                | Self::I1MissRate
                | Self::D1MissRate
                | Self::LLiMissRate
                | Self::LLdMissRate
                | Self::LLMissRate
                | Self::L1HitRate
                | Self::LLHitRate
                | Self::RamHitRate
        )
    }

    /// Return the name of the metric which is the exact name of the enum variant
    pub fn to_name(&self) -> String {
        format!("{:?}", *self)
    }
}

impl Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ir => f.write_str("Instructions"),
            Self::L1hits => f.write_str("L1 Hits"),
            Self::LLhits => f.write_str("LL Hits"),
            Self::RamHits => f.write_str("RAM Hits"),
            Self::TotalRW => f.write_str("Total read+write"),
            Self::EstimatedCycles => f.write_str("Estimated Cycles"),
            Self::I1MissRate => f.write_str("I1 Miss Rate"),
            Self::D1MissRate => f.write_str("D1 Miss Rate"),
            Self::LLiMissRate => f.write_str("LLi Miss Rate"),
            Self::LLdMissRate => f.write_str("LLd Miss Rate"),
            Self::LLMissRate => f.write_str("LL Miss Rate"),
            Self::L1HitRate => f.write_str("L1 Hit Rate"),
            Self::LLHitRate => f.write_str("LL Hit Rate"),
            Self::RamHitRate => f.write_str("RAM Hit Rate"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl From<CachegrindMetric> for EventKind {
    fn from(value: CachegrindMetric) -> Self {
        match value {
            CachegrindMetric::Ir => Self::Ir,
            CachegrindMetric::Dr => Self::Dr,
            CachegrindMetric::Dw => Self::Dw,
            CachegrindMetric::I1mr => Self::I1mr,
            CachegrindMetric::D1mr => Self::D1mr,
            CachegrindMetric::D1mw => Self::D1mw,
            CachegrindMetric::ILmr => Self::ILmr,
            CachegrindMetric::DLmr => Self::DLmr,
            CachegrindMetric::DLmw => Self::DLmw,
            CachegrindMetric::L1hits => Self::L1hits,
            CachegrindMetric::LLhits => Self::LLhits,
            CachegrindMetric::RamHits => Self::RamHits,
            CachegrindMetric::TotalRW => Self::TotalRW,
            CachegrindMetric::EstimatedCycles => Self::EstimatedCycles,
            CachegrindMetric::Bc => Self::Bc,
            CachegrindMetric::Bcm => Self::Bcm,
            CachegrindMetric::Bi => Self::Bi,
            CachegrindMetric::Bim => Self::Bim,
            CachegrindMetric::I1MissRate => Self::I1MissRate,
            CachegrindMetric::D1MissRate => Self::D1MissRate,
            CachegrindMetric::LLiMissRate => Self::LLiMissRate,
            CachegrindMetric::LLdMissRate => Self::LLdMissRate,
            CachegrindMetric::LLMissRate => Self::LLMissRate,
            CachegrindMetric::L1HitRate => Self::L1HitRate,
            CachegrindMetric::LLHitRate => Self::LLHitRate,
            CachegrindMetric::RamHitRate => Self::RamHitRate,
        }
    }
}

#[cfg(feature = "runner")]
impl FromStr for EventKind {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let lower = string.to_lowercase();
        let event_kind = match lower.as_str() {
            "instructions" | "ir" => Self::Ir,
            "dr" => Self::Dr,
            "dw" => Self::Dw,
            "i1mr" => Self::I1mr,
            "d1mr" => Self::D1mr,
            "d1mw" => Self::D1mw,
            "ilmr" => Self::ILmr,
            "dlmr" => Self::DLmr,
            "dlmw" => Self::DLmw,
            "syscount" => Self::SysCount,
            "systime" => Self::SysTime,
            "syscputime" => Self::SysCpuTime,
            "ge" => Self::Ge,
            "bc" => Self::Bc,
            "bcm" => Self::Bcm,
            "bi" => Self::Bi,
            "bim" => Self::Bim,
            "ildmr" => Self::ILdmr,
            "dldmr" => Self::DLdmr,
            "dldmw" => Self::DLdmw,
            "accost1" => Self::AcCost1,
            "accost2" => Self::AcCost2,
            "sploss1" => Self::SpLoss1,
            "sploss2" => Self::SpLoss2,
            "l1hits" => Self::L1hits,
            "llhits" => Self::LLhits,
            "ramhits" => Self::RamHits,
            "totalrw" => Self::TotalRW,
            "estimatedcycles" => Self::EstimatedCycles,
            "i1missrate" => Self::I1MissRate,
            "d1missrate" => Self::D1MissRate,
            "llimissrate" => Self::LLiMissRate,
            "lldmissrate" => Self::LLdMissRate,
            "llmissrate" => Self::LLMissRate,
            "l1hitrate" => Self::L1HitRate,
            "llhitrate" => Self::LLHitRate,
            "ramhitrate" => Self::RamHitRate,
            _ => return Err(anyhow!("Unknown event kind: '{string}'")),
        };

        Ok(event_kind)
    }
}

#[cfg(feature = "runner")]
impl TypeChecker for EventKind {
    fn is_int(&self) -> bool {
        match self {
            Self::Ir
            | Self::Dr
            | Self::Dw
            | Self::I1mr
            | Self::D1mr
            | Self::D1mw
            | Self::ILmr
            | Self::DLmr
            | Self::DLmw
            | Self::L1hits
            | Self::LLhits
            | Self::RamHits
            | Self::TotalRW
            | Self::EstimatedCycles
            | Self::SysCount
            | Self::SysTime
            | Self::SysCpuTime
            | Self::Ge
            | Self::Bc
            | Self::Bcm
            | Self::Bi
            | Self::Bim
            | Self::ILdmr
            | Self::DLdmr
            | Self::DLdmw
            | Self::AcCost1
            | Self::AcCost2
            | Self::SpLoss1
            | Self::SpLoss2 => true,
            Self::I1MissRate
            | Self::LLiMissRate
            | Self::D1MissRate
            | Self::LLdMissRate
            | Self::LLMissRate
            | Self::L1HitRate
            | Self::LLHitRate
            | Self::RamHitRate => false,
        }
    }

    fn is_float(&self) -> bool {
        !self.is_int()
    }
}

#[cfg(feature = "runner")]
impl From<CachegrindMetrics> for IndexSet<CachegrindMetric> {
    fn from(value: CachegrindMetrics) -> Self {
        let mut metrics = Self::new();
        match value {
            CachegrindMetrics::None => {}
            CachegrindMetrics::All => metrics.extend(CachegrindMetric::iter()),
            CachegrindMetrics::Default => {
                metrics.insert(CachegrindMetric::Ir);
                metrics.extend(Self::from(CachegrindMetrics::CacheHits));
                metrics.extend([CachegrindMetric::TotalRW, CachegrindMetric::EstimatedCycles]);
                metrics.extend(Self::from(CachegrindMetrics::BranchSim));
            }
            CachegrindMetrics::CacheMisses => metrics.extend([
                CachegrindMetric::I1mr,
                CachegrindMetric::D1mr,
                CachegrindMetric::D1mw,
                CachegrindMetric::ILmr,
                CachegrindMetric::DLmr,
                CachegrindMetric::DLmw,
            ]),
            CachegrindMetrics::CacheMissRates => metrics.extend([
                CachegrindMetric::I1MissRate,
                CachegrindMetric::LLiMissRate,
                CachegrindMetric::D1MissRate,
                CachegrindMetric::LLdMissRate,
                CachegrindMetric::LLMissRate,
            ]),
            CachegrindMetrics::CacheHits => {
                metrics.extend([
                    CachegrindMetric::L1hits,
                    CachegrindMetric::LLhits,
                    CachegrindMetric::RamHits,
                ]);
            }
            CachegrindMetrics::CacheHitRates => {
                metrics.extend([
                    CachegrindMetric::L1HitRate,
                    CachegrindMetric::LLHitRate,
                    CachegrindMetric::RamHitRate,
                ]);
            }
            CachegrindMetrics::CacheSim => {
                metrics.extend([CachegrindMetric::Dr, CachegrindMetric::Dw]);
                metrics.extend(Self::from(CachegrindMetrics::CacheMisses));
                metrics.extend(Self::from(CachegrindMetrics::CacheMissRates));
                metrics.extend(Self::from(CachegrindMetrics::CacheHits));
                metrics.extend(Self::from(CachegrindMetrics::CacheHitRates));
                metrics.insert(CachegrindMetric::TotalRW);
                metrics.insert(CachegrindMetric::EstimatedCycles);
            }
            CachegrindMetrics::BranchSim => {
                metrics.extend([
                    CachegrindMetric::Bc,
                    CachegrindMetric::Bcm,
                    CachegrindMetric::Bi,
                    CachegrindMetric::Bim,
                ]);
            }
            CachegrindMetrics::SingleEvent(metric) => {
                metrics.insert(metric);
            }
        }

        metrics
    }
}

#[cfg(feature = "runner")]
impl From<DhatMetrics> for IndexSet<DhatMetric> {
    fn from(value: DhatMetrics) -> Self {
        use DhatMetric::*;
        match value {
            DhatMetrics::All => DhatMetric::iter().collect(),
            DhatMetrics::Default => indexset! {
            TotalUnits,
            TotalEvents,
            TotalBytes,
            TotalBlocks,
            AtTGmaxBytes,
            AtTGmaxBlocks,
            AtTEndBytes,
            AtTEndBlocks,
            ReadsBytes,
            WritesBytes },
            DhatMetrics::SingleMetric(dhat_metric) => indexset! { dhat_metric },
        }
    }
}

#[cfg(feature = "runner")]
impl From<CallgrindMetrics> for IndexSet<EventKind> {
    fn from(value: CallgrindMetrics) -> Self {
        let mut event_kinds = Self::new();
        match value {
            CallgrindMetrics::None => {}
            CallgrindMetrics::All => event_kinds.extend(EventKind::iter()),
            CallgrindMetrics::Default => {
                event_kinds.insert(EventKind::Ir);
                event_kinds.extend(Self::from(CallgrindMetrics::CacheHits));
                event_kinds.extend([EventKind::TotalRW, EventKind::EstimatedCycles]);
                event_kinds.extend(Self::from(CallgrindMetrics::SystemCalls));
                event_kinds.insert(EventKind::Ge);
                event_kinds.extend(Self::from(CallgrindMetrics::BranchSim));
                event_kinds.extend(Self::from(CallgrindMetrics::WriteBackBehaviour));
                event_kinds.extend(Self::from(CallgrindMetrics::CacheUse));
            }
            CallgrindMetrics::CacheMisses => event_kinds.extend([
                EventKind::I1mr,
                EventKind::D1mr,
                EventKind::D1mw,
                EventKind::ILmr,
                EventKind::DLmr,
                EventKind::DLmw,
            ]),
            CallgrindMetrics::CacheMissRates => event_kinds.extend([
                EventKind::I1MissRate,
                EventKind::LLiMissRate,
                EventKind::D1MissRate,
                EventKind::LLdMissRate,
                EventKind::LLMissRate,
            ]),
            CallgrindMetrics::CacheHits => {
                event_kinds.extend([EventKind::L1hits, EventKind::LLhits, EventKind::RamHits]);
            }
            CallgrindMetrics::CacheHitRates => {
                event_kinds.extend([
                    EventKind::L1HitRate,
                    EventKind::LLHitRate,
                    EventKind::RamHitRate,
                ]);
            }
            CallgrindMetrics::CacheSim => {
                event_kinds.extend([EventKind::Dr, EventKind::Dw]);
                event_kinds.extend(Self::from(CallgrindMetrics::CacheMisses));
                event_kinds.extend(Self::from(CallgrindMetrics::CacheMissRates));
                event_kinds.extend(Self::from(CallgrindMetrics::CacheHits));
                event_kinds.extend(Self::from(CallgrindMetrics::CacheHitRates));
                event_kinds.insert(EventKind::TotalRW);
                event_kinds.insert(EventKind::EstimatedCycles);
            }
            CallgrindMetrics::CacheUse => event_kinds.extend([
                EventKind::AcCost1,
                EventKind::AcCost2,
                EventKind::SpLoss1,
                EventKind::SpLoss2,
            ]),
            CallgrindMetrics::SystemCalls => {
                event_kinds.extend([
                    EventKind::SysCount,
                    EventKind::SysTime,
                    EventKind::SysCpuTime,
                ]);
            }
            CallgrindMetrics::BranchSim => {
                event_kinds.extend([EventKind::Bc, EventKind::Bcm, EventKind::Bi, EventKind::Bim]);
            }
            CallgrindMetrics::WriteBackBehaviour => {
                event_kinds.extend([EventKind::ILdmr, EventKind::DLdmr, EventKind::DLdmw]);
            }
            CallgrindMetrics::SingleEvent(event_kind) => {
                event_kinds.insert(event_kind);
            }
        }

        event_kinds
    }
}

impl LibraryBenchmarkConfig {
    /// Update this configuration with all other configurations in the given order
    #[must_use]
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.default_tool = update_option(&self.default_tool, &other.default_tool);
            self.env_clear = update_option(&self.env_clear, &other.env_clear);

            self.valgrind_args
                .extend_ignore_flag(other.valgrind_args.0.iter());

            self.envs.extend_from_slice(&other.envs);
            if let Some(other_tools) = &other.tools_override {
                self.tools = other_tools.clone();
            } else if !other.tools.is_empty() {
                self.tools.update_from_other(&other.tools);
            } else {
                // do nothing
            }

            self.output_format = update_option(&self.output_format, &other.output_format);
        }
        self
    }

    /// Resolve the environment variables and create key, value pairs out of them
    ///
    /// Same as [`BinaryBenchmarkConfig::resolve_envs`]
    pub fn resolve_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, value)| match value {
                Some(value) => Some((key.clone(), value.clone())),
                None => std::env::var_os(key).map(|value| (key.clone(), value)),
            })
            .collect()
    }

    /// Collect all environment variables which don't have a `None` value
    ///
    /// Same as [`BinaryBenchmarkConfig::collect_envs`]
    pub fn collect_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, option)| option.as_ref().map(|value| (key.clone(), value.clone())))
            .collect()
    }
}

#[cfg(feature = "runner")]
impl From<runner::metrics::Metric> for Limit {
    fn from(value: runner::metrics::Metric) -> Self {
        match value {
            runner::metrics::Metric::Int(a) => Self::Int(a),
            runner::metrics::Metric::Float(b) => Self::Float(b),
        }
    }
}

impl From<f64> for Limit {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<u64> for Limit {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}

impl RawArgs {
    /// Create new arguments for a valgrind tool
    pub fn new<I, T>(args: T) -> Self
    where
        I: Into<String>,
        T: IntoIterator<Item = I>,
    {
        Self(args.into_iter().map(Into::into).collect())
    }

    /// Extend the arguments with the contents of an iterator
    pub fn extend_ignore_flag<I, T>(&mut self, args: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.extend(
            args.into_iter()
                .filter(|s| !s.as_ref().is_empty())
                .map(|s| {
                    let string = s.as_ref();
                    if string.starts_with('-') {
                        string.to_owned()
                    } else {
                        format!("--{string}")
                    }
                }),
        );
    }

    /// Return true if there are no tool arguments
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Append the arguments of another `RawArgs`
    pub fn update(&mut self, other: &Self) {
        self.extend_ignore_flag(other.0.iter());
    }

    /// Prepend the arguments of another `RawArgs`
    pub fn prepend(&mut self, other: &Self) {
        if !other.is_empty() {
            let mut other = other.clone();
            other.update(self);
            *self = other;
        }
    }
}

impl<I> FromIterator<I> for RawArgs
where
    I: AsRef<str>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend_ignore_flag(iter);
        this
    }
}

impl Stdin {
    #[cfg(feature = "runner")]
    pub(crate) fn apply(
        &self,
        command: &mut StdCommand,
        stream: Stream,
        child: Option<&mut Child>,
    ) -> Result<(), String> {
        match (self, child) {
            (Self::Setup(Pipe::Stdout), Some(child)) => {
                command.stdin(
                    child
                        .stdout
                        .take()
                        .ok_or_else(|| "Error piping setup stdout".to_owned())?,
                );
                Ok(())
            }
            (Self::Setup(Pipe::Stderr), Some(child)) => {
                command.stdin(
                    child
                        .stderr
                        .take()
                        .ok_or_else(|| "Error piping setup stderr".to_owned())?,
                );
                Ok(())
            }
            (Self::Setup(_) | Self::Pipe, _) => Stdio::Pipe.apply(command, stream),
            (Self::Inherit, _) => Stdio::Inherit.apply(command, stream),
            (Self::Null, _) => Stdio::Null.apply(command, stream),
            (Self::File(path), _) => Stdio::File(path.clone()).apply(command, stream),
        }
    }
}

impl From<Stdio> for Stdin {
    fn from(value: Stdio) -> Self {
        match value {
            Stdio::Inherit => Self::Inherit,
            Stdio::Null => Self::Null,
            Stdio::File(file) => Self::File(file),
            Stdio::Pipe => Self::Pipe,
        }
    }
}

impl From<PathBuf> for Stdin {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl From<&PathBuf> for Stdin {
    fn from(value: &PathBuf) -> Self {
        Self::File(value.to_owned())
    }
}

impl From<&Path> for Stdin {
    fn from(value: &Path) -> Self {
        Self::File(value.to_path_buf())
    }
}

impl Stdio {
    #[cfg(feature = "runner")]
    pub(crate) fn apply(&self, command: &mut StdCommand, stream: Stream) -> Result<(), String> {
        let stdio = match self {
            Self::Pipe => StdStdio::piped(),
            Self::Inherit => StdStdio::inherit(),
            Self::Null => StdStdio::null(),
            Self::File(path) => match stream {
                Stream::Stdin => StdStdio::from(File::open(path).map_err(|error| {
                    format!(
                        "Failed to open file '{}' in read mode for {stream}: {error}",
                        path.display()
                    )
                })?),
                Stream::Stdout | Stream::Stderr => {
                    StdStdio::from(File::create(path).map_err(|error| {
                        format!(
                            "Failed to create file '{}' for {stream}: {error}",
                            path.display()
                        )
                    })?)
                }
            },
        };

        match stream {
            Stream::Stdin => command.stdin(stdio),
            Stream::Stdout => command.stdout(stdio),
            Stream::Stderr => command.stderr(stdio),
        };

        Ok(())
    }

    #[cfg(feature = "runner")]
    pub(crate) fn is_pipe(&self) -> bool {
        match self {
            Self::Inherit => false,
            Self::Null | Self::File(_) | Self::Pipe => true,
        }
    }
}

impl From<PathBuf> for Stdio {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl From<&PathBuf> for Stdio {
    fn from(value: &PathBuf) -> Self {
        Self::File(value.to_owned())
    }
}

impl From<&Path> for Stdio {
    fn from(value: &Path) -> Self {
        Self::File(value.to_path_buf())
    }
}

#[cfg(feature = "runner")]
impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}").to_lowercase())
    }
}

impl Tool {
    /// Create a new `Tool` configuration
    pub fn new(kind: ValgrindTool) -> Self {
        Self {
            kind,
            enable: None,
            raw_args: RawArgs::default(),
            show_log: None,
            regression_config: None,
            flamegraph_config: None,
            output_format: None,
            entry_point: None,
            frames: None,
        }
    }

    /// Create a new `Tool` configuration with the given command-line `args`
    pub fn with_args<I, T>(kind: ValgrindTool, args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        let mut this = Self::new(kind);
        this.raw_args = RawArgs::from_iter(args);
        this
    }

    /// Update this tool configuration with another configuration
    pub fn update(&mut self, other: &Self) {
        if self.kind == other.kind {
            self.enable = update_option(&self.enable, &other.enable);
            self.show_log = update_option(&self.show_log, &other.show_log);
            self.regression_config =
                update_option(&self.regression_config, &other.regression_config);
            self.flamegraph_config =
                update_option(&self.flamegraph_config, &other.flamegraph_config);
            self.output_format = update_option(&self.output_format, &other.output_format);
            self.entry_point = update_option(&self.entry_point, &other.entry_point);
            self.frames = update_option(&self.frames, &other.frames);

            self.raw_args.extend_ignore_flag(other.raw_args.0.iter());
        }
    }
}

impl Tools {
    /// Return true if `Tools` is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Update `Tools`
    pub fn update(&mut self, other: Tool) {
        if let Some(tool) = self.0.iter_mut().find(|t| t.kind == other.kind) {
            tool.update(&other);
        } else {
            self.0.push(other);
        }
    }

    /// Update `Tools` with all [`Tool`]s from an iterator
    pub fn update_all<T>(&mut self, tools: T)
    where
        T: IntoIterator<Item = Tool>,
    {
        for tool in tools {
            self.update(tool);
        }
    }

    /// Update `Tools` with another `Tools`
    pub fn update_from_other(&mut self, tools: &Self) {
        self.update_all(tools.0.iter().cloned());
    }

    /// Search for the [`Tool`] with `kind` and if present remove it from this `Tools` and return it
    pub fn consume(&mut self, kind: ValgrindTool) -> Option<Tool> {
        self.0
            .iter()
            .position(|p| p.kind == kind)
            .map(|position| self.0.remove(position))
    }
}

impl ValgrindTool {
    /// Return the id used by the `valgrind --tool` option
    pub fn id(&self) -> String {
        match self {
            Self::DHAT => "dhat".to_owned(),
            Self::Callgrind => "callgrind".to_owned(),
            Self::Memcheck => "memcheck".to_owned(),
            Self::Helgrind => "helgrind".to_owned(),
            Self::DRD => "drd".to_owned(),
            Self::Massif => "massif".to_owned(),
            Self::BBV => "exp-bbv".to_owned(),
            Self::Cachegrind => "cachegrind".to_owned(),
        }
    }

    /// Return true if this tool has output files in addition to log files
    pub fn has_output_file(&self) -> bool {
        matches!(
            self,
            Self::Callgrind | Self::DHAT | Self::BBV | Self::Massif | Self::Cachegrind
        )
    }

    /// Return true if this tool supports xtree memory files
    pub fn has_xtree_file(&self) -> bool {
        matches!(self, Self::Memcheck | Self::Massif | Self::Helgrind)
    }

    /// Return true if this tool supports xleak files
    pub fn has_xleak_file(&self) -> bool {
        *self == Self::Memcheck
    }
}

impl Display for ValgrindTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.id())
    }
}

#[cfg(feature = "runner")]
impl FromStr for ValgrindTool {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_lowercase().as_str())
    }
}

#[cfg(feature = "runner")]
impl TryFrom<&str> for ValgrindTool {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "callgrind" => Ok(Self::Callgrind),
            "cachegrind" => Ok(Self::Cachegrind),
            "dhat" => Ok(Self::DHAT),
            "memcheck" => Ok(Self::Memcheck),
            "helgrind" => Ok(Self::Helgrind),
            "drd" => Ok(Self::DRD),
            "massif" => Ok(Self::Massif),
            "exp-bbv" => Ok(Self::BBV),
            v => Err(anyhow!("Unknown tool '{}'", v)),
        }
    }
}

/// Update the value of an [`Option`]
pub fn update_option<T: Clone>(first: &Option<T>, other: &Option<T>) -> Option<T> {
    other.clone().or_else(|| first.clone())
}

#[cfg(test)]
mod tests {
    use indexmap::indexset;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::EventKind::*;
    use super::{CachegrindMetric as Cm, *};

    #[test]
    fn test_cachegrind_metric_from_str_ignore_case() {
        for metric in CachegrindMetric::iter() {
            let string = format!("{metric:?}");
            let actual = CachegrindMetric::from_str(&string);
            assert_eq!(actual.unwrap(), metric);
        }
    }

    #[test]
    fn test_event_kind_from_str_ignore_case() {
        for event_kind in EventKind::iter() {
            let string = format!("{event_kind:?}");
            let actual = EventKind::from_str(&string);
            assert_eq!(actual.unwrap(), event_kind);
        }
    }

    #[test]
    fn test_library_benchmark_config_update_from_all_when_default() {
        assert_eq!(
            LibraryBenchmarkConfig::default()
                .update_from_all([Some(&LibraryBenchmarkConfig::default())]),
            LibraryBenchmarkConfig::default()
        );
    }

    #[test]
    fn test_library_benchmark_config_update_from_all_when_no_tools_override() {
        let base = LibraryBenchmarkConfig::default();
        let other = LibraryBenchmarkConfig {
            env_clear: Some(true),
            valgrind_args: RawArgs(vec!["--valgrind-arg=yes".to_owned()]),
            envs: vec![(OsString::from("MY_ENV"), Some(OsString::from("value")))],
            tools: Tools(vec![Tool {
                kind: ValgrindTool::DHAT,
                enable: None,
                raw_args: RawArgs(vec![]),
                show_log: None,
                regression_config: Some(ToolRegressionConfig::Callgrind(
                    CallgrindRegressionConfig::default(),
                )),
                flamegraph_config: Some(ToolFlamegraphConfig::Callgrind(
                    FlamegraphConfig::default(),
                )),
                entry_point: Some(EntryPoint::default()),
                output_format: Some(ToolOutputFormat::None),
                frames: Some(vec!["some::frame".to_owned()]),
            }]),
            tools_override: None,
            output_format: None,
            default_tool: Some(ValgrindTool::BBV),
        };

        assert_eq!(base.update_from_all([Some(&other.clone())]), other);
    }

    #[test]
    fn test_library_benchmark_config_update_from_all_when_tools_override() {
        let base = LibraryBenchmarkConfig::default();
        let other = LibraryBenchmarkConfig {
            env_clear: Some(true),
            valgrind_args: RawArgs(vec!["--valgrind-arg=yes".to_owned()]),
            envs: vec![(OsString::from("MY_ENV"), Some(OsString::from("value")))],
            tools: Tools(vec![Tool {
                kind: ValgrindTool::DHAT,
                enable: None,
                raw_args: RawArgs(vec![]),
                show_log: None,
                regression_config: Some(ToolRegressionConfig::Callgrind(
                    CallgrindRegressionConfig::default(),
                )),
                flamegraph_config: Some(ToolFlamegraphConfig::Callgrind(
                    FlamegraphConfig::default(),
                )),
                entry_point: Some(EntryPoint::default()),
                output_format: Some(ToolOutputFormat::None),
                frames: Some(vec!["some::frame".to_owned()]),
            }]),
            tools_override: Some(Tools(vec![])),
            output_format: Some(OutputFormat::default()),
            default_tool: Some(ValgrindTool::BBV),
        };
        let expected = LibraryBenchmarkConfig {
            tools: other.tools_override.as_ref().unwrap().clone(),
            tools_override: None,
            ..other.clone()
        };

        assert_eq!(base.update_from_all([Some(&other)]), expected);
    }

    #[rstest]
    #[case::env_clear(
        LibraryBenchmarkConfig {
            env_clear: Some(true),
            ..Default::default()
        }
    )]
    fn test_library_benchmark_config_update_from_all_truncate_description(
        #[case] config: LibraryBenchmarkConfig,
    ) {
        let actual = LibraryBenchmarkConfig::default().update_from_all([Some(&config)]);
        assert_eq!(actual, config);
    }

    #[rstest]
    #[case::all_none(None, None, None)]
    #[case::some_and_none(Some(true), None, Some(true))]
    #[case::none_and_some(None, Some(true), Some(true))]
    #[case::some_and_some(Some(false), Some(true), Some(true))]
    #[case::some_and_some_value_does_not_matter(Some(true), Some(false), Some(false))]
    fn test_update_option(
        #[case] first: Option<bool>,
        #[case] other: Option<bool>,
        #[case] expected: Option<bool>,
    ) {
        assert_eq!(update_option(&first, &other), expected);
    }

    #[rstest]
    #[case::empty(vec![], &[], vec![])]
    #[case::empty_base(vec![], &["--a=yes"], vec!["--a=yes"])]
    #[case::no_flags(vec![], &["a=yes"], vec!["--a=yes"])]
    #[case::already_exists_single(vec!["--a=yes"], &["--a=yes"], vec!["--a=yes","--a=yes"])]
    #[case::already_exists_when_multiple(vec!["--a=yes", "--b=yes"], &["--a=yes"], vec!["--a=yes", "--b=yes", "--a=yes"])]
    fn test_raw_args_extend_ignore_flags(
        #[case] base: Vec<&str>,
        #[case] data: &[&str],
        #[case] expected: Vec<&str>,
    ) {
        let mut base = RawArgs(base.iter().map(std::string::ToString::to_string).collect());
        base.extend_ignore_flag(data.iter().map(std::string::ToString::to_string));

        assert_eq!(base.0.into_iter().collect::<Vec<String>>(), expected);
    }

    #[rstest]
    #[case::none(CallgrindMetrics::None, indexset![])]
    #[case::all(CallgrindMetrics::All, indexset![Ir, Dr, Dw, I1mr, D1mr, D1mw, ILmr, DLmr,
        DLmw, I1MissRate, LLiMissRate, D1MissRate, LLdMissRate, LLMissRate, L1hits, LLhits, RamHits,
        TotalRW, L1HitRate, LLHitRate, RamHitRate, EstimatedCycles, SysCount, SysTime, SysCpuTime,
        Ge, Bc, Bcm, Bi, Bim, ILdmr, DLdmr, DLdmw, AcCost1, AcCost2, SpLoss1, SpLoss2]
    )]
    #[case::default(CallgrindMetrics::Default, indexset![Ir, L1hits, LLhits, RamHits, TotalRW,
        EstimatedCycles, SysCount, SysTime, SysCpuTime, Ge, Bc,
        Bcm, Bi, Bim, ILdmr, DLdmr, DLdmw, AcCost1, AcCost2, SpLoss1, SpLoss2]
    )]
    #[case::cache_misses(CallgrindMetrics::CacheMisses, indexset![I1mr, D1mr, D1mw, ILmr,
        DLmr, DLmw]
    )]
    #[case::cache_miss_rates(CallgrindMetrics::CacheMissRates, indexset![I1MissRate,
        D1MissRate, LLMissRate, LLiMissRate, LLdMissRate]
    )]
    #[case::cache_hits(CallgrindMetrics::CacheHits, indexset![L1hits, LLhits, RamHits])]
    #[case::cache_hit_rates(CallgrindMetrics::CacheHitRates, indexset![
        L1HitRate, LLHitRate, RamHitRate
    ])]
    #[case::cache_sim(CallgrindMetrics::CacheSim, indexset![Dr, Dw, I1mr, D1mr, D1mw, ILmr, DLmr,
        DLmw, I1MissRate, LLiMissRate, D1MissRate, LLdMissRate, LLMissRate, L1hits, LLhits, RamHits,
        TotalRW, L1HitRate, LLHitRate, RamHitRate, EstimatedCycles]
    )]
    #[case::cache_use(CallgrindMetrics::CacheUse, indexset![AcCost1, AcCost2, SpLoss1, SpLoss2])]
    #[case::system_calls(CallgrindMetrics::SystemCalls, indexset![SysCount, SysTime, SysCpuTime])]
    #[case::branch_sim(CallgrindMetrics::BranchSim, indexset![Bc, Bcm, Bi, Bim])]
    #[case::write_back(CallgrindMetrics::WriteBackBehaviour, indexset![ILdmr, DLdmr, DLdmw])]
    #[case::single_event(CallgrindMetrics::SingleEvent(Ir), indexset![Ir])]
    fn test_callgrind_metrics_into_index_set(
        #[case] callgrind_metrics: CallgrindMetrics,
        #[case] expected_metrics: IndexSet<EventKind>,
    ) {
        assert_eq!(IndexSet::from(callgrind_metrics), expected_metrics);
    }

    #[rstest]
    #[case::none(CachegrindMetrics::None, indexset![])]
    #[case::all(CachegrindMetrics::All, indexset![Cm::Ir, Cm::Dr, Cm::Dw, Cm::I1mr, Cm::D1mr,
        Cm::D1mw, Cm::ILmr, Cm::DLmr, Cm::DLmw, Cm::I1MissRate, Cm::LLiMissRate, Cm::D1MissRate,
        Cm::LLdMissRate, Cm::LLMissRate, Cm::L1hits, Cm::LLhits, Cm::RamHits, Cm::TotalRW,
        Cm::L1HitRate, Cm::LLHitRate, Cm::RamHitRate, Cm::EstimatedCycles, Cm::Bc, Cm::Bcm, Cm::Bi,
        Cm::Bim,
    ])]
    #[case::default(CachegrindMetrics::Default, indexset![Cm::Ir, Cm::L1hits, Cm::LLhits,
        Cm::RamHits, Cm::TotalRW, Cm::EstimatedCycles, Cm::Bc, Cm::Bcm, Cm::Bi, Cm::Bim
    ])]
    #[case::cache_misses(CachegrindMetrics::CacheMisses, indexset![Cm::I1mr, Cm::D1mr, Cm::D1mw,
        Cm::ILmr, Cm::DLmr, Cm::DLmw
    ])]
    #[case::cache_miss_rates(CachegrindMetrics::CacheMissRates, indexset![Cm::I1MissRate,
        Cm::D1MissRate, Cm::LLMissRate, Cm::LLiMissRate, Cm::LLdMissRate
    ])]
    #[case::cache_hits(CachegrindMetrics::CacheHits, indexset![
        Cm::L1hits, Cm::LLhits, Cm::RamHits
    ])]
    #[case::cache_hit_rates(CachegrindMetrics::CacheHitRates, indexset![
        Cm::L1HitRate, Cm::LLHitRate, Cm::RamHitRate
    ])]
    #[case::cache_sim(CachegrindMetrics::CacheSim, indexset![Cm::Dr, Cm::Dw, Cm::I1mr, Cm::D1mr,
        Cm::D1mw, Cm::ILmr, Cm::DLmr, Cm::DLmw, Cm::I1MissRate, Cm::LLiMissRate, Cm::D1MissRate,
        Cm::LLdMissRate, Cm::LLMissRate, Cm::L1hits, Cm::LLhits, Cm::RamHits, Cm::TotalRW,
        Cm::L1HitRate, Cm::LLHitRate, Cm::RamHitRate, Cm::EstimatedCycles
    ])]
    #[case::branch_sim(CachegrindMetrics::BranchSim, indexset![
        Cm::Bc, Cm::Bcm, Cm::Bi, Cm::Bim
    ])]
    #[case::single_event(CachegrindMetrics::SingleEvent(Cm::Ir), indexset![Cm::Ir])]
    fn test_cachegrind_metrics_into_index_set(
        #[case] cachegrind_metrics: CachegrindMetrics,
        #[case] expected_metrics: IndexSet<CachegrindMetric>,
    ) {
        assert_eq!(IndexSet::from(cachegrind_metrics), expected_metrics);
    }

    #[rstest]
    #[case::empty(&[], &[], &[])]
    #[case::prepend_empty(&["--some"], &[], &["--some"])]
    #[case::initial_empty(&[], &["--some"], &["--some"])]
    #[case::both_same_arg(&["--some"], &["--some"], &["--some", "--some"])]
    #[case::both_different_arg(&["--some"], &["--other"], &["--other", "--some"])]
    fn test_raw_args_prepend(
        #[case] raw_args: &[&str],
        #[case] other: &[&str],
        #[case] expected: &[&str],
    ) {
        let mut raw_args = RawArgs::new(raw_args.iter().map(ToOwned::to_owned));
        let other = RawArgs::new(other.iter().map(ToOwned::to_owned));
        let expected = RawArgs::new(expected.iter().map(ToOwned::to_owned));

        raw_args.prepend(&other);
        assert_eq!(raw_args, expected);
    }

    #[test]
    fn test_tool_update_when_tools_match() {
        let mut base = Tool::new(ValgrindTool::Callgrind);
        let other = Tool {
            kind: ValgrindTool::Callgrind,
            enable: Some(true),
            raw_args: RawArgs::new(["--some"]),
            show_log: Some(false),
            regression_config: Some(ToolRegressionConfig::None),
            flamegraph_config: Some(ToolFlamegraphConfig::None),
            output_format: Some(ToolOutputFormat::None),
            entry_point: Some(EntryPoint::Default),
            frames: Some(vec!["some::frame".to_owned()]),
        };
        let expected = other.clone();
        base.update(&other);
        assert_eq!(base, expected);
    }

    #[test]
    fn test_tool_update_when_tools_not_match() {
        let mut base = Tool::new(ValgrindTool::Callgrind);
        let other = Tool {
            kind: ValgrindTool::DRD,
            enable: Some(true),
            raw_args: RawArgs::new(["--some"]),
            show_log: Some(false),
            regression_config: Some(ToolRegressionConfig::None),
            flamegraph_config: Some(ToolFlamegraphConfig::None),
            output_format: Some(ToolOutputFormat::None),
            entry_point: Some(EntryPoint::Default),
            frames: Some(vec!["some::frame".to_owned()]),
        };

        let expected = base.clone();
        base.update(&other);

        assert_eq!(base, expected);
    }
}
