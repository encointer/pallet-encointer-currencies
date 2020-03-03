//  Copyright (c) 2019 Alain Brenzikofer
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! # Encointer Currencies Module
//!
//! provides functionality for 
//! - registering new currencies
//! - modify currency characteristics
//!

#![cfg_attr(not(feature = "std"), no_std)]

use support::{decl_module, decl_storage, decl_event, ensure,
	storage::{StorageDoubleMap, StorageMap, StorageValue},
	traits::Currency,
	dispatch::Result};
use system::{ensure_signed, ensure_root};

use rstd::prelude::*;

use sr_primitives::traits::{Verify, Member, CheckedAdd, IdentifyAccount};
use sr_primitives::MultiSignature;
use runtime_io::misc::print_utf8;

use codec::{Codec, Encode, Decode};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type CurrencyIndexType = u32;
pub type LocationIndexType = u32;

// L
const MAX_LAT: i32 = 
//! Location in lat/lon. Translate float to u32 by round(value*10^6) giving a precision of at least 11cm
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Location {
	pub lat: i32,
	pub lon: u32,
}
pub type CurrencyIdentifier = Hash;

decl_storage! {
	trait Store for Module<T: Trait> as EncointerCeremonies {
		Locations get(locations_registry): map CurrencyIdentifier => Vec<Location>;
		Bootstrappers get(bootstrappers): map CurrencyIdentifier => Vec<T::AccountId>;
		// caution: index starts with 1, not 0! (because null and 0 is the same for state storage)
		CurrencyIndex get(currency_index): map CurrencyIndexType => CurrencyIdentifier;
		CurrencyCount get(currency_count): CurrencyIndexType;
		// TODO: replace this with on-chain governance
		CurrencyMaster get(currency_master) config(): T::AccountId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		// FIXME: this function has complexity O(n^2)! 
		// where n is the number of all locations of all currencies 
		// this should be run off-chain in substraTEE-worker later
		pub fn new_currency(origin, cid: CurrencyIdentifier, loc: Vec<Location>, bootstrappers: Vec<AccountId>) -> Result {
			let sender = ensure_signed(origin)?;
			validate_locations(loc)?;
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		CurrencyRegistered(CurrencyIdentifier),
	}
);

impl<T: Trait> Module<T> {
	fn validate_locations(loc: Vec<Location>) -> Result {
		// ensure we're not near the poles
		for l in loc.iter() {
			ensure!(loc.lat > MAX_LAT, "too far north"}
		}
		Ok(())
	}
}


#[cfg(test)]
mod tests;

