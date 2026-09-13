//! Configuration of the LOD scheduler: distance thresholds, hysteresis,
//! operation budget, and the active zone.

use std::error::Error;
use std::fmt;

/// Configuration of one planet's LOD scheduler, in world units.
///
/// Split thresholds follow a geometric progression: a chunk at level `L`
/// splits when the player comes closer than
/// [`split_threshold(L)`](Self::split_threshold)
/// (`base_split_distance / 2^L`), matching the subdivision hierarchy where
/// each triangle refines into four children. Merge thresholds add a
/// ratio-based hysteresis band: a split group whose parent sits at level `L`
/// merges only when the player moves farther than
/// [`merge_threshold(L)`](Self::merge_threshold)
/// (`split_threshold(L) * hysteresis_ratio`). Between the two thresholds the
/// state is stable: a player hovering at a threshold causes no split/merge
/// oscillation.
///
/// # Example
///
/// ```
/// use planet_crafter_engine::lod::LodConfig;
///
/// let config = LodConfig::default();
/// assert_eq!(config.split_threshold(0), 1000.0);
/// assert_eq!(config.split_threshold(1), 500.0);
/// assert_eq!(config.merge_threshold(0), 1300.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodConfig {
    /// Player distance below which a level-0 chunk splits, in world units.
    /// Deeper levels halve this threshold per level. Must be positive and
    /// finite.
    pub base_split_distance: f32,
    /// Ratio between the merge threshold and the split threshold of the same
    /// level (the hysteresis band). Must be finite and greater than 1 so the
    /// band has a positive width.
    pub hysteresis_ratio: f32,
    /// Deepest subdivision level the scheduler produces. Level-0 chunks are
    /// the base mesh faces.
    pub max_level: u32,
    /// Shallowest subdivision level the scheduler holds: every chunk is
    /// refined to at least this level, anywhere on the planet and regardless
    /// of player distance, and no group merges below it. Must be at most
    /// `max_level`.
    pub min_level: u32,
    /// Maximum number of split/merge operations executed per
    /// [`update`](crate::lod::LodScheduler::update) call. Forced neighbor
    /// splits count against the same budget. Must be at least 1.
    pub operations_per_frame: usize,
    /// Radius of the active zone: a full sphere around the player position.
    /// Chunks inside the sphere are loaded (active); chunks outside are
    /// unloaded. Must be positive and finite.
    pub active_distance: f32,
    /// Minimum number of active chunks maintained at all times. When fewer
    /// chunks lie inside the active zone, the nearest outside chunks are
    /// loaded to reach the minimum.
    pub min_active_meshes: usize,
}

impl LodConfig {
    /// Player distance below which a chunk at `level` splits:
    /// `base_split_distance / 2^level`.
    pub fn split_threshold(&self, level: u32) -> f32 {
        self.base_split_distance / 2.0_f32.powi(level as i32)
    }

    /// Player distance above which a split group whose parent sits at
    /// `level` merges: `split_threshold(level) * hysteresis_ratio`.
    pub fn merge_threshold(&self, level: u32) -> f32 {
        self.split_threshold(level) * self.hysteresis_ratio
    }

    /// Checks the field invariants documented on [`LodConfig`].
    ///
    /// # Errors
    ///
    /// Returns the first violated invariant as a [`LodConfigError`].
    pub fn validate(&self) -> Result<(), LodConfigError> {
        if !self.base_split_distance.is_finite() || self.base_split_distance <= 0.0 {
            return Err(LodConfigError::InvalidBaseSplitDistance);
        }
        if !self.hysteresis_ratio.is_finite() || self.hysteresis_ratio <= 1.0 {
            return Err(LodConfigError::InvalidHysteresisRatio);
        }
        if self.min_level > self.max_level {
            return Err(LodConfigError::InvalidMinLevel);
        }
        if self.operations_per_frame == 0 {
            return Err(LodConfigError::InvalidOperationsPerFrame);
        }
        if !self.active_distance.is_finite() || self.active_distance <= 0.0 {
            return Err(LodConfigError::InvalidActiveDistance);
        }
        Ok(())
    }
}

impl Default for LodConfig {
    fn default() -> Self {
        LodConfig {
            base_split_distance: 1000.0,
            hysteresis_ratio: 1.3,
            max_level: 8,
            min_level: 0,
            operations_per_frame: 2,
            active_distance: 2000.0,
            min_active_meshes: 20,
        }
    }
}

/// A violated [`LodConfig`] invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodConfigError {
    /// `base_split_distance` was not positive and finite.
    InvalidBaseSplitDistance,
    /// `hysteresis_ratio` was not finite and greater than 1.
    InvalidHysteresisRatio,
    /// `min_level` was greater than `max_level`.
    InvalidMinLevel,
    /// `operations_per_frame` was 0.
    InvalidOperationsPerFrame,
    /// `active_distance` was not positive and finite.
    InvalidActiveDistance,
}

impl fmt::Display for LodConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            LodConfigError::InvalidBaseSplitDistance => {
                "base_split_distance must be positive and finite"
            }
            LodConfigError::InvalidHysteresisRatio => {
                "hysteresis_ratio must be finite and greater than 1"
            }
            LodConfigError::InvalidMinLevel => "min_level must be at most max_level",
            LodConfigError::InvalidOperationsPerFrame => "operations_per_frame must be at least 1",
            LodConfigError::InvalidActiveDistance => "active_distance must be positive and finite",
        };
        f.write_str(message)
    }
}

impl Error for LodConfigError {}
