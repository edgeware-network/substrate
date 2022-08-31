// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_gilt
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-05-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/production/substrate
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_gilt
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --template=./.maintain/frame-weight-template.hbs
// --output=./frame/gilt/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{RefTimeWeight, Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_gilt.
pub trait WeightInfo {
	fn place_bid(l: u32, ) -> Weight;
	fn place_bid_max() -> Weight;
	fn retract_bid(l: u32, ) -> Weight;
	fn set_target() -> Weight;
	fn thaw() -> Weight;
	fn pursue_target_noop() -> Weight;
	fn pursue_target_per_item(b: u32, ) -> Weight;
	fn pursue_target_per_queue(q: u32, ) -> Weight;
}

/// Weights for pallet_gilt using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn place_bid(l: u32, ) -> Weight {
		Weight::from_ref_time(41_605_000 as RefTimeWeight)
			// Standard Error: 0
			.saturating_add(Weight::from_ref_time(62_000 as RefTimeWeight).scalar_saturating_mul(l as RefTimeWeight))
			.saturating_add(T::DbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn place_bid_max() -> Weight {
		Weight::from_ref_time(97_715_000 as RefTimeWeight)
			.saturating_add(T::DbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn retract_bid(l: u32, ) -> Weight {
		Weight::from_ref_time(42_061_000 as RefTimeWeight)
			// Standard Error: 0
			.saturating_add(Weight::from_ref_time(52_000 as RefTimeWeight).scalar_saturating_mul(l as RefTimeWeight))
			.saturating_add(T::DbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	fn set_target() -> Weight {
		Weight::from_ref_time(5_026_000 as RefTimeWeight)
			.saturating_add(T::DbWeight::get().reads(1 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(1 as RefTimeWeight))
	}
	// Storage: Gilt Active (r:1 w:1)
	// Storage: Gilt ActiveTotal (r:1 w:1)
	fn thaw() -> Weight {
		Weight::from_ref_time(47_753_000 as RefTimeWeight)
			.saturating_add(T::DbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:0)
	fn pursue_target_noop() -> Weight {
		Weight::from_ref_time(1_663_000 as RefTimeWeight)
			.saturating_add(T::DbWeight::get().reads(1 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt Active (r:0 w:1)
	fn pursue_target_per_item(b: u32, ) -> Weight {
		Weight::from_ref_time(40_797_000 as RefTimeWeight)
			// Standard Error: 1_000
			.saturating_add(Weight::from_ref_time(4_122_000 as RefTimeWeight).scalar_saturating_mul(b as RefTimeWeight))
			.saturating_add(T::DbWeight::get().reads(3 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes(3 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes((1 as RefTimeWeight).saturating_mul(b as RefTimeWeight)))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt Active (r:0 w:1)
	fn pursue_target_per_queue(q: u32, ) -> Weight {
		Weight::from_ref_time(14_944_000 as RefTimeWeight)
			// Standard Error: 6_000
			.saturating_add(Weight::from_ref_time(8_135_000 as RefTimeWeight).scalar_saturating_mul(q as RefTimeWeight))
			.saturating_add(T::DbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().reads((1 as RefTimeWeight).saturating_mul(q as RefTimeWeight)))
			.saturating_add(T::DbWeight::get().writes(2 as RefTimeWeight))
			.saturating_add(T::DbWeight::get().writes((2 as RefTimeWeight).saturating_mul(q as RefTimeWeight)))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn place_bid(l: u32, ) -> Weight {
		Weight::from_ref_time(41_605_000 as RefTimeWeight)
			// Standard Error: 0
			.saturating_add(Weight::from_ref_time(62_000 as RefTimeWeight).scalar_saturating_mul(l as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn place_bid_max() -> Weight {
		Weight::from_ref_time(97_715_000 as RefTimeWeight)
			.saturating_add(RocksDbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	fn retract_bid(l: u32, ) -> Weight {
		Weight::from_ref_time(42_061_000 as RefTimeWeight)
			// Standard Error: 0
			.saturating_add(Weight::from_ref_time(52_000 as RefTimeWeight).scalar_saturating_mul(l as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	fn set_target() -> Weight {
		Weight::from_ref_time(5_026_000 as RefTimeWeight)
			.saturating_add(RocksDbWeight::get().reads(1 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(1 as RefTimeWeight))
	}
	// Storage: Gilt Active (r:1 w:1)
	// Storage: Gilt ActiveTotal (r:1 w:1)
	fn thaw() -> Weight {
		Weight::from_ref_time(47_753_000 as RefTimeWeight)
			.saturating_add(RocksDbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(2 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:0)
	fn pursue_target_noop() -> Weight {
		Weight::from_ref_time(1_663_000 as RefTimeWeight)
			.saturating_add(RocksDbWeight::get().reads(1 as RefTimeWeight))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt Active (r:0 w:1)
	fn pursue_target_per_item(b: u32, ) -> Weight {
		Weight::from_ref_time(40_797_000 as RefTimeWeight)
			// Standard Error: 1_000
			.saturating_add(Weight::from_ref_time(4_122_000 as RefTimeWeight).scalar_saturating_mul(b as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().reads(3 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes(3 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes((1 as RefTimeWeight).saturating_mul(b as RefTimeWeight)))
	}
	// Storage: Gilt ActiveTotal (r:1 w:1)
	// Storage: Gilt QueueTotals (r:1 w:1)
	// Storage: Gilt Queues (r:1 w:1)
	// Storage: Gilt Active (r:0 w:1)
	fn pursue_target_per_queue(q: u32, ) -> Weight {
		Weight::from_ref_time(14_944_000 as RefTimeWeight)
			// Standard Error: 6_000
			.saturating_add(Weight::from_ref_time(8_135_000 as RefTimeWeight).scalar_saturating_mul(q as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().reads(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().reads((1 as RefTimeWeight).saturating_mul(q as RefTimeWeight)))
			.saturating_add(RocksDbWeight::get().writes(2 as RefTimeWeight))
			.saturating_add(RocksDbWeight::get().writes((2 as RefTimeWeight).saturating_mul(q as RefTimeWeight)))
	}
}
