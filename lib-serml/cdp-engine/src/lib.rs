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

//! # CDP Engine Module
//!
//! ## Overview
//!
//! The core module of SetMint protocol. CDP engine is responsible for handle
//! internal processes about CDPs, including liquidation, settlement and risk
//! management.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{log, pallet_prelude::*, traits::UnixTime, transactional};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use loans::Position;
use orml_traits::Change;
use orml_utilities::OffchainErr;
use primitives::{Amount, Balance, CurrencyId};
use rand_chacha::{
	rand_core::{RngCore, SeedableRng},
	ChaChaRng,
};
use sp_runtime::{
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, Time},
		Duration,
	},
	traits::{Bounded, Convert, One, Saturating, StaticLookup, UniqueSaturatedInto, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::prelude::*;
use support::{
	CDPTreasury, CDPTreasuryExtended, EmergencyShutdown, ExchangeRate, Price, PriceProvider, Rate, Ratio, RiskManager,
};

mod debit_exchange_rate_convertor;
mod mock;
mod tests;
pub mod weights;

pub use debit_exchange_rate_convertor::DebitExchangeRateConvertor;
pub use module::*;
pub use weights::WeightInfo;

pub const OFFCHAIN_WORKER_DATA: &[u8] = b"setheum/cdp-engine/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"setheum/cdp-engine/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"setheum/cdp-engine/max-iterations/";
pub const LOCK_DURATION: u64 = 100;
pub const DEFAULT_MAX_ITERATIONS: u32 = 1000;

pub type LoansOf<T> = loans::Pallet<T>;

/// Risk management params
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, Default)]
pub struct RiskManagementParams {
	/// Maximum total debit value generated from it, when reach the hard
	/// cap, CDP's owner cannot issue more stablecoin under the collateral
	/// type.
	pub maximum_total_debit_value: Balance,

	/// Liquidation ratio, when the collateral ratio of
	/// CDP under this collateral type is below the liquidation ratio, this
	/// CDP is unsafe and can be liquidated. `None` value means not set
	pub liquidation_ratio: Option<Ratio>,

	/// Liquidation penalty rate, when liquidation occurs,
	/// CDP will be deducted an additional penalty base on the product of
	/// penalty rate and debit value. `None` value means not set
	pub liquidation_penalty: Option<Rate>,

	/// Required collateral ratio, if it's set, cannot adjust the position
	/// of CDP so that the current collateral ratio is lower than the
	/// required collateral ratio. `None` value means not set
	pub required_collateral_ratio: Option<Ratio>,
}

// typedef to help polkadot.js disambiguate Change with different generic
// parameters
type ChangeOptionRate = Change<Option<Rate>>;
type ChangeOptionRatio = Change<Option<Ratio>>;
type ChangeBalance = Change<Balance>;

/// Liquidation strategy available
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub enum LiquidationStrategy {
	/// Liquidation CDP's collateral by create collateral auction
	Auction,
	/// Liquidation CDP's collateral by swap with DEX
	Exchange,
}

