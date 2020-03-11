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


use super::*;
use crate::{GenesisConfig, Module, Trait};
use support::{impl_outer_event, impl_outer_origin, parameter_types, assert_ok};
use sr_primitives::traits::{Verify, Member, CheckedAdd, IdentifyAccount};
use sr_primitives::{Perbill, traits::{IdentityLookup, BlakeTwo256}, testing::Header};
use std::{collections::HashSet, cell::RefCell};
use externalities::set_and_run_with_externalities;
use primitives::{H256, Blake2Hasher, Pair, Public, sr25519};
use support::traits::{Currency, Get, FindAuthor, LockIdentifier};
use sr_primitives::weights::Weight;
use node_primitives::{AccountId, Signature};
use test_client::AccountKeyring;

const NONE: u64 = 0;
const REWARD: Balance = 1000;

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
}
pub type BlockNumber = u64;
pub type Balance = u64;

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestRuntime;

impl Trait for TestRuntime {
    type Event = ();
}

pub type EncointerCurrencies = Module<TestRuntime>;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for TestRuntime {
    type Origin = Origin;
    type Index = u64;
    type Call = ();
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
}

pub type System = system::Module<TestRuntime>;

parameter_types! {
    pub const TransferFee: Balance = 0;
    pub const CreationFee: Balance = 0;
    pub const TransactionBaseFee: u64 = 0;
    pub const TransactionByteFee: u64 = 0;
}
impl balances::Trait for TestRuntime {
    type Balance = Balance;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type Event = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
}
pub type Balances = balances::Module<TestRuntime>;

type AccountPublic = <Signature as Verify>::Signer;

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> runtime_io::TestExternalities {
        let mut storage = system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();
        balances::GenesisConfig::<TestRuntime> {
            balances: vec![],
            vesting: vec![],
        }.assimilate_storage(&mut storage).unwrap();		
        GenesisConfig::<TestRuntime> {
            currency_master: get_accountid(&test_client::AccountKeyring::Alice.pair()),
        }.assimilate_storage(&mut storage).unwrap();		
        runtime_io::TestExternalities::from(storage)
    }
}

impl_outer_origin!{
    pub enum Origin for TestRuntime {}
}

fn get_accountid(pair: &sr25519::Pair) -> AccountId {
    AccountPublic::from(pair.public()).into_account()
}

#[test]
fn solar_trip_time_works() {
    // one degree equator
    let a = Location {lat: 0_000_000, lon: 0_000_000 };
    let b = Location {lat: 0_000_000, lon: 1_000_000 }; // one degree lat is 111km at the equator
    assert_eq!(EncointerCurrencies::solar_trip_time(&a,&b), 1099);
    assert_eq!(EncointerCurrencies::solar_trip_time(&b,&a), 1099);
    // Reykjavik one degree lon: expect to yield much shorter times than at the equator 
    let a = Location {lat: 64_135_480, lon: -21_895_410 }; // this is reykjavik
    let b = Location {lat: 64_135_480, lon: -20_895_410 };
    assert_eq!(EncointerCurrencies::solar_trip_time(&a,&b), 344);
    // Reykjavik 111km: expect to yield much shorter times than at the equator because
    // next time zone is much closer in meter overland. 
    // -> require locations to be further apart (in east-west) at this latitude
    let a = Location {lat: 64_135_480, lon: 0 }; // this is at reykjavik lat
    let b = Location {lat: 64_135_480, lon: 2_290_000 }; // 2.29° is 111km
    assert_eq!(EncointerCurrencies::solar_trip_time(&a,&b), 858);
    // maximal 
    let a = Location {lat: 0_000_000, lon: 0_000_000 };
    let b = Location {lat: 0_000_000, lon: 180_000_000 };
    assert_eq!(EncointerCurrencies::solar_trip_time(&a,&b), 197945); 
    assert_eq!(EncointerCurrencies::solar_trip_time(&b,&a), 197945);
}

