// This file is part of Setheum.

// Copyright (C) 2019-2021 Setheum Labs.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benefit pallet benchmarking.
use super::*;
use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use sp_std::vec;

const SEED: u32 = 0;
const ACCOUNT_INIT_BALANCE: u32 = 1_000_000_000;

use crate::Module as Benefits;

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_INIT_BALANCE.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}


benchmarks! {
    add_benefit_funds {
        let user = create_funded_user::<T>("user", 100);
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 1000u32.into(), FundsType::MARKET)
    verify {
        assert_eq!(Benefits::<T>::market_benefits(&user).active_funds, T::Currency::minimum_balance() * 1000u32.into());
    }

    cut_benefit_funds {
        let user = create_funded_user::<T>("user", 100);
        Benefits::<T>::add_benefit_funds(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into(), FundsType::MARKET).expect("Something wrong during add benefit funds");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 1500u32.into(), FundsType::MARKET)
    verify {
        assert_eq!(Benefits::<T>::market_benefits(&user).active_funds, T::Currency::minimum_balance() * 500u32.into());
    }

    rebond_benefit_funds {
        let user = create_funded_user::<T>("user", 100);
        Benefits::<T>::add_benefit_funds(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into(), FundsType::MARKET).expect("Something wrong during add benefit funds");
        Benefits::<T>::cut_benefit_funds(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 1000u32.into(), FundsType::MARKET).expect("Something wrong during cut benefit funds");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 500u32.into(), FundsType::MARKET)
    verify {
        assert_eq!(Benefits::<T>::market_benefits(&user).active_funds, T::Currency::minimum_balance() * 1500u32.into());
    }

    withdraw_benefit_funds {
        let user = create_funded_user::<T>("user", 100);
        Benefits::<T>::add_benefit_funds(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into(), FundsType::MARKET).expect("Something wrong during add benefit funds");
        Benefits::<T>::cut_benefit_funds(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 1000u32.into(), FundsType::MARKET).expect("Something wrong during cut benefit funds");
        Benefits::<T>::update_era_benefit(200u32.into(), 100u32.into());
    }: _(RawOrigin::Signed(user.clone()))
    verify {
        assert_eq!(Benefits::<T>::market_benefits(&user).unlocking_funds.len(), 0);
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn add_benefit_funds() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_add_benefit_funds::<Test>());
        });
    }

    #[test]
    fn cut_benefit_funds() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_cut_benefit_funds::<Test>());
        });
    }

    #[test]
    fn rebond_benefit_funds() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_rebond_benefit_funds::<Test>());
        });
    }

    #[test]
    fn withdraw_benefit_funds() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_benefit_funds::<Test>());
        });
    }
}