/// Status of CDP
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub enum CDPStatus {
	Safe,
	Unsafe,
	ChecksFailed(DispatchError),
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + loans::Config + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The origin which may update risk management parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The list of valid collateral currency types
		type CollateralCurrencyIds: Get<Vec<CurrencyId>>;

		#[pallet::constant]
		/// The default liquidation ratio for all collateral types of CDP
		type DefaultLiquidationRatio: Get<Ratio>;

		#[pallet::constant]
		/// The default debit exchange rate for all collateral types
		type DefaultDebitExchangeRate: Get<ExchangeRate>;

		#[pallet::constant]
		/// The default liquidation penalty rate when liquidate unsafe CDP
		type DefaultLiquidationPenalty: Get<Rate>;

		#[pallet::constant]
		/// The minimum debit value to avoid debit dust
		type MinimumDebitValue: Get<Balance>;

		#[pallet::constant]
		/// Stablecoin currency id = SETUSD
		type GetStableCurrencyId: Get<CurrencyId>;

		/// When swap with DEX, the acceptable max slippage for the price from oracle.
		#[pallet::constant]
		type MaxSwapSlippageCompareToOracle: Get<Ratio>;

		/// The CDP treasury to maintain bad debts and surplus generated by CDPs
		type CDPTreasury: CDPTreasuryExtended<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// The price source of all types of currencies related to CDP
		type PriceSource: PriceProvider<CurrencyId>;

		#[pallet::constant]
		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple modules send unsigned transactions.
		type UnsignedPriority: Get<TransactionPriority>;

		/// Emergency shutdown.
		type EmergencyShutdown: EmergencyShutdown;

		/// The default parital path list for CDP engine to swap collateral to stable,
		/// Note: the path is parital, the whole swap path is collateral currency id concat
		/// the partial path. And the list is sorted, CDP engine tries to swap stable by order.
		#[pallet::constant]
		type DefaultSwapParitalPathList: Get<Vec<Vec<CurrencyId>>>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The total debit value of specific collateral type already exceed the
		/// hard cap
		ExceedDebitValueHardCap,
		/// The collateral ratio below the required collateral ratio
		BelowRequiredCollateralRatio,
		/// The collateral ratio below the liquidation ratio
		BelowLiquidationRatio,
		/// The CDP must be unsafe status
		MustBeUnsafe,
		/// The CDP must be safe status
		MustBeSafe,
		/// Invalid collateral type
		InvalidCollateralType,
		/// Remain debit value in CDP below the dust amount
		RemainDebitValueTooSmall,
		/// Feed price is invalid
		InvalidFeedPrice,
		/// No debit value in CDP so that it cannot be settled
		NoDebitValue,
		/// System has already been shutdown
		AlreadyShutdown,
		/// Must after system shutdown
		MustAfterShutdown,
		/// Failed to swap debit by default path list
		SwapDebitFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Liquidate the unsafe CDP. \[collateral_type, owner,
		/// collateral_amount, bad_debt_value, liquidation_strategy\]
		LiquidateUnsafeCDP(CurrencyId, T::AccountId, Balance, Balance, LiquidationStrategy),
		/// Settle the CDP has debit. [collateral_type, owner]
		SettleCDPInDebit(CurrencyId, T::AccountId),
		/// Directly close CDP has debit by handle debit with DEX.
		/// \[collateral_type, owner, sold_collateral_amount,
		/// refund_collateral_amount, debit_value\]
		CloseCDPInDebitByDEX(CurrencyId, T::AccountId, Balance, Balance, Balance),
		/// The liquidation fee for specific collateral type updated.
		/// \[collateral_type, new_liquidation_ratio\]
		LiquidationRatioUpdated(CurrencyId, Option<Ratio>),
		/// The liquidation penalty rate for specific collateral type updated.
		/// \[collateral_type, new_liquidation_panelty\]
		LiquidationPenaltyUpdated(CurrencyId, Option<Rate>),
		/// The required collateral penalty rate for specific collateral type
		/// updated. \[collateral_type, new_required_collateral_ratio\]
		RequiredCollateralRatioUpdated(CurrencyId, Option<Ratio>),
		/// The hard cap of total debit value for specific collateral type
		/// updated. \[collateral_type, new_total_debit_value\]
		MaximumTotalDebitValueUpdated(CurrencyId, Balance),
	}

	/// Mapping from collateral type to its exchange rate of debit units and
	/// debit value
	///
	/// DebitExchangeRate: CurrencyId => Option<ExchangeRate>
	#[pallet::storage]
	#[pallet::getter(fn debit_exchange_rate)]
	pub type DebitExchangeRate<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, ExchangeRate, OptionQuery>;

	/// Mapping from collateral type to its risk management params
	///
	/// CollateralParams: CurrencyId => RiskManagementParams
	#[pallet::storage]
	#[pallet::getter(fn collateral_params)]
	pub type CollateralParams<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, RiskManagementParams, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		#[allow(clippy::type_complexity)]
		pub collaterals_params: Vec<(
			CurrencyId,
			Option<Ratio>,
			Option<Rate>,
			Option<Ratio>,
			Balance,
		)>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {
				collaterals_params: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			self.collaterals_params.iter().for_each(
				|(
					currency_id,
					liquidation_ratio,
					liquidation_penalty,
					required_collateral_ratio,
					maximum_total_debit_value,
				)| {
					CollateralParams::<T>::insert(
						currency_id,
						RiskManagementParams {
							maximum_total_debit_value: *maximum_total_debit_value,
							liquidation_ratio: *liquidation_ratio,
							liquidation_penalty: *liquidation_penalty,
							required_collateral_ratio: *required_collateral_ratio,
						},
					);
				},
			);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// Runs after every block. Start offchain worker to check CDP and
		/// submit unsigned tx to trigger liquidation or settlement.
		fn offchain_worker(now: T::BlockNumber) {
			if let Err(e) = Self::_offchain_worker() {
				debug::info!(
					target: "cdp-engine offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				debug::debug!(
					target: "cdp-engine offchain worker",
					"offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}

		fn integrity_test() {
			assert!(T::DefaultSwapParitalPathList::get()
				.iter()
				.all(|path| !path.is_empty() && path[path.len() - 1] == T::GetStableCurrencyId::get()));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Liquidate unsafe CDP
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// - `currency_id`: CDP's collateral type.
		/// - `who`: CDP's owner.
		#[pallet::weight(<T as Config>::WeightInfo::liquidate_by_dex())]
		#[transactional]
		pub fn liquidate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;
			ensure!(!T::EmergencyShutdown::is_shutdown(), Error::<T>::AlreadyShutdown);
			Self::liquidate_unsafe_cdp(who, currency_id)?;
			Ok(())
		}

		/// Settle CDP has debit after system shutdown
		///
		/// The dispatch origin of this call must be _None_.
		///
		/// - `currency_id`: CDP's collateral type.
		/// - `who`: CDP's owner.
		#[pallet::weight(<T as Config>::WeightInfo::settle())]
		#[transactional]
		pub fn settle(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;
			ensure!(T::EmergencyShutdown::is_shutdown(), Error::<T>::MustAfterShutdown);
			Self::settle_cdp_has_debit(who, currency_id)?;
			Ok(())
		}

		/// Update parameters related to risk management of CDP under specific
		/// collateral type
		///
		/// The dispatch origin of this call must be `UpdateOrigin`.
		///
		/// - `currency_id`: collateral type.
		/// - `liquidation_ratio`: liquidation ratio, `None` means do not
		///   update, `Some(None)` means update it to `None`.
		/// - `liquidation_penalty`: liquidation penalty, `None` means do not
		///   update, `Some(None)` means update it to `None`.
		/// - `required_collateral_ratio`: required collateral ratio, `None`
		///   means do not update, `Some(None)` means update it to `None`.
		/// - `maximum_total_debit_value`: maximum total debit value.
		#[pallet::weight((<T as Config>::WeightInfo::set_collateral_params(), DispatchClass::Operational))]
		#[transactional]
		pub fn set_collateral_params(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			liquidation_ratio: ChangeOptionRatio,
			liquidation_penalty: ChangeOptionRate,
			required_collateral_ratio: ChangeOptionRatio,
			maximum_total_debit_value: ChangeBalance,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::CollateralCurrencyIds::get().contains(&currency_id),
				Error::<T>::InvalidCollateralType,
			);

			let mut collateral_params = Self::collateral_params(currency_id);
			if let Change::NewValue(update) = liquidation_ratio {
				collateral_params.liquidation_ratio = update;
				Self::deposit_event(Event::LiquidationRatioUpdated(currency_id, update));
			}
			if let Change::NewValue(update) = liquidation_penalty {
				collateral_params.liquidation_penalty = update;
				Self::deposit_event(Event::LiquidationPenaltyUpdated(currency_id, update));
			}
			if let Change::NewValue(update) = required_collateral_ratio {
				collateral_params.required_collateral_ratio = update;
				Self::deposit_event(Event::RequiredCollateralRatioUpdated(currency_id, update));
			}
			if let Change::NewValue(val) = maximum_total_debit_value {
				collateral_params.maximum_total_debit_value = val;
				Self::deposit_event(Event::MaximumTotalDebitValueUpdated(currency_id, val));
			}
			CollateralParams::<T>::insert(currency_id, collateral_params);
			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::liquidate(currency_id, who) => {
					let account = T::Lookup::lookup(who.clone())?;
					let Position { collateral, debit } = <LoansOf<T>>::positions(currency_id, &account);
					if !matches!(
						Self::check_cdp_status(*currency_id, collateral, debit),
						CDPStatus::Unsafe
					) || T::EmergencyShutdown::is_shutdown()
					{
						return InvalidTransaction::Stale.into();
					}

					ValidTransaction::with_tag_prefix("CDPEngineOffchainWorker")
						.priority(T::UnsignedPriority::get())
						.and_provides((<frame_system::Pallet<T>>::block_number(), currency_id, who))
						.longevity(64_u64)
						.propagate(true)
						.build()
				}
				Call::settle(currency_id, who) => {
					let account = T::Lookup::lookup(who.clone())?;
					let Position { debit, .. } = <LoansOf<T>>::positions(currency_id, account);
					if debit.is_zero() || !T::EmergencyShutdown::is_shutdown() {
						return InvalidTransaction::Stale.into();
					}

					ValidTransaction::with_tag_prefix("CDPEngineOffchainWorker")
						.priority(T::UnsignedPriority::get())
						.and_provides((currency_id, who))
						.longevity(64_u64)
						.propagate(true)
						.build()
				}
				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	fn submit_unsigned_liquidation_tx(currency_id: CurrencyId, who: T::AccountId) {
		let who = T::Lookup::unlookup(who);
		let call = Call::<T>::liquidate(currency_id, who.clone());
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "cdp-engine offchain worker",
				"submit unsigned liquidation tx for \nCDP - AccountId {:?} CurrencyId {:?} \nfailed!",
				who, currency_id,
			);
		}
	}

	fn submit_unsigned_settlement_tx(currency_id: CurrencyId, who: T::AccountId) {
		let who = T::Lookup::unlookup(who);
		let call = Call::<T>::settle(currency_id, who.clone());
		if SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).is_err() {
			log::info!(
				target: "cdp-engine offchain worker",
				"submit unsigned settlement tx for \nCDP - AccountId {:?} CurrencyId {:?} \nfailed!",
				who, currency_id,
			);
		}
	}

	fn _offchain_worker() -> Result<(), OffchainErr> {
		let collateral_currency_ids = T::CollateralCurrencyIds::get();
		if collateral_currency_ids.len().is_zero() {
			return Ok(());
		}

		// check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		let collateral_currency_ids = T::CollateralCurrencyIds::get();
		let to_be_continue = StorageValueRef::persistent(OFFCHAIN_WORKER_DATA);

		// get to_be_continue record
		let (collateral_position, start_key) =
			if let Ok(Some((last_collateral_position, maybe_last_iterator_previous_key))) =
				to_be_continue.get::<(u32, Option<Vec<u8>>)>()
			{
				(last_collateral_position, maybe_last_iterator_previous_key)
			} else {
				let mut rng = ChaChaRng::from_seed(sp_io::offchain::random_seed());
				(pick_u32(&mut rng, collateral_currency_ids.len() as u32), None)
			};

		// get the max iterationns config
		let max_iterations = StorageValueRef::persistent(OFFCHAIN_WORKER_MAX_ITERATIONS)
			.get::<u32>()
			.unwrap_or(Some(DEFAULT_MAX_ITERATIONS))
			.unwrap_or(DEFAULT_MAX_ITERATIONS);

		let currency_id = collateral_currency_ids[collateral_position as usize];
		let is_shutdown = T::EmergencyShutdown::is_shutdown();

		// If start key is Some(value) continue iterating from that point in storage otherwise start
		// iterating from the beginning of <loans::Positons<T>>
		let mut map_iterator = match start_key.clone() {
			Some(key) => <loans::Positions<T>>::iter_prefix_from(currency_id, key),
			None => <loans::Positions<T>>::iter_prefix(currency_id),
		};

		let mut finished = true;
		let mut iteration_count = 0;
		let iteration_start_time = sp_io::offchain::timestamp();

		#[allow(clippy::while_let_on_iterator)]
		while let Some((who, Position { collateral, debit })) = map_iterator.next() {
			if !is_shutdown
				&& matches!(
					Self::check_cdp_status(currency_id, collateral, debit),
					CDPStatus::Unsafe
				) {
				// liquidate unsafe CDPs before emergency shutdown occurs
				Self::submit_unsigned_liquidation_tx(currency_id, who);
			} else if is_shutdown && !debit.is_zero() {
				// settle CDPs with debit after emergency shutdown occurs.
				Self::submit_unsigned_settlement_tx(currency_id, who);
			}

			iteration_count += 1;
			if iteration_count == max_iterations {
				finished = false;
				break;
			}
			// extend offchain worker lock
			guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
		}
		let iteration_end_time = sp_io::offchain::timestamp();
		log::debug!(
			target: "cdp-engine offchain worker",
			"iteration info:\n max iterations is {:?}\n currency id: {:?}, start key: {:?}, iterate count: {:?}\n iteration start at: {:?}, end at: {:?}, execution time: {:?}\n",
			max_iterations,
			currency_id,
			start_key,
			iteration_count,
			iteration_start_time,
			iteration_end_time,
			iteration_end_time.diff(&iteration_start_time)
		);

		// if iteration for map storage finished, clear to be continue record
		// otherwise, update to be continue record
		if finished {
			let next_collateral_position =
				if collateral_position < collateral_currency_ids.len().saturating_sub(1) as u32 {
					collateral_position + 1
				} else {
					0
				};
			to_be_continue.set(&(next_collateral_position, Option::<Vec<u8>>::None));
		} else {
			to_be_continue.set(&(collateral_position, Some(map_iterator.last_raw_key())));
		}

		// Consume the guard but **do not** unlock the underlying lock.
		guard.forget();

		Ok(())
	}

	pub fn check_cdp_status(currency_id: CurrencyId, collateral_amount: Balance, debit_amount: Balance) -> CDPStatus {
		let stable_currency_id = T::GetStableCurrencyId::get();
		if let Some(feed_price) = T::PriceSource::get_relative_price(currency_id, stable_currency_id) {
			let collateral_ratio =
				Self::calculate_collateral_ratio(currency_id, collateral_amount, debit_amount, feed_price);
			if collateral_ratio < Self::get_liquidation_ratio(currency_id) {
				CDPStatus::Unsafe
			} else {
				CDPStatus::Safe
			}
		} else {
			CDPStatus::ChecksFailed(Error::<T>::InvalidFeedPrice.into())
		}
	}

	pub fn maximum_total_debit_value(currency_id: CurrencyId) -> Balance {
		Self::collateral_params(currency_id).maximum_total_debit_value
	}

	pub fn required_collateral_ratio(currency_id: CurrencyId) -> Option<Ratio> {
		Self::collateral_params(currency_id).required_collateral_ratio
	}

	pub fn get_liquidation_ratio(currency_id: CurrencyId) -> Ratio {
		Self::collateral_params(currency_id)
			.liquidation_ratio
			.unwrap_or_else(T::DefaultLiquidationRatio::get)
	}

	pub fn get_liquidation_penalty(currency_id: CurrencyId) -> Rate {
		Self::collateral_params(currency_id)
			.liquidation_penalty
			.unwrap_or_else(T::DefaultLiquidationPenalty::get)
	}

	pub fn get_debit_exchange_rate(currency_id: CurrencyId) -> ExchangeRate {
		Self::debit_exchange_rate(currency_id).unwrap_or_else(T::DefaultDebitExchangeRate::get)
	}

	pub fn get_debit_value(currency_id: CurrencyId, debit_balance: Balance) -> Balance {
		crate::DebitExchangeRateConvertor::<T>::convert((currency_id, debit_balance))
	}

	pub fn calculate_collateral_ratio(
		currency_id: CurrencyId,
		collateral_balance: Balance,
		debit_balance: Balance,
		price: Price,
	) -> Ratio {
		let locked_collateral_value = price.saturating_mul_int(collateral_balance);
		let debit_value = Self::get_debit_value(currency_id, debit_balance);

		Ratio::checked_from_rational(locked_collateral_value, debit_value).unwrap_or_else(Ratio::max_value)
	}

	pub fn adjust_position(
		who: &T::AccountId,
		currency_id: CurrencyId,
		collateral_adjustment: Amount,
		debit_adjustment: Amount,
	) -> DispatchResult {
		ensure!(
			T::CollateralCurrencyIds::get().contains(&currency_id),
			Error::<T>::InvalidCollateralType,
		);
		<LoansOf<T>>::adjust_position(who, currency_id, collateral_adjustment, debit_adjustment)?;
		Ok(())
	}

	// settle cdp has debit when emergency shutdown
	pub fn settle_cdp_has_debit(who: T::AccountId, currency_id: CurrencyId) -> DispatchResult {
		let Position { collateral, debit } = <LoansOf<T>>::positions(currency_id, &who);
		ensure!(!debit.is_zero(), Error::<T>::NoDebitValue);

		// confiscate collateral in cdp to cdp treasury
		// and decrease CDP's debit to zero
		let settle_price: Price = T::PriceSource::get_relative_price(T::GetStableCurrencyId::get(), currency_id)
			.ok_or(Error::<T>::InvalidFeedPrice)?;
		let bad_debt_value = Self::get_debit_value(currency_id, debit);
		let confiscate_collateral_amount =
			sp_std::cmp::min(settle_price.saturating_mul_int(bad_debt_value), collateral);

		// confiscate collateral and all debit
		<LoansOf<T>>::confiscate_collateral_and_debit(&who, currency_id, confiscate_collateral_amount, debit)?;

		Self::deposit_event(Event::SettleCDPInDebit(currency_id, who));
		Ok(())
	}

	// close cdp has debit by swap collateral to exact debit
	#[transactional]
	pub fn close_cdp_has_debit_by_dex(
		who: T::AccountId,
		currency_id: CurrencyId,
		max_collateral_amount: Balance,
		maybe_path: Option<&[CurrencyId]>,
	) -> DispatchResult {
		let Position { collateral, debit } = <LoansOf<T>>::positions(currency_id, &who);
		ensure!(!debit.is_zero(), Error::<T>::NoDebitValue);
		ensure!(
			matches!(Self::check_cdp_status(currency_id, collateral, debit), CDPStatus::Safe),
			Error::<T>::MustBeSafe
		);

		// confiscate all collateral and debit of unsafe cdp to cdp treasury
		<LoansOf<T>>::confiscate_collateral_and_debit(&who, currency_id, collateral, debit)?;

		// swap exact stable with DEX in limit of price impact
		let debit_value = Self::get_debit_value(currency_id, debit);
		let collateral_supply = collateral.min(max_collateral_amount);

		// if specify swap path
		let actual_supply_collateral = (|| -> Result<Balance, DispatchError> {
			if let Some(path) = maybe_path {
				<T as Config>::CDPTreasury::swap_collateral_to_exact_stable(
					currency_id,
					collateral_supply,
					debit_value,
					path,
					false,
				)
			} else {
				let default_swap_parital_path_list: Vec<Vec<CurrencyId>> = T::DefaultSwapParitalPathList::get();

				// iterator default_swap_parital_path_list to try swap until swap succeed.
				for partial_path in default_swap_parital_path_list {
					let partial_path_len = partial_path.len();

					// check collateral currency_id and partial_path can form a valid swap path.
					if partial_path_len > 0 && currency_id != partial_path[0] {
						let mut swap_path = vec![currency_id];
						swap_path.extend(partial_path);

						if let Ok(actual_supply_collateral) =
							<T as Config>::CDPTreasury::swap_collateral_to_exact_stable(
								currency_id,
								collateral_supply,
								debit_value,
								&swap_path,
								false,
							) {
							return Ok(actual_supply_collateral);
						}
					}
				}

				Err(Error::<T>::SwapDebitFailed.into())
			}
		})()?;

		// refund remain collateral to CDP owner
		let refund_collateral_amount = collateral
			.checked_sub(actual_supply_collateral)
			.expect("swap succecced means collateral >= actual_supply_collateral; qed");
		<T as Config>::CDPTreasury::withdraw_collateral(&who, currency_id, refund_collateral_amount)?;

		Self::deposit_event(Event::CloseCDPInDebitByDEX(
			currency_id,
			who,
			actual_supply_collateral,
			refund_collateral_amount,
			debit_value,
		));
		Ok(())
	}

	// liquidate unsafe cdp
	pub fn liquidate_unsafe_cdp(who: T::AccountId, currency_id: CurrencyId) -> DispatchResult {
		let Position { collateral, debit } = <LoansOf<T>>::positions(currency_id, &who);

		// ensure the cdp is unsafe
		ensure!(
			matches!(
				Self::check_cdp_status(currency_id, collateral, debit),
				CDPStatus::Unsafe
			),
			Error::<T>::MustBeUnsafe
		);

		// confiscate all collateral and debit of unsafe cdp to cdp treasury
		<LoansOf<T>>::confiscate_collateral_and_debit(&who, currency_id, collateral, debit)?;

		let bad_debt_value = Self::get_debit_value(currency_id, debit);
		let target_stable_amount = Self::get_liquidation_penalty(currency_id).saturating_mul_acc_int(bad_debt_value);
		let liquidation_strategy = (|| -> Result<LiquidationStrategy, DispatchError> {
			let default_swap_parital_path_list: Vec<Vec<CurrencyId>> = T::DefaultSwapParitalPathList::get();

			// calculate the supply limit by slippage limit for the price of oracle,
			let max_supply_limit = Ratio::one()
				.saturating_sub(T::MaxSwapSlippageCompareToOracle::get())
				.reciprocal()
				.unwrap_or_else(Ratio::max_value)
				.saturating_mul_int(
					T::PriceSource::get_relative_price(T::GetStableCurrencyId::get(), currency_id)
						.expect("the oracle price should be avalible because liquidation are triggered by it.")
						.saturating_mul_int(target_stable_amount),
				);
			let collateral_supply = collateral.min(max_supply_limit);

			// iterator default_swap_parital_path_list to try swap until swap succeed.
			for partial_path in default_swap_parital_path_list {
				let partial_path_len = partial_path.len();

				// check collateral currency_id and partial_path can form a valid swap path.
				if partial_path_len > 0 && currency_id != partial_path[0] {
					let mut swap_path = vec![currency_id];
					swap_path.extend(partial_path);

					if let Ok(actual_supply_collateral) = <T as Config>::CDPTreasury::swap_collateral_to_exact_stable(
						currency_id,
						collateral_supply,
						target_stable_amount,
						&swap_path,
						false,
					) {
						// refund remain collateral to CDP owner
						let refund_collateral_amount = collateral
							.checked_sub(actual_supply_collateral)
							.expect("swap succecced means collateral >= actual_supply_collateral; qed");

						<T as Config>::CDPTreasury::withdraw_collateral(&who, currency_id, refund_collateral_amount)?;

						return Ok(LiquidationStrategy::Exchange);
					}
				}
			}

			// if cannot liquidate by swap, create collateral auctions by cdp treasury
			<T as Config>::CDPTreasury::create_collateral_auctions(
				currency_id,
				collateral,
				target_stable_amount,
				who.clone(),
				true,
			)?;

			Ok(LiquidationStrategy::Auction)
		})()?;

		Self::deposit_event(Event::LiquidateUnsafeCDP(
			currency_id,
			who,
			collateral,
			bad_debt_value,
			liquidation_strategy,
		));
		Ok(())
	}
}

impl<T: Config> RiskManager<T::AccountId, CurrencyId, Balance, Balance> for Pallet<T> {
	fn get_bad_debt_value(currency_id: CurrencyId, debit_balance: Balance) -> Balance {
		Self::get_debit_value(currency_id, debit_balance)
	}

	fn check_position_valid(
		currency_id: CurrencyId,
		collateral_balance: Balance,
		debit_balance: Balance,
		check_required_ratio: bool,
	) -> DispatchResult {
		if !debit_balance.is_zero() {
			let debit_value = Self::get_debit_value(currency_id, debit_balance);
			let feed_price = <T as Config>::PriceSource::get_relative_price(currency_id, T::GetStableCurrencyId::get())
				.ok_or(Error::<T>::InvalidFeedPrice)?;
			let collateral_ratio =
				Self::calculate_collateral_ratio(currency_id, collateral_balance, debit_balance, feed_price);

			// check the required collateral ratio
			if check_required_ratio {
				if let Some(required_collateral_ratio) = Self::required_collateral_ratio(currency_id) {
					ensure!(
						collateral_ratio >= required_collateral_ratio,
						Error::<T>::BelowRequiredCollateralRatio
					);
				}
			}

			// check the liquidation ratio
			ensure!(
				collateral_ratio >= Self::get_liquidation_ratio(currency_id),
				Error::<T>::BelowLiquidationRatio
			);

			// check the minimum_debit_value
			ensure!(
				debit_value >= T::MinimumDebitValue::get(),
				Error::<T>::RemainDebitValueTooSmall,
			);
		}

		Ok(())
	}

	fn check_debit_cap(currency_id: CurrencyId, total_debit_balance: Balance) -> DispatchResult {
		let hard_cap = Self::maximum_total_debit_value(currency_id);
		let total_debit_value = Self::get_debit_value(currency_id, total_debit_balance);

		ensure!(total_debit_value <= hard_cap, Error::<T>::ExceedDebitValueHardCap,);

		Ok(())
	}
}

/// Pick a new PRN, in the range [0, `max`) (exclusive).
fn pick_u32<R: RngCore>(rng: &mut R, max: u32) -> u32 {
	rng.next_u32() % max
}