#[test]
fn new_currency_works() {
    ExtBuilder::build().execute_with(|| {
        let master = AccountId::from(AccountKeyring::Alice);
        let alice = AccountId::from(AccountKeyring::Alice);
        let bob = AccountId::from(AccountKeyring::Bob);
        let charlie = AccountId::from(AccountKeyring::Charlie);
        let a = Location {lat: 1_000_000, lon: 1_000_000 };
        let b = Location {lat: 1_000_000, lon: 2_000_000 };
        let loc = vec!(a,b);
        let bs = vec!(alice.clone(), bob.clone(), charlie.clone());
        let cid = CurrencyIdentifier::default();
        assert_ok!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc.clone(), bs.clone()));
        let cids = EncointerCurrencies::currency_identifiers();
        assert!(cids.contains(&cid));        
        assert_eq!(EncointerCurrencies::locations(&cid), loc);
        assert_eq!(EncointerCurrencies::bootstrappers(&cid), bs);
    });
}


#[test]
fn new_currency_with_too_close_inner_locations_fails() {
    ExtBuilder::build().execute_with(|| {
        let master = AccountId::from(AccountKeyring::Alice);
        let alice = AccountId::from(AccountKeyring::Alice);
        let bob = AccountId::from(AccountKeyring::Bob);
        let charlie = AccountId::from(AccountKeyring::Charlie);
        let a = Location {lat: 1_000_000, lon: 1_000_000 };
        let b = Location {lat: 1_000_000, lon: 1_000_001 };
        // a and b roughly 11cm apart
        let loc = vec!(a,b);
        let bs = vec!(alice.clone(), bob.clone(), charlie.clone());
        let cid = CurrencyIdentifier::default();

        assert!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc, bs)
            .is_err());
    });
}

#[test]
fn new_currency_too_close_to_existing_currency_fails() {
    ExtBuilder::build().execute_with(|| {
        let master = AccountId::from(AccountKeyring::Alice);
        let alice = AccountId::from(AccountKeyring::Alice);
        let bob = AccountId::from(AccountKeyring::Bob);
        let charlie = AccountId::from(AccountKeyring::Charlie);
        let a = Location {lat: 1_000_000, lon: 1_000_000 };
        let b = Location {lat: 1_000_000, lon: 2_000_000 };
        let loc = vec!(a,b);
        let bs = vec!(alice.clone(), bob.clone(), charlie.clone());
        let cid = CurrencyIdentifier::default();
        assert_ok!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc.clone(), bs.clone()));
        
        // second currency
        let a = Location {lat: 1_000_001, lon: 1_000_001 };
        let b = Location {lat: 1_000_001, lon: 2_000_001 };
        let loc = vec!(a,b);
        assert!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc.clone(), bs.clone())
            .is_err());
    });
}


#[test]
fn new_currency_with_near_pole_locations_fails() {
    ExtBuilder::build().execute_with(|| {
        let master = AccountId::from(AccountKeyring::Alice);
        let alice = AccountId::from(AccountKeyring::Alice);
        let bob = AccountId::from(AccountKeyring::Bob);
        let charlie = AccountId::from(AccountKeyring::Charlie);
        let bs = vec!(alice.clone(), bob.clone(), charlie.clone());
        let cid = CurrencyIdentifier::default();

        let a = Location {lat: 89_000_000, lon: 60_000_000 };
        let b = Location {lat: 89_000_000, lon: -60_000_000 };
        let loc = vec!(a,b);
        assert!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc, bs.clone())
            .is_err());

        let a = Location {lat: -89_000_000, lon: 60_000_000 };
        let b = Location {lat: -89_000_000, lon: -60_000_000 };
        let loc = vec!(a,b);
        assert!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc, bs)
            .is_err());
    
    });
}

#[test]
fn new_currency_near_dateline_fails() {
    ExtBuilder::build().execute_with(|| {
        let master = AccountId::from(AccountKeyring::Alice);
        let alice = AccountId::from(AccountKeyring::Alice);
        let bob = AccountId::from(AccountKeyring::Bob);
        let charlie = AccountId::from(AccountKeyring::Charlie);
        let bs = vec!(alice.clone(), bob.clone(), charlie.clone());
        let cid = CurrencyIdentifier::default();

        let a = Location {lat: 10_000_000, lon: 179_000_000 };
        let b = Location {lat: 11_000_000, lon: 179_000_000 };
        let loc = vec!(a,b);
        assert!(EncointerCurrencies::new_currency(Origin::signed(alice.clone()), cid, loc, bs.clone())
            .is_err());
    });
}

