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

use host_calls::runtime_interfaces;
use support::{decl_module, decl_storage, decl_event, ensure,
	storage::{StorageMap, StorageValue},
	dispatch::Result};
use system::ensure_signed;

use rstd::prelude::*;

use runtime_io::misc::{print_utf8, print_hex};
use primitives::H256; 
use codec::{Encode, Decode};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type CurrencyIndexType = u32;
pub type LocationIndexType = u32;

// Location in lat/lon. Translate float to u32 by round(value*10^6) giving a precision of at least 11cm
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Location {
	pub lat: i32,
	pub lon: u32,
}
pub type CurrencyIdentifier = H256;

const MIN_DISTANCE_M : u32 = 100; // meetup locations must be that many meters apart

decl_storage! {
	trait Store for Module<T: Trait> as EncointerCeremonies {
		Locations get(locations): map CurrencyIdentifier => Vec<Location>;
		Bootstrappers get(bootstrappers): map CurrencyIdentifier => Vec<T::AccountId>;
		CurrencyIdentifiers get(currency_identifiers): Vec<CurrencyIdentifier>;
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
		pub fn new_currency(origin, cid: CurrencyIdentifier, loc: Vec<Location>, bootstrappers: Vec<T::AccountId>) -> Result {
			let sender = ensure_signed(origin)?;
			//let a=loc[0];
			//let b=loc[1];
			//let d = distance(a.lat, a.lon, b.lat, b.lon);
			// TODO: validate distance between all locations globally
			let cids = Self::currency_identifiers();
			for l1 in loc.iter() {
				//test within this currencies' set
				for l2 in loc.iter() {
					if l2 == l1 { continue }
					ensure!(Self::distance(&l1, &l2) >= MIN_DISTANCE_M, "minimum distance violated within supplied locations");
				}
				// test against all other currencies
				for other in cids.iter() {
					for l2 in Self::locations(other) {
						if Self::distance(&l1, &l2) < MIN_DISTANCE_M {
							print_utf8(b"location distance violation for:");
							print_hex(&other.encode());
							return Err("minimum distance violated towards other registered currency");
						}
					}
				}
			}
		
			<CurrencyIdentifiers>::mutate(|v| v.push(cid));
			<Locations>::insert(&cid, &loc);
			<Bootstrappers<T>>::insert(&cid, &bootstrappers);
			Self::deposit_event(RawEvent::CurrencyRegistered(sender, cid));		
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		CurrencyRegistered(AccountId, CurrencyIdentifier),
	}
);

impl<T: Trait> Module<T> {
	fn distance(from: &Location, to: &Location) -> u32 {
		// FIXME: replace by fixpoint implementation within runtime.
		runtime_interfaces::distance(from.lat, from.lon, to.lat, to.lon)
	}
}


#[cfg(test)]
mod tests;

