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

// use host_calls::runtime_interfaces;
use support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::DispatchResult,
    ensure,
    storage::{StorageMap, StorageValue},
};
use system::ensure_signed;

use rstd::prelude::*;

use codec::{Decode, Encode};
pub use fixed::traits::{LossyFrom, LossyInto};
use fixed::transcendental::{asin, cos, powi, sin, sqrt};
use fixed::types::{I32F0, I32F32, U0F64, I64F64};
use primitives::H256;
use runtime_io::{
    hashing::blake2_256,
    misc::{print_hex, print_utf8},
};

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type CurrencyIndexType = u32;
pub type LocationIndexType = u32;
pub type Degree = I32F32;
pub type Demurrage = I64F64;

// Location in lat/lon. Fixpoint value in degree with 8 decimal bits and 24 fractional bits
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Location {
    pub lat: Degree,
    pub lon: Degree,
}
pub type CurrencyIdentifier = H256;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct CurrencyPropertiesType {
    pub name_utf8: Vec<u8>,
    pub demurrage_per_block: Demurrage,
}

const MAX_SPEED_MPS: i32 = 83; // [m/s] max speed over ground of adversary
const MIN_SOLAR_TRIP_TIME_S: i32 = 1; // [s] minimum adversary trip time between two locations measured in local (solar) time.

const DATELINE_DISTANCE_M: u32 = 1_000_000; // meetups may not be closer to dateline (or poles) than this

const NORTH_POLE: Location = Location {
    lon: Degree::from_bits(0i64),
    lat: Degree::from_bits(90i64 << 32),
};
const SOUTH_POLE: Location = Location {
    lon: Degree::from_bits(0i64),
    lat: Degree::from_bits(-90i64 << 32),
};
const DATELINE_LON: Degree = Degree::from_bits(180i64 << 32);

// dec2hex(round(pi/180 * 2^64),16)
const RADIANS_PER_DEGREE: U0F64 = U0F64::from_bits(0x0477D1A894A74E40);

// dec2hex(6371000,8)
// in meters
const MEAN_EARTH_RADIUS: I32F0 = I32F0::from_bits(0x006136B8);

decl_storage! {
    trait Store for Module<T: Trait> as EncointerCurrencies {
        Locations get(fn locations): map hasher(blake2_128_concat) CurrencyIdentifier => Vec<Location>;
        Bootstrappers get(fn bootstrappers): map hasher(blake2_128_concat) CurrencyIdentifier => Vec<T::AccountId>;
        CurrencyIdentifiers get(fn currency_identifiers): Vec<CurrencyIdentifier>;
        CurrencyProperties get(fn currency_properties): map hasher(blake2_128_concat) CurrencyIdentifier => CurrencyPropertiesType;
        // TODO: replace this with on-chain governance
        CurrencyMaster get(fn currency_master) config(): T::AccountId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
        // FIXME: this function has complexity O(n^2)!
        // where n is the number of all locations of all currencies
        // this should be run off-chain in substraTEE-worker later
        pub fn new_currency(origin, loc: Vec<Location>, bootstrappers: Vec<T::AccountId>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let cid = CurrencyIdentifier::from(blake2_256(&(loc.clone(), bootstrappers.clone()).encode()));
            let cids = Self::currency_identifiers();
            ensure!(!cids.contains(&cid), "currency already registered");

            for l1 in loc.iter() {
                ensure!(Self::is_valid_geolocation(&l1), "invalid geolocation specified");
                //test within this currencies' set
                for l2 in loc.iter() {
                    if l2 == l1 { continue }
                    ensure!(Self::solar_trip_time(&l1, &l2) >= MIN_SOLAR_TRIP_TIME_S, "minimum solar trip time violated within supplied locations");
                }
                // prohibit proximity to poles
                if Self::haversine_distance(&l1, &NORTH_POLE) < DATELINE_DISTANCE_M
                    || Self::haversine_distance(&l1, &SOUTH_POLE) < DATELINE_DISTANCE_M {
                    print_utf8(b"location distance violation for:");
                    print_hex(&l1.encode());
                    return Err(<Error<T>>::MinimumDistanceViolationToPole.into());
                }
                // prohibit proximity to dateline
                let dateline_proxy = Location { lat: l1.lat, lon: DATELINE_LON };
                if Self::haversine_distance(&l1, &dateline_proxy) < DATELINE_DISTANCE_M {
                    print_utf8(b"location distance violation for:");
                    print_hex(&l1.encode());
                    return Err(<Error<T>>::MinimumDistanceViolationToDateLine.into());
                }
                // test against all other currencies globally
                for other in cids.iter() {
                    for l2 in Self::locations(other) {
                        if Self::solar_trip_time(&l1, &l2) < MIN_SOLAR_TRIP_TIME_S {
                            print_utf8(b"location distance violation for:");
                            print_hex(&other.encode());
                            return Err(<Error<T>>::MinimumDistanceViolationToOtherCurrency.into());
                        }
                    }
                }
            }

            <CurrencyIdentifiers>::mutate(|v| v.push(cid));
            <Locations>::insert(&cid, &loc);
            <Bootstrappers<T>>::insert(&cid, &bootstrappers);
            <CurrencyProperties>::insert(&cid, 
                CurrencyPropertiesType {
                    name_utf8: b"encointer dummy".to_vec(), 
                    demurrage_per_block: Demurrage::from_bits(0x0000000000000000000001E3F0A8A973_i128)
                }
            );
            Self::deposit_event(RawEvent::CurrencyRegistered(sender, cid));
            print_utf8(b"registered currency wth cid:");
            print_hex(&cid.encode());
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        CurrencyRegistered(AccountId, CurrencyIdentifier),
    }
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// minimum distance violated towards pole
        MinimumDistanceViolationToPole,
        /// minimum distance violated towards dateline
        MinimumDistanceViolationToDateLine,
        /// minimum distance violated towards other currency's location
		MinimumDistanceViolationToOtherCurrency,
	}
}

