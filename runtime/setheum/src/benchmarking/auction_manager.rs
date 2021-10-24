// This file is part of Setheum.

// Copyright (C) 2020-2021 Setheum Labs.
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

use crate::{
	dollar, SetheumOracle, AccountId, AuctionId, AuctionManager, CdpTreasury, Currencies, EmergencyShutdown,
	GetNativeCurrencyId, GetStableCurrencyId, Price, Runtime, SETM,
};

use super::utils::set_balance;
use frame_benchmarking::account;
use frame_system::RawOrigin;
use module_support::AuctionManager as AuctionManagerTrait;
use module_support::CDPTreasury;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, module_auction_manager }

	// `cancel` a collateral auction, worst case:
	// auction have been already bid
	cancel_collateral_auction {
		let bidder: AccountId = account("bidder", 0, SEED);
		let funder: AccountId = account("funder", 0, SEED);
		let stable_currency_id = GetStableCurrencyId::get();

		// set balance
		Currencies::deposit(stable_currency_id, &bidder, 80 * dollar(stable_currency_id))?;
		Currencies::deposit(SETM, &funder, dollar(SETM))?;
		CdpTreasury::deposit_collateral(&funder, SETM, dollar(SETM))?;

		// feed price
		feed_price(SETM, Price::saturating_from_integer(120))?;

		// create collateral auction
		AuctionManager::new_collateral_auction(&funder, SETM, dollar(SETM), 100 * dollar(stable_currency_id))?;
		let auction_id: AuctionId = Default::default();

		// bid collateral auction
		let _ = AuctionManager::collateral_auction_bid_handler(1, auction_id, (bidder, 80 * dollar(stable_currency_id)), None);

		// shutdown
		EmergencyShutdown::emergency_shutdown(RawOrigin::Root.into())?;
	}: cancel(RawOrigin::None, auction_id)
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;

	fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}

	#[test]
	fn test_cancel_collateral_auction() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_cancel_collateral_auction());
		});
	}
}