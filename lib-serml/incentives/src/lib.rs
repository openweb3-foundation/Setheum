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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{log, pallet_prelude::*, transactional, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::{Happened, MultiCurrency, RewardHandler};
use primitives::{Amount, Balance, CurrencyId};
use sp_runtime::{
	traits::{AccountIdConversion, MaybeDisplay, UniqueSaturatedInto, Zero},
	DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{fmt::Debug, vec::Vec};
use support::{SerpTreasury, DEXIncentives, DEXManager, Rate};

mod mock;
mod tests;
pub mod weights;

pub use module::*;
pub use weights::WeightInfo;

/// PoolId for various rewards pools
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PoolId<AccountId> {
	/// Rewards pool(DexCurrencyId) (SDEX or HALAL) for market makers who provide Dex liquidity
	/// for all pools.
	DexIncentive(CurrencyId),

	/// Rewards pool(SetterCurrencyId) (SETT) for market makers who provide Dex liquidity
	/// for NativeCurrency (DNAR or NEOM) pools only.
	DexPremium(CurrencyId),

	/// Rewards pool(SettUSDCurrencyId) (USDJ or JUSD) for market makers who provide Dex liquidity
	/// for all SettCurrency (all system stablecoins) pools only.
	DexPlus(CurrencyId),

	/// Rewards pool(SetterCurrencyId) (SETT) for market makers who provide Dex liquidity
	/// for DexCurrency / Dexer (SDEX or HALAL) pools only.
	DexBonus(CurrencyId),
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ orml_rewards::Config<Share = Balance, Balance = Balance, PoolId = PoolId>
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The vault account to keep rewards for type DexIncentive PoolId
		/// (Receives SettinDex - SDEX/HALAL/RDEX)
		/// For all system currencies except this reward type itself(SDEX/HALAL/RDEX)
		#[pallet::constant]
		type DexIncentivePool: Get<Self::AccountId>;

		/// The vault account to keep rewards for type DexPremium PoolId
		/// (Receives Setter - SETT/NSETT/rSETT)
		/// For Dinar (NativeCurrencyId - DNAR/NEOM/ROME) LPs
		#[pallet::constant]
		type DexPremiumPool: Get<Self::AccountId>;

		/// The vault account to keep rewards for type DexPlus PoolId
		/// (Receives SettUSD - USDJ/JUSD/rUSD)
		/// For SettCurrency (System StableCurrencies) LPs
		#[pallet::constant]
		type DexPlusPool: Get<Self::AccountId>;

		/// The vault account to keep rewards for type DexBonus PoolId
		/// (Receives Setter - SETT/NSETT/rSETT)
		/// For SettinDex (IncentiveCurrencyId - SDEX/HALAL/RDEX) LPs
		#[pallet::constant]
		type DexBonusPool: Get<Self::AccountId>;

		/// The vault account to keep rewards for type DexExtra PoolId
		/// (Receives SettEuro - EURJ/JEUR/rEUR)
		/// For Certain Strategic Currencies LPs
		#[pallet::constant]
		type DexExtraPool: Get<Self::AccountId>;

		/// The period to accumulate rewards
		#[pallet::constant]
		type AccumulatePeriod: Get<Self::BlockNumber>;

		/// The stable currency ids (SettCurrencies)
		type StableCurrencyIds: Get<Vec<CurrencyId>>;

		/// The Incentive reward type (SDEX/HALAL/RDEX)
		/// SDEX in Setheum, HALAL in Neom, RDEX in NewRome testnet
		/// For all LPs
		#[pallet::constant]
		type IncentiveCurrencyId: Get<CurrencyId>;

		/// The Premium reward type (SETT/NSETT/rSETT)
		/// SETT in Setheum, NSETT in Neom, rSETT in NewRome testnet
		/// For Dinar (NativeCurrencyId - DNAR/NEOM/ROME) LPs
		/// DNAR in Setheum, NEOM in Neom, ROME in NewRome testnet
		#[pallet::constant]
		type PremiumCurrencyId: Get<CurrencyId>;

		/// The Plus reward type (USDJ/JUSD/rUSD)
		/// USDJ in Setheum, JUSD in Neom, rUSD in NewRome testnet
		/// For SettCurrency (System StableCurrencies) LPs
		#[pallet::constant]
		type PlusCurrencyId: Get<CurrencyId>;

		/// The Bonus reward type (SETT/NSETT/rSETT)
		/// SETT in Setheum, NSETT in Neom, rSETT in NewRome testnet
		/// For SettinDex (IncentiveCurrencyId - SDEX/HALAL/RDEX) LPs
		/// SDEX in Setheum, HALAL in Neom, RDEX in NewRome testnet
		#[pallet::constant]
		type BonusCurrencyId: Get<CurrencyId>;

		/// The Extra reward type (EURJ/JEUR/rEUR)
		/// EURJ in Setheum, JEUR in Neom, rEUR in NewRome testnet
		/// For Certain Strategic Currencies LPs, e.g. DOT, XBTC et al.
		#[pallet::constant]
		type ExtraCurrencyId: Get<CurrencyId>;

		/// The Native Currency type (DNAR/NEOM/ROME)
		/// DNAR in Setheum, NEOM in Neom, ROME in NewRome testnet
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyId>;

		/// The Dex governance currency type (SDEX/HALAL/rDEX)
		/// SDEX in Setheum, HALAL in Neom, rDEX in NewRome testnet
		#[pallet::constant]
		type DexCurrencyId: Get<CurrencyId>;

		/// The origin which may update incentive related params
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// SERP treasury to issue rewards in stablecoin (Setter (SETT)).
		type SerpTreasury: SerpTreasury<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// Currency to transfer/issue assets
		type Currency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

		/// DEX to supply liquidity info
		type DEX: DEXManager<Self::AccountId, CurrencyId, Balance>;

		/// The module id, keep DexShare LP.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Share amount is not enough
		NotEnough,
		/// Invalid currency id
		InvalidCurrencyType,
		/// Invalid pool id
		InvalidPoolId, // Remove this
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Deposit DEX share. \[who, dex_share_type, deposit_amount\]
		DepositDexShare(T::AccountId, CurrencyId, Balance),
		/// Withdraw DEX share. \[who, dex_share_type, withdraw_amount\]
		WithdrawDexShare(T::AccountId, CurrencyId, Balance),
		/// Claim rewards. \[who, pool_id\]
		ClaimRewards(T::AccountId, T::PoolId),
	}

	/// Mapping from dex liquidity currency type to its Incentive rewards
	/// amount per period
	#[pallet::storage]
	#[pallet::getter(fn dex_incentive_rewards)]
	pub type DexIncentiveRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	/// Mapping from native currency type to its Premium rewards
	/// amount per period
	#[pallet::storage]
	#[pallet::getter(fn dex_premium_rewards)]
	pub type DexPremiumRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	/// Mapping from settcurrency type to its Plus rewards
	/// amount per period
	#[pallet::storage]
	#[pallet::getter(fn dex_plus_rewards)]
	pub type DexPlusRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	/// Mapping from dex incentive currency type to its Bonus rewards
	/// amount per period
	#[pallet::storage]
	#[pallet::getter(fn dex_bonus_rewards)]
	pub type DexBonusRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	/// Mapping from dex incentive currency type to its Extra rewards
	/// amount per period
	#[pallet::storage]
	#[pallet::getter(fn dex_extra_rewards)]
	pub type DexExtraRewards<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::deposit_dex_share())]
		#[transactional]
		pub fn deposit_dex_share(
			origin: OriginFor<T>,
			lp_currency_id: CurrencyId,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_deposit_dex_share(&who, lp_currency_id, amount)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::withdraw_dex_share())]
		#[transactional]
		pub fn withdraw_dex_share(
			origin: OriginFor<T>,
			lp_currency_id: CurrencyId,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_withdraw_dex_share(&who, lp_currency_id, amount)?;
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::claim_rewards())]
		#[transactional]
		pub fn claim_rewards(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			<orml_rewards::Pallet<T>>::claim_rewards(&who, &pool_id);
			Self::deposit_event(Event::ClaimRewards(who, pool_id));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::update_dex_incentive_rewards(updates.len() as u32))]
		#[transactional]
		pub fn update_dex_incentive_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				ensure!(currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
				DexIncentiveRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::update_dex_premium_rewards(updates.len() as u32))]
		#[transactional]
		pub fn update_dex_premium_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				ensure!(currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
				/// ensure it is only offered to NativeCurrency (native incentive currency) pools
				/// therefore only the NativeCurrency (native incentive currency) could be
				/// added/updated to/in this rewards pool.
				ensure!(
                    currency_id == T::NativeCurrencyId::get(),
                    Error::<T>::InvalidCurrencyType,
                );
				DexPremiumRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}
		#[pallet::weight(<T as Config>::WeightInfo::update_dex_plus_rewards(updates.len() as u32))]
		#[transactional]
		pub fn update_dex_plus_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				ensure!(currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
				/// ensure it is only offered to DexCurrency (dex incentive currency) pools
				/// therefore only the DexCurrency (dex incentive currency) could be
				/// added/updated to/in this rewards pool.
				ensure!(
                    currency_id == T::DexCurrencyId::get(),
                    Error::<T>::InvalidCurrencyType,
                );
				DexPlusRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::update_dex_bonus_rewards(updates.len() as u32))]
		#[transactional]
		pub fn update_dex_bonus_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				ensure!(currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
				/// ensure it is only offered to SettCurrencies (system stablecoins) pools
				/// therefore only the the SettCurrencies (system stablecoins) could be
				/// added/updated to/in this rewards pool.
				ensure!(
                    T::StableCurrencyIds::get().contains(&currency_id),
                    Error::<T>::InvalidCurrencyType,
                );
				DexBonusRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::update_dex_extra_rewards(updates.len() as u32))]
		#[transactional]
		pub fn update_dex_extra_rewards(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, Balance)>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			for (currency_id, amount) in updates {
				ensure!(currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
				DexExtraRewards::<T>::insert(currency_id, amount);
			}
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}
}

