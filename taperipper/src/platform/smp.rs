// SPDX-License-Identifier: BSD-3-Clause

// How many possible CPU cores we want to support,
// Value should be between 2 and 65536 where log₂(n) ∈ ℤ⁺
// Picked at random by rolling a d20 until it was (0..=15)
pub static MAX_CORES: usize = 2048;
