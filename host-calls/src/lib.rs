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

use runtime_interface::runtime_interface;

#[cfg(feature = "std")]
use log::*;

#[cfg(feature = "std")]
use geo::Point;
#[cfg(feature = "std")]
use geo::prelude::*;

#[runtime_interface]
pub trait RuntimeInterfaces {
    // Only types that implement the RIType (Runtime Interface Type) trait can be returned or supplied as arguments
    // return distance in meters rounded to 1m
    fn distance(a_lat: i32, a_lon: u32, b_lat: i32, b_lon: u32) -> u32 {
        debug!("calling into host call validate_locations()");
        let a = Point::<f64>::from((a_lat as f64 * 1e-6, a_lon as f64 * 1e-6));
        let b = Point::<f64>::from((b_lat as f64 * 1e-6, b_lon as f64 * 1e-6));
        let distance = p1.vincenty_distance(&p2).unwrap();
        distance.rounded() as u32
    }
}
