//! # `config` — workspace-wide configuration (per `dol-rewrite-plan-v2.md` §6.1).
//!
//! Everything else in DOL is parameterised by [`Config`]. This module
//! groups the four sub-configs:
//!
//! | sub-config       | governs                                          |
//! |------------------|--------------------------------------------------|
//! | [`BudgetConfig`] | per-traversal caps (depth / nodes / bytes)       |
//! | [`PoolConfig`]   | sharded arena geometry (shard count / seed)      |
//! | [`HashConfig`]   | hash strategy (xxHash3 / BLAKE3-128 / BLAKE3-256)|
//! | [`Profile`]      | deployment profile tag (Standard / Embedded / IotMin) |
//!
//! Three presets cover the common deployment shapes:
//!
//! | preset                  | profile     | notes                            |
//! |-------------------------|-------------|----------------------------------|
//! | [`Config::standard`]    | `Standard`  | host defaults — generous caps    |
//! | [`Config::embedded`]    | `Embedded`  | `no_std + alloc`; moderate caps  |
//! | [`Config::iot_min`]     | `IotMin`    | bare-metal, tight caps           |
//!
//! All types are `Copy + Eq + Debug` and `no_std + alloc`-clean.

/// Deployment profile tag. Used by downstream crates (e.g. `dol-cas`
/// `Pool`/`StaticStringPool` selection in M2) to pick implementations
/// without sprinkling `cfg` macros across call sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Profile {
    /// Host (`std`) deployment: desktop, server, container.
    Standard,
    /// `no_std + alloc` deployment with a real allocator
    /// (`heapless::Vec`-style ringbuffers possible but not required).
    Embedded,
    /// Bare-metal `no_std` deployment without an allocator. Pools must
    /// be backed by `static mut` storage (`StaticStringPool` etc.).
    IotMin,
}

/// Per-traversal caps. A fresh [`crate::budget::Budget`] is seeded from
/// this struct at the entry point of every decode / lower / walk path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BudgetConfig {
    /// Maximum nesting depth (e.g. parser recursion, IR tree height).
    /// Default: `512`.
    pub max_depth: u32,
    /// Maximum number of distinct nodes (AST, IR, wire frames) that
    /// may be materialised in a single traversal. Default: `1_000_000`.
    pub max_nodes: u32,
    /// Maximum cumulative bytes a traversal is allowed to allocate or
    /// deserialise. Default: `64 MiB`.
    pub max_bytes: u64,
}

impl BudgetConfig {
    /// Host preset — defaults documented above. Returns `512`/`1_000_000`/`64 MiB`.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            max_depth: 512,
            max_nodes: 1_000_000,
            max_bytes: 64 * 1024 * 1024,
        }
    }

    /// `no_std + alloc` preset. Tighter than host; allows generous
    /// caps for tabletop / single-board-computer deployments.
    #[must_use]
    pub const fn embedded() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 65_536,
            max_bytes: 4 * 1024 * 1024,
        }
    }

    /// Bare-metal preset sized for `thumbv7em` / `riscv32imac`-class
    /// devices. Designed to fit a single traversal in a few KiB.
    #[must_use]
    pub const fn iot_min() -> Self {
        Self {
            max_depth: 16,
            max_nodes: 1_024,
            max_bytes: 16 * 1024,
        }
    }
}

/// Sharded arena geometry. The DoS-protection `hash_seed` defends
/// against algorithmic-complexity attacks on the pool's hash buckets
/// (only relevant when the [`HashConfig`] is [`HashStrategy::Fast`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PoolConfig {
    /// Number of shards per pool. Must be a power of two so the
    /// fast-modulo (`hash & (shard_count - 1)`) is correct.
    /// Default: `64`.
    pub shard_count: u32,
    /// DoS-protection seed for xxHash3 bucketing. Picked once per
    /// process; embedded targets may hard-code zero.
    pub hash_seed: u64,
    /// Bump pre-allocation per shard, in bytes. Default: `4096`.
    pub initial_bytes: u32,
}

impl PoolConfig {
    /// Host preset — `shard_count = 64`, `initial_bytes = 4 KiB`,
    /// `hash_seed = 0` (callers should override with a per-process
    /// random value for adversarial workloads).
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            shard_count: 64,
            hash_seed: 0,
            initial_bytes: 4096,
        }
    }

    /// `no_std + alloc` preset — smaller shard table to keep per-pool
    /// footprint within a few KiB.
    #[must_use]
    pub const fn embedded() -> Self {
        Self {
            shard_count: 8,
            hash_seed: 0,
            initial_bytes: 1024,
        }
    }

    /// Bare-metal preset — single shard, no dynamic growth. Backing
    /// storage must be supplied as `&'static mut [u8]` by the caller.
    #[must_use]
    pub const fn iot_min() -> Self {
        Self {
            shard_count: 1,
            hash_seed: 0,
            initial_bytes: 0,
        }
    }

    /// Returns `true` when [`Self::shard_count`] is a non-zero power of two.
    /// Pools assume this invariant when computing
    /// `hash & (shard_count - 1)`.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.shard_count != 0 && self.shard_count.is_power_of_two()
    }
}