impl<T: Trait> Module<T> {
    fn solar_trip_time(from: &Location, to: &Location) -> i32 {
        // FIXME: replace by fixpoint implementation within runtime.
        let d = Module::<T>::haversine_distance(&from, &to) as i32;
        // FIXME: this will not panic, but make sure!
        let dt = from
            .lon
            .checked_sub(to.lon)
            .unwrap()
            .checked_div(Degree::from_num(1))
            .unwrap()
            .checked_mul(Degree::from_num(240))
            .unwrap(); // 24h * 3600s / 360° = 240s/°
        let tflight = d.checked_div(MAX_SPEED_MPS).unwrap();
        let dt: i32 = dt.abs().lossy_into();
        tflight - dt
    }

    pub fn is_valid_geolocation(loc: &Location) -> bool {
        if loc.lat > NORTH_POLE.lat {
            return false;
        }
        if loc.lat < SOUTH_POLE.lat {
            return false;
        }
        if loc.lon > DATELINE_LON {
            return false;
        }
        if loc.lon < -DATELINE_LON {
            return false;
        }
        true
    }

    pub fn haversine_distance(a: &Location, b: &Location) -> u32 {
        type I = I32F32;
        let two = I::from_num(2);
        let theta1 = I::from(a.lat) * I::lossy_from(RADIANS_PER_DEGREE);
        let theta2 = I::from(b.lat) * I::lossy_from(RADIANS_PER_DEGREE);
        let delta_theta = theta1 - theta2;
        let delta_lambda = I::from(a.lon - b.lon) * I::lossy_from(RADIANS_PER_DEGREE);
        let tmp0 = sin(delta_theta / two);
        let tmp1 = if let Ok(r) = powi::<I, I>(tmp0, 2) {
            r
        } else {
            I::from_num(0)
        };
        let tmp2 = cos(theta1) * cos(theta2);
        let tmp3 = sin(delta_lambda / two);
        let tmp4 = if let Ok(r) = powi::<I, I>(tmp3, 2) {
            r
        } else {
            I::from_num(0)
        };
        let aa = tmp1 + tmp2 * tmp4;
        let c: I = two * asin(sqrt::<I, I>(aa).unwrap());
        let d = I::from(MEAN_EARTH_RADIUS) * c;
        let d: i64 = d.lossy_into();
        d as u32
    }
}

#[cfg(test)]
#[macro_use]
extern crate approx;

#[cfg(test)]
mod tests;