impl<T: Config> DEXIncentives<T::AccountId, CurrencyId, Balance> for Pallet<T> {
	fn do_deposit_dex_share(who: &T::AccountId, lp_currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(lp_currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);

		T::Currency::transfer(lp_currency_id, who, &Self::account_id(), amount)?;
		<orml_rewards::Pallet<T>>::add_share(
			who,
			PoolId::DexIncentive(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::add_share(
			who,
			PoolId::DexPremium(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::add_share(
			who,
			PoolId::DexPlus(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::add_share(
			who,
			PoolId::DexBonus(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::add_share(
			who,
			PoolId::DexExtra(lp_currency_id),
			amount.unique_saturated_into(),
		);
		Self::deposit_event(Event::DepositDexShare(who.clone(), lp_currency_id, amount));
		Ok(())
	}

	fn do_withdraw_dex_share(who: &T::AccountId, lp_currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(lp_currency_id.is_dex_share_currency_id(), Error::<T>::InvalidCurrencyType);
		ensure!(
			<orml_rewards::Pallet<T>>::share_and_withdrawn_reward(
				PoolId::DexIncentive(lp_currency_id), &who
			).0 >= amount && <orml_rewards::Pallet<T>>::share_and_withdrawn_reward(
				PoolId::DexPremium(lp_currency_id), &who
			).0 >= amount && <orml_rewards::Pallet<T>>::share_and_withdrawn_reward(
				PoolId::DexPlus(lp_currency_id), &who
			).0 >= amount && <orml_rewards::Pallet<T>>::share_and_withdrawn_reward(
				PoolId::DexBonus(lp_currency_id), &who
			).0 >= amount && <orml_rewards::Pallet<T>>::share_and_withdrawn_reward(
				PoolId::DexExtra(lp_currency_id), &who
			).0 >= amount,
			Error::<T>::NotEnough,
		);

		T::Currency::transfer(lp_currency_id, &Self::account_id(), &who, amount)?;
		<orml_rewards::Pallet<T>>::remove_share(
			who,
			PoolId::DexIncentive(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::remove_share(
			who,
			PoolId::DexPremium(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::remove_share(
			who,
			PoolId::DexPlus(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::remove_share(
			who,
			PoolId::DexBonus(lp_currency_id),
			amount.unique_saturated_into(),
		);
		<orml_rewards::Pallet<T>>::remove_share(
			who,
			PoolId::DexExtra(lp_currency_id),
			amount.unique_saturated_into(),
		);
		Self::deposit_event(Event::WithdrawDexShare(who.clone(), lp_currency_id, amount));
		Ok(())
	}
}

impl<T: Config> RewardHandler<T::AccountId> for Pallet<T> {
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type PoolId = T::PoolId;
	type Share = Balance;

	fn accumulate_reward(now: T::BlockNumber, mut callback: impl FnMut(PoolId, Balance)) -> Vec<(CurrencyId, Balance)> {
		let mut accumulated_rewards: Vec<(CurrencyId, Balance)> = vec![];

		// accumulate reward periodically
		if now % T::AccumulatePeriod::get() == Zero::zero() {
			let mut accumulated_incentive: Balance = Zero::zero();
			let mut accumulated_premium: Balance = Zero::zero();
			let mut accumulated_plus: Balance = Zero::zero();
			let mut accumulated_bonus: Balance = Zero::zero();
			let mut accumulated_extra: Balance = Zero::zero();
			let incentive_currency_id = T::IncentiveCurrencyId::get();
			let premium_currency_id = T::PremiumCurrencyId::get();
			let primed_currency_id = T::PrimedCurrencyId::get();
			let plus_currency_id = T::PlusCurrencyId::get();
			let plussed_currency_id = T::DexCurrencyId::get();
			let bonus_currency_id = T::BonusCurrencyId::get();
			let extra_currency_id = T::ExtraCurrencyId::get();

			for (pool_id, pool_info) in orml_rewards::Pools::<T>::iter() {
				if !pool_info.total_shares.is_zero() {
					match pool_id {
						PoolId::DexIncentive(currency_id) => {
							let incentive_reward = Self::dex_incentive_rewards(currency_id);

							/// issue Dex Incentive Currency
							if !incentive_reward.is_zero()
								&& T::Currency::deposit(
									incentive_currency_id,
									&T::DexIncentivePool::get(),
									incentive_reward,
								)
								.is_ok()
							{
								callback(pool_id, incentive_reward);
								accumulated_incentive = accumulated_incentive.saturating_add(incentive_reward);
							}
						}

						PoolId::DexPremium(currency_id) => {
							let premium_reward = Self::dex_premium_rewards(currency_id);

							/// issue Dex Premium Currency
							if !premium_reward.is_zero()
								&& T::Currency::deposit(
									premium_currency_id,
									&T::DexPremiumPool::get(),
									premium_reward,
								)
								.is_ok()
							{
								callback(pool_id, premium_reward);
								accumulated_premium = accumulated_premium.saturating_add(premium_reward);
							}
						}

						PoolId::DexPlus(currency_id) => {
							let plus_reward = Self::dex_plus_rewards(currency_id);

							/// issue Dex Plus Currency
							if !plus_reward.is_zero()
								&& T::Currency::deposit(
									plus_currency_id,
									&T::DexPlusPool::get(),
									plus_reward,
								)
								.is_ok()
							{
								callback(pool_id, plus_reward);
								accumulated_plus = accumulated_plus.saturating_add(plus_reward);
							}
						}

						PoolId::DexBonus(currency_id) => {
							let bonus_reward = Self::dex_bonus_rewards(currency_id);

							/// issue Dex Bonus Currency
							if !bonus_reward.is_zero()
								&& T::Currency::deposit(
									bonus_currency_id,
									&T::DexBonusPool::get(),
									bonus_reward,
								)
								.is_ok()
							{
								callback(pool_id, bonus_reward);
								accumulated_bonus = accumulated_bonus.saturating_add(bonus_reward);
							}
						}

						PoolId::DexExtra(currency_id) => {
							let extra_reward = Self::dex_extra_rewards(currency_id);

							/// issue Dex Extra Currency
							if !extra_reward.is_zero()
								&& T::Currency::deposit(
									extra_currency_id,
									&T::DexExtraPool::get(),
									extra_reward,
								)
								.is_ok()
							{
								callback(pool_id, extra_reward);
								accumulated_extra = accumulated_extra.saturating_add(extra_reward);
							}
						}
					}
				}
			}
			if !accumulated_incentive.is_zero() {
				accumulated_rewards.push((incentive_currency_id, accumulated_incentive));
			}
			if !accumulated_premium.is_zero() {
				accumulated_rewards.push((premium_currency_id, accumulated_premium));
			}
			if !accumulated_plus.is_zero() {
				accumulated_rewards.push((plus_currency_id, accumulated_plus));
			}
			if !accumulated_bonus.is_zero() {
				accumulated_rewards.push((bonus_currency_id, accumulated_bonus));
			}
			if !accumulated_extra.is_zero() {
				accumulated_rewards.push((extra_currency_id, accumulated_extra));
			}
		}

		accumulated_rewards
	}
	fn payout(who: &T::AccountId, pool_id: PoolId, amount: Balance) {
		let (pool_account, currency_id) = match pool_id {
			PoolId::DexIncentive(_) => (T::DexIncentivePool::get(), T::IncentiveCurrencyId::get()),
			PoolId::DexPremium(_) => (T::DexPremiumPool::get(), T::PremiumCurrencyId::get()),
			PoolId::DexPlus(_) => (T::DexPlusPool::get(), T::PlusCurrencyId::get()),
			PoolId::DexBonus(_) => (T::DexBonusPool::get(), T::BonusCurrencyId::get()),
			PoolId::DexExtra(_) => (T::DexExtraPool::get(), T::ExtraCurrencyId::get()),
		};

		// payout the reward to user from the pool. it should not affect the
		// process, ignore the result to continue. if it fails, just the user will not
		// be rewarded, there will not be increase in user balance.
		let _ = T::Currency::transfer(currency_id, &pool_account, &who, amount);
	}
}