/// Hash strategy used by [`crate::hash`] consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum HashStrategy {
    /// xxHash3-64 seeded. Fast, non-cryptographic. Suitable for
    /// in-process dedup keys (`Lid`, `Pool` bucketing).
    ///
    /// Ids computed this way are **not** cross-process stable unless
    /// every process uses the same seed.
    Fast {
        /// Seed used by xxHash3.
        seed: u64,
    },

    /// BLAKE3 truncated to 128 bits. Default for content addressing.
    /// Cross-process stable. ~1 GB/s on ARM Cortex-M4.
    Crypto,

    /// Full BLAKE3 256-bit output. For signed manifests and
    /// tamper-evident wire envelopes.
    CryptoFull,
}

/// Bundles a [`HashStrategy`] with the seed that consumers use when
/// the strategy is [`HashStrategy::Fast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct HashConfig {
    /// Selected strategy.
    pub strategy: HashStrategy,
}

impl HashConfig {
    /// Default — [`HashStrategy::Crypto`].
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            strategy: HashStrategy::Crypto,
        }
    }

    /// Fast-only preset. Use when callers do not need cross-process
    /// stable identities (e.g. ephemeral in-memory caches).
    #[must_use]
    pub const fn fast(seed: u64) -> Self {
        Self {
            strategy: HashStrategy::Fast { seed },
        }
    }

    /// Full-cryptographic preset for signed manifests.
    #[must_use]
    pub const fn crypto_full() -> Self {
        Self {
            strategy: HashStrategy::CryptoFull,
        }
    }
}

/// The root configuration object. Constructed once at startup and
/// passed (by `&Config` or by cloning sub-configs) to every consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Config {
    /// Per-traversal caps. See [`BudgetConfig`].
    pub budget: BudgetConfig,
    /// Pool geometry. See [`PoolConfig`].
    pub pool: PoolConfig,
    /// Hash strategy. See [`HashConfig`].
    pub hash: HashConfig,
    /// Deployment profile tag. See [`Profile`].
    pub profile: Profile,
}

impl Config {
    /// Host preset — generous caps, sharded pools, BLAKE3-128 content addresses.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            budget: BudgetConfig::standard(),
            pool: PoolConfig::standard(),
            hash: HashConfig::standard(),
            profile: Profile::Standard,
        }
    }

    /// `no_std + alloc` preset — moderate caps, smaller shard table.
    #[must_use]
    pub const fn embedded() -> Self {
        Self {
            budget: BudgetConfig::embedded(),
            pool: PoolConfig::embedded(),
            hash: HashConfig::standard(),
            profile: Profile::Embedded,
        }
    }

    /// Bare-metal preset — tight caps, single-shard static pools, BLAKE3-128.
    #[must_use]
    pub const fn iot_min() -> Self {
        Self {
            budget: BudgetConfig::iot_min(),
            pool: PoolConfig::iot_min(),
            hash: HashConfig::standard(),
            profile: Profile::IotMin,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_preset_has_non_zero_budgets() {
        let cfg = Config::standard();
        assert!(cfg.budget.max_depth > 0);
        assert!(cfg.budget.max_nodes > 0);
        assert!(cfg.budget.max_bytes > 0);
        assert!(cfg.pool.shard_count > 0);
        assert!(cfg.pool.is_valid());
        assert!(matches!(cfg.hash.strategy, HashStrategy::Crypto));
        assert!(matches!(cfg.profile, Profile::Standard));
    }

    #[test]
    fn embedded_preset_has_non_zero_budgets() {
        let cfg = Config::embedded();
        assert!(cfg.budget.max_depth > 0);
        assert!(cfg.budget.max_nodes > 0);
        assert!(cfg.budget.max_bytes > 0);
        assert!(cfg.pool.is_valid());
        assert!(matches!(cfg.profile, Profile::Embedded));
    }

    #[test]
    fn iot_min_preset_has_non_zero_budgets_and_single_shard() {
        let cfg = Config::iot_min();
        assert!(cfg.budget.max_depth > 0);
        assert!(cfg.budget.max_nodes > 0);
        assert!(cfg.budget.max_bytes > 0);
        assert_eq!(cfg.pool.shard_count, 1);
        assert!(cfg.pool.is_valid());
        assert!(matches!(cfg.profile, Profile::IotMin));
    }

    #[test]
    fn pool_config_validates_power_of_two() {
        assert!(
            PoolConfig {
                shard_count: 1,
                hash_seed: 0,
                initial_bytes: 0
            }
            .is_valid()
        );
        assert!(
            PoolConfig {
                shard_count: 64,
                hash_seed: 0,
                initial_bytes: 0
            }
            .is_valid()
        );
        assert!(
            !PoolConfig {
                shard_count: 0,
                hash_seed: 0,
                initial_bytes: 0
            }
            .is_valid()
        );
        assert!(
            !PoolConfig {
                shard_count: 3,
                hash_seed: 0,
                initial_bytes: 0
            }
            .is_valid()
        );
    }

    #[test]
    fn hash_config_constructors() {
        assert!(matches!(
            HashConfig::standard().strategy,
            HashStrategy::Crypto
        ));
        assert!(matches!(
            HashConfig::fast(42).strategy,
            HashStrategy::Fast { seed: 42 }
        ));
        assert!(matches!(
            HashConfig::crypto_full().strategy,
            HashStrategy::CryptoFull
        ));
    }

    #[test]
    fn config_default_is_standard() {
        assert_eq!(Config::default(), Config::standard());
    }
}
