use sp_core::{Pair, Public, sr25519, H160, Bytes};
use setheum_runtime::{
	AccountId, AirDropConfig, AirDropCurrencyId, CurrencyId, BabeConfig, Balance, BalancesConfig, BlockNumber, GenesisConfig, GrandpaConfig,
	SudoConfig, SystemConfig, IndicesConfig, EvmConfig, TradingPair, EnabledTradingPairs, StakingConfig, SessionConfig, AuthorityDiscoveryConfig,
	WASM_BINARY, dollar, get_all_module_accounts, SS58Prefix, TokenSymbol, TokensConfig, StakerStatus, ImOnlineId, Period, AuthorityDiscoveryId,
	CdpEngineConfig, CdpTreasuryConfig, SystemConfig, NativeTokenExistentialDeposit, opaque::SessionKeys, RenVmBridgeConfig, SessionManagerConfig,
	DexConfig, ShuraCouncilMembershipConfig, SudoConfig, OrmlNFTConfig, FinancialCouncilMembershipConfig, PublicFundCouncilMembershipConfig,
	TechnicalCommitteeMembershipConfig, OperatorMembershipSetheumConfig, SETM, SERP, DNAR, SETR, SETUSD, RENBTC,
};
use runtime_common::TokenInfo;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount};
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;

use sp_std::{collections::btree_map::BTreeMap, str::FromStr};
use sc_chain_spec::ChainSpecExtension;

use serde::{Deserialize, Serialize};

use hex_literal::hex;
use sp_core::{crypto::UncheckedInto, bytes::from_hex};

use setheum_primitives::{AccountPublic, Balance, Nonce};

// The URL for the telemetry server.
const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<setheum_primitives::Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<setheum_primitives::Block>,
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

fn get_session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
	) -> SessionKeys {
	SessionKeys { babe, grandpa, im_online, authority_discovery }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an authority keys.
pub fn get_authority_keys_from_seed(seed: &str)
	-> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?;
	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				get_authority_keys_from_seed("Alice"),
			],
			// Sudo account
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			// Pre-funded accounts
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			],
			// Multicurrency Pre-funded accounts
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
		),
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(setheum_properties()),
		// Extensions
		Default::default(),
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?;
	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"setheum_local_testnet",
		ChainType::Local,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				get_authority_keys_from_seed("Alice"),
				get_authority_keys_from_seed("Bob"),
			],
			// Sudo account
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			// Pre-funded accounts
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
			],
			// Multicurrency Pre-funded accounts
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
		),
		// Bootnodes
		vec![],
		// Telemetry
		// TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		None,
		// Protocol ID
		Some("setheum_local_testnet"),
		// Properties
		Some(setheum_properties()),
		// Extensions
		Default::default(),
	))
}

pub fn public_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?;
	Ok(ChainSpec::from_genesis(
		// Name
		"Setheum Testnet",
		// ID
		"setheum_testnet",
		ChainType::Live,
		move || testnet_genesis(
			wasm_binary,
			// Initial authorities keys:
			// stash
			// controller
			// grandpa
			// babe
			// im-online
			// authority-discovery
			vec![
				(
					hex!["b2902b07056f7365bc22bf7e69c4e4fdba03e6af9c73ca6eb1703ccbc0248857"].into(),
					hex!["cc2ea454844cc1a2e821198d9e0ce1de1aee7d014af5dd3404fc8199df89f821"].into(),
					hex!["607712f6581e191b69046427a7e33c4713e96b4ae4654e2467c74279dc20beb2"].unchecked_into(),
					hex!["ba630d2df03743a6441ab9221a25fc00a62e6f3b56c6920634eebb72a15fc90f"].unchecked_into(),
					hex!["72c0d10c9cd6e44ccf5e7acf0bb1b7c4d6987dda55a36343f3d45b54ad8bfe32"].unchecked_into(),
					hex!["f287831caa53bc1dce6f0d676ab43d248921a4c34535be8f7d7d153eda29dc3f"].unchecked_into(),
				),
				(
					hex!["06ee8fc0e34e40f6f2c98328d70874c6dd7d7989159634c8c87301efbcbe4470"].into(),
					hex!["9cf9f939c16ef458e677472ff113af53e7fb9139244fcfa6fccb765aa8831019"].into(),
					hex!["db6d2cb33abebdc024a14ef7bfbc68823660be8d1acac66770e406e484de3184"].unchecked_into(),
					hex!["d09f879b3273d2cedab83fa741cdac328679c98914dc8dc07e359e19f0379844"].unchecked_into(),
					hex!["8c38deff9ab24a8c49e2b4fbdc963af7cbf06f99d6aabfaa6e50bfe6ae0d071d"].unchecked_into(),
					hex!["dcc1644697e98d4171a29074a4bfaeb49b39b6ea91a8ec5e049d23ea3c4a4134"].unchecked_into(),
				),
				(
					hex!["48267bffea5e524f1c0e06cce77f0ef920be7ed9a7dd47705e181edad64f532a"].into(),
					hex!["38594d7640612c49337f3a0bc7b39232b86f9c9c4fedec3f8b00e45d3f073a2d"].into(),
					hex!["c8996b17688cab9bcda8dafb4dde9bab4d9b1dc81c71419fca46fedcba74a14e"].unchecked_into(),
					hex!["568c17ce5ef308bd9544e7b16f34089a2c2329193f31577a830ffe8a023a6874"].unchecked_into(),
					hex!["66db4135f59db92ce98cdd6c29befaf21a93f1a9059adc2326c7d371a214f97d"].unchecked_into(),
					hex!["00858734321b53f0987a45906cbb91fe7ce1588fce03758c7c07f09022372c30"].unchecked_into(),
				),
			],
			// Sudo
			hex!["0c994e7589709a85128a6695254af16227f7873816ae0269aa705861c315ba1e"].into(),
			// Endowed accounts
			vec![
				// Root
				hex!["0c994e7589709a85128a6695254af16227f7873816ae0269aa705861c315ba1e"].into(),
				// Faucet
				hex!["9e42365c1a43fe7bd886118f49a2247aabda7079c3e4c5288f41afadd7bb1963"].into(),
				// Team Faucet
				hex!["6c1371ce4b06b8d191d6f552d716c00da31aca08a291ccbdeaf0f7aeae51201b"].into(),
				// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA) Faucet
				hex!["6c1371ce4b06b8d191d6f552d716c00da31aca08a291ccbdeaf0f7aeae51201b"].into(),
			],
			// Faucet - MultiCurrency
			hex!["9e42365c1a43fe7bd886118f49a2247aabda7079c3e4c5288f41afadd7bb1963"].into(),
			// Team Faucet
			hex!["6c1371ce4b06b8d191d6f552d716c00da31aca08a291ccbdeaf0f7aeae51201b"].into(),
			// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA) Faucet
			hex!["6c1371ce4b06b8d191d6f552d716c00da31aca08a291ccbdeaf0f7aeae51201b"].into(),
		),
		// Bootnodes
		vec![
			// "/dns/bootnode-t1.setheumscan.com/tcp/30334/p2p/12D3KooWKmFtS7BFtkkKWrP5ZcCpPFokmST2JFXFSsVBNeW5SXWg".parse().unwrap()
		],
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("setheum_testnet"),
		// Properties
		Some(setheum_properties()),
		// Extensions
		Default::default(),
	))
}


pub fn live_mainnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../resources/chain_spec_mainnet_raw.json")[..])
}

pub fn live_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../resources/chain_spec_testnet_raw.json")[..])
}

pub fn mainnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?;
	Ok(ChainSpec::from_genesis(
		// Name
		"Setheum Mainnet",
		// ID
		"setheum_mainnet",
		ChainType::Live,
		move || mainnet_genesis(
			wasm_binary,
			// Initial authorities keys:
			// stash
			// controller
			// grandpa
			// babe
			// im-online
			// authority-discovery
			vec![
				(
					hex!["6c08c1f8e0cf1e200b24b43fca4c4e407b963b6b1e459d1aeff80c566a1da469"].into(),
					hex!["864eff3160ff8609c030316867630850a9d6e35c47d3efe54de44264fef7665e"].into(),
					hex!["dc41d9325da71d90806d727b826d125cd523da28eb39ab048ab983d7bb74fb32"].unchecked_into(),
					hex!["8a688a748fd39bedaa507c942600c40478c2082dee17b8263613fc3c086b0c53"].unchecked_into(),
					hex!["3a4e80c48718f72326b49c4ae80199d35285643751e75a743f30b7561b538676"].unchecked_into(),
					hex!["68d39d0d386ed4e9dd7e280d62e7dc9cf61dc508ef25efb74b6d740fa4dde463"].unchecked_into(),
				),
				(
					hex!["5c22097b5c8b5912ce28b72ba4de52c3da8aca9379c748c1356a6642107d4c4a"].into(),
					hex!["543fd4fd9a284c0f955bb083ae6e0fe7a584eb6f6e72b386071a250b94f99a59"].into(),
					hex!["f15a651be0ea0afcfe691a118ee7acfa114d11a27cf10991ee91ea97942d2135"].unchecked_into(),
					hex!["70e74bed02b733e47bc044da80418fd287bb2b7a0c032bd211d7956c68c9561b"].unchecked_into(),
					hex!["724cefffeaa10a44935a973511b9427a8c3c4fb08582afc4af8bf110fe4aac4b"].unchecked_into(),
					hex!["a068435c438ddc61b1b656e3f61c876e109706383cf4e27309cc1e308f88b86f"].unchecked_into(),
				),
				(
					hex!["a67f388c1b8d68287fb3288b5aa36f069875c15ebcb9b1e4e62678aad6b24b44"].into(),
					hex!["ec912201d98911842b1a8e82983f71f2116dd8b898798ece4e1d210590de7d60"].into(),
					hex!["347f5342875b9847ec089ca723c1c09cc532e53dca4b940a6138040025d94eb9"].unchecked_into(),
					hex!["64841d2d124e1b1dd5485a58908ab244b296b184ae645a0c103adcbcc565f070"].unchecked_into(),
					hex!["50a3452ca93800a8b660d624521c240e5cb20a47a33d23174bb7681811950646"].unchecked_into(),
					hex!["7a0caeb50fbcd657b8388adfaeca41a2ae3e85b8916a2ce92761ce1a4db89035"].unchecked_into(),
				),
			],
			// Sudo
			hex!["9c48c0498bdf1d716f4544fc21f050963409f2db8154ba21e5233001202cbf08"].into(),
			// Endowed accounts
			vec![
				// Foundation
				(hex!["9c48c0498bdf1d716f4544fc21f050963409f2db8154ba21e5233001202cbf08"].into()),
				// Treasury - TODO: Update to `treasury_account`
				(hex!["3c483acc759b79f8b12fa177e4bdfa0448a6ea03c389cf4db2b4325f0fc8f84a"].into()),
				// CashDropFund - TODO: Update to `cashdrop_pool_account`
				(hex!["3c483acc759b79f8b12fa177e4bdfa0448a6ea03c389cf4db2b4325f0fc8f84a"].into()),
				// PublicFundTreasury - TODO: Update to `into_account_id` from `PublicFundTreasuryModuleId`
				(hex!["5adebb35eb317412b58672db0434e4b112fcd27abaf28039f07c0db155b26650"].into()),
				// Team and DEX Liquidity Offering Fund
				(hex!["da512d1335a62ad6f79baecfe87578c5d829113dc85dbb984d90a83f50680145"].into()),
				// Advisors and Partners Fund - Labs
				(hex!["746db342d3981b230804d1a187245e565f8eb3a2897f83d0d841cc52282e324c"].into()),
				// Labs - Shura Council Member
				(hex!["da512d1335a62ad6f79baecfe87578c5d829113dc85dbb984d90a83f50680145"].into()),
				// Muhammad-Jibril B.A. (Khalifa) - Shura Council Member
				(hex!["746db342d3981b230804d1a187245e565f8eb3a2897f83d0d841cc52282e324c"].into()),
			],
			// Treasury - TODO: Update to `treasury_account`
			hex!["3c483acc759b79f8b12fa177e4bdfa0448a6ea03c389cf4db2b4325f0fc8f84a"].into(),
			// CashDropFund - TODO: Update to `cashdrop_pool_account`
			hex!["3c483acc759b79f8b12fa177e4bdfa0448a6ea03c389cf4db2b4325f0fc8f84a"].into(),
			// PublicFundTreasury - TODO: Update to `into_account_id` from `PublicFundTreasuryModuleId`
			hex!["5adebb35eb317412b58672db0434e4b112fcd27abaf28039f07c0db155b26650"].into(),
			// Team and DEX Liquidity Offering Fund
			hex!["da512d1335a62ad6f79baecfe87578c5d829113dc85dbb984d90a83f50680145"].into(),
			// Advisors and Partners Fund
			hex!["746db342d3981b230804d1a187245e565f8eb3a2897f83d0d841cc52282e324c"].into(),
			// Labs - Shura Council Member
			hex!["da512d1335a62ad6f79baecfe87578c5d829113dc85dbb984d90a83f50680145"].into(),
			// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA) - Shura Council Member
			hex!["746db342d3981b230804d1a187245e565f8eb3a2897f83d0d841cc52282e324c"].into(),
		),
		// Bootnodes
		vec![
			// TODO: Uncomment and Update on launch.
		//	"/dns/bootnode.setterscan.com/tcp/30333/p2p/12D3KooWFHSc9cUcyNtavUkLg4VBAeBnYNgy713BnovUa9WNY5pp".parse().unwrap(),
		//	"/dns/bootnode.setheum.xyz/tcp/30333/p2p/12D3KooWAQqcXvcvt4eVEgogpDLAdGWgR5bY1drew44We6FfJAYq".parse().unwrap(),
		//	"/dns/bootnode.setheum.network/tcp/30333/p2p/12D3KooWCT7rnUmEK7anTp7svwr4GTs6k3XXnSjmgTcNvdzWzgWU".parse().unwrap(),
		],
		// Telemetry
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		// Protocol ID
		Some("setheum_mainnet"),
		// Properties
		Some(setheum_properties()),
		// Extensions
		Default::default(),
	))
}

fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	multicurrency_faucet: AccountId,
	labs_faucet: AccountId,
	founder_khalifa_faucet: AccountId,
) -> GenesisConfig {

	let evm_genesis_accounts = evm_genesis();

	let  initial_balance: u128 = 100_000_000 * SETM;
	let  initial_staking: u128 =   10_000_000 * SETM;
	let existential_deposit = NativeTokenExistentialDeposit::get();

	let balances = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), initial_staking))
		.chain(endowed_accounts.iter().cloned().map(|k| (k, initial_balance)))
		.chain(
			get_all_module_accounts()
				.iter()
				.map(|x| (x.clone(), existential_deposit)),
		)
		.fold(
			BTreeMap::<AccountId, Balance>::new(),
			|mut acc, (account_id, amount)| {
				if let Some(balance) = acc.get_mut(&account_id) {
					*balance = balance
						.checked_add(amount)
						.expect("balance cannot overflow when building genesis");
				} else {
					acc.insert(account_id.clone(), amount);
				}
				acc
			},
		)
		.into_iter()
		.collect::<Vec<(AccountId, Balance)>>();

	GenesisConfig {
		frame_system: Some(SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_indices: Some(IndicesConfig { indices: vec![] }),
		pallet_balances: Some(BalancesConfig { balances }),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (
						x.0.clone(), // stash
						x.0.clone(), // stash
						get_session_keys(
							x.2.clone(), // grandpa
							x.3.clone(), // babe
							x.4.clone(), // im-online
							x.5.clone(), // authority-discovery
						)))
				.collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), initial_staking, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: sp_runtime::Perbill::from_percent(10),
			..Default::default()
		}),
		pallet_babe: Some(BabeConfig { authorities: vec![] }),
		pallet_grandpa: Some(GrandpaConfig { authorities: vec![] }),
		pallet_authority_discovery: Some(AuthorityDiscoveryConfig { keys: vec![] }),
		pallet_im_online: Default::default(),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: vec![
				// Faucet allocation
				(multicurrency_faucet.clone(), SETM, initial_balance),
				(multicurrency_faucet.clone(), SERP, initial_balance),
				(multicurrency_faucet.clone(), DNAR, initial_balance),
				(multicurrency_faucet.clone(), SETR, initial_balance),
				(multicurrency_faucet.clone(), SETUSD, initial_balance),
				(multicurrency_faucet.clone(), RENBTC, initial_balance),
				// Team Faucet allocation
				(labs_faucet.clone(), SETM, initial_balance),
				(labs_faucet.clone(), SERP, initial_balance),
				(labs_faucet.clone(), DNAR, initial_balance),
				(labs_faucet.clone(), SETR, initial_balance),
				(labs_faucet.clone(), SETUSD, initial_balance),
				(labs_faucet.clone(), RENBTC, initial_balance),
				// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA) Faucet allocation
				(founder_khalifa_faucet.clone(), SETM, initial_balance),
				(founder_khalifa_faucet.clone(), SERP, initial_balance),
				(founder_khalifa_faucet.clone(), DNAR, initial_balance),
				(founder_khalifa_faucet.clone(), SETR, initial_balance),
				(founder_khalifa_faucet.clone(), SETUSD, initial_balance),
				(founder_khalifa_faucet.clone(), RENBTC, initial_balance),
			],
		}),
		serp_treasury: SerpTreasuryConfig {
			stable_currency_inflation_rate: vec![
				(SETR, 4_000 * dollar(SETR)), 	  // (currency_id, inflation rate of a setcurrency)
				(SETUSD, 8_000 * dollar(SETUSD)), // (currency_id, inflation rate of a setcurrency)
			],
		},
		module_cdp_treasury: CdpTreasuryConfig {
			expected_collateral_auction_size: vec![
				(SETM, dollar(SETM)), 		// (currency_id, max size of a collateral auction)
				(SERP, dollar(SERP)), 		// (currency_id, max size of a collateral auction)
				(DNAR, dollar(DNAR)), 		// (currency_id, max size of a collateral auction)
				(SETR, 5 * dollar(SETR)), 	// (currency_id, max size of a collateral auction)
				(RENBTC, dollar(RENBTC)), 	// (currency_id, max size of a collateral auction)
			],
		},
		module_cdp_engine: CdpEngineConfig {
			collaterals_params: vec![
				(
					SETM,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					SERP,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					DNAR,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					SETR,
					Some(FixedU128::saturating_from_rational(103, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(3, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(106, 100)), // required liquidation ratio
					33_000_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					RENBTC,
					Some(FixedU128::saturating_from_rational(115, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(10, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(125, 100)), // required liquidation ratio
					31_300_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
			],
		},
		module_airdrop: AirDropConfig {
			airdrop_accounts: {
				let setter_airdrop_accounts_json = &include_bytes!("../../../../resources/testnet-airdrop-SETR.json")[..];
				let setter_airdrop_accounts: Vec<(AccountId, Balance)> =
					serde_json::from_slice(setter_airdrop_accounts_json).unwrap();
				let setdollar_airdrop_accounts_json = &include_bytes!("../../../../resources/testnet-airdrop-SETUSD.json")[..];
				let setdollar_airdrop_accounts: Vec<(AccountId, Balance)> =
					serde_json::from_slice(setdollar_airdrop_accounts_json).unwrap();

				setter_airdrop_accounts
					.iter()
					.map(|(account_id, setter_amount)| (account_id.clone(), AirDropCurrencyId::SETR, *setter_amount))
					.chain(
						setdollar_airdrop_accounts
							.iter()
							.map(|(account_id, setdollar_amount)| (account_id.clone(), AirDropCurrencyId::SETUSD, *setdollar_amount)),
					)
					.collect::<Vec<_>>()
			},
		},
		// TODO: Add `SerpTreasuryConfig`
		module_evm: Some(EvmConfig {
			accounts: evm_genesis_accounts,
		}),
		setheum_renvm_bridge: RenVmBridgeConfig {
			ren_vm_public_key: hex!["4b939fc8ade87cb50b78987b1dda927460dc456a"],
		},
		orml_nft: OrmlNFTConfig { tokens: vec![] },
		module_dex: DexConfig {
			initial_listing_trading_pairs: vec![],
			initial_enabled_trading_pairs: EnabledTradingPairs::get(),
			initial_added_liquidity_pools: vec![(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					(TradingPair::new(SETUSD, SETM), (51_600 * dollar(SETUSD), 4_000_000 * dollar(SETM))),
					(TradingPair::new(SETUSD, SERP), (51_600 * dollar(SETUSD), 516_000 * dollar(SERP))),
					(TradingPair::new(SETUSD, DNAR), (51_600 * dollar(SETUSD), 51_600 * dollar(DNAR))),
					(TradingPair::new(SETUSD, SETR), (400_000 * dollar(SETUSD), 200_000 * dollar(SETR))),
					(TradingPair::new(SETUSD, RENBTC), (2_815_000_000 * dollar(SETUSD), 50_000 * dollar(RENBTC))),
				],
			)],
		},
		pallet_treasury: Default::default(),
		pallet_sudo: Some(SudoConfig { key: root_key }),
		pallet_collective_Instance1: Default::default(),
		pallet_membership_Instance1: ShuraCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		pallet_collective_Instance2: Default::default(),
		pallet_membership_Instance2: FinancialCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		pallet_collective_Instance3: Default::default(),
		pallet_membership_Instance3: PublicFundCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		pallet_collective_Instance4: Default::default(),
		pallet_membership_Instance4: TechnicalCommitteeMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		},
		pallet_membership_Instance5: OperatorMembershipSetheumConfig {
			members: endowed_accounts.clone(),
			phantom: Default::default(),
		},
	}
}

fn mainnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)>,
	root_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	treasury_fund: AccountId,
	cashdrop_fund: AccountId,
	spf_fund: AccountId,
	team_fund: AccountId,
	advisors_and_partners_fund: AccountId,
	labs: AccountId,
	founder_khalifa: AccountId,
) -> GenesisConfig {

	let evm_genesis_accounts = evm_genesis();

	let  setm_foundation_alloc: u128 = 516_000_000 * dollar(SETM);
	let  setm_treasury_alloc: u128 = 258_000_000 * dollar(SETM);
	let  setm_spf_alloc: u128 = 516_000_000 * dollar(SETM);
	let  setm_team_alloc: u128 = 1_083_600_000 * dollar(SETM);
	let  setm_advisors_n_partners_alloc: u128 = 206_400_000 * dollar(SETM);

	let  serp_foundation_alloc: u128 = 516_000_000 * dollar(SERP);
	let  serp_treasury_alloc: u128 = 258_000_000 * dollar(SERP);
	let  serp_spf_alloc: u128 = 516_000_000 * dollar(SERP);
	let  serp_team_alloc: u128 = 1_083_600_000 * dollar(SERP);
	let  serp_advisors_n_partners_alloc: u128 = 206_400_000 * dollar(SERP);

	let  dnar_foundation_alloc: u128 = 51_600_000 * dollar(DNAR);
	let  dnar_treasury_alloc: u128 = 25_800_000 * dollar(DNAR);
	let  dnar_spf_alloc: u128 = 51_600_000 * dollar(DNAR);
	let  dnar_team_alloc: u128 = 108_360_000 * dollar(DNAR);
	let  dnar_advisors_n_partners_alloc: u128 = 20_640_000 * dollar(DNAR);

	let  setr_foundation_alloc: u128 = 2_000_000_000 * dollar(SETR);
	let  setr_treasury_alloc: u128 = 1_000_000_000 * dollar(SETR);
	let  setr_cashdrop_alloc: u128 = 2_000_000_000 * dollar(SETR);
	let  setr_spf_alloc: u128 = 2_000_000_000 * dollar(SETR);
	let  setr_team_alloc: u128 = 2_000_000_000 * dollar(SETR);
	let  setr_advisors_n_partners_alloc: u128 = 500_000_000 * dollar(SETR);

	let  setusd_foundation_alloc: u128 = 2_000_000_000 * dollar(SETUSD);
	let  setusd_treasury_alloc: u128 = 1_000_000_000 * dollar(SETUSD);
	let  setusd_cashdrop_alloc: u128 = 2_000_000_000 * dollar(SETUSD);
	let  setusd_spf_alloc: u128 = 2_000_000_000 * dollar(SETUSD);
	let  setusd_team_alloc: u128 = 2_000_000_000 * dollar(SETUSD);
	let  setusd_advisors_n_partners_alloc: u128 = 500_000_000 * dollar(SETUSD);

	let initial_staking: u128 = 258_000 * dollar(SETM);
	let existential_deposit = NativeTokenExistentialDeposit::get();
	let mut total_allocated: Balance = Zero::zero();

	let balances = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), initial_staking + dollar(SETM))) // bit more for fee
		.chain(endowed_accounts.iter().cloned().map(|x| (x.0.clone(), x.1 * dollar(SETM))))
		.chain(
			get_all_module_accounts()
				.iter()
				.map(|x| (x.clone(), existential_deposit)),  // add ED for module accounts
		)
		.fold(
			BTreeMap::<AccountId, Balance>::new(),
			|mut acc, (account_id, amount)| {
				// merge duplicated accounts
				if let Some(balance) = acc.get_mut(&account_id) {
					*balance = balance
						.checked_add(amount)
						.expect("balance cannot overflow when building genesis");
				} else {
					acc.insert(account_id.clone(), amount);
				}

				total_allocated = total_allocated
					.checked_add(amount)
					.expect("total insurance cannot overflow when building genesis");
				acc
			},
		)
		.into_iter()
		.collect::<Vec<(AccountId, Balance)>>();

	GenesisConfig {
		frame_system: Some(SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_indices: Some(IndicesConfig { indices: vec![] }),
		pallet_balances: Some(BalancesConfig { balances }),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (
						x.0.clone(), // stash
						x.0.clone(), // stash
						get_session_keys(
							x.2.clone(), // grandpa
							x.3.clone(), // babe
							x.4.clone(), // im-online
							x.5.clone(), // authority-discovery
						)))
				.collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), initial_staking, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: sp_runtime::Perbill::from_percent(10),
			..Default::default()
		}),
		pallet_babe: Some(BabeConfig { authorities: vec![] }),
		pallet_grandpa: Some(GrandpaConfig { authorities: vec![] }),
		pallet_authority_discovery: Some(AuthorityDiscoveryConfig { keys: vec![] }),
		pallet_im_online: Default::default(),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: vec![
				// Foundation allocation
				(root_key.clone(), SETM, setm_foundation_alloc),
				(root_key.clone(), SERP, serp_foundation_alloc),
				(root_key.clone(), DNAR, dnar_foundation_alloc),
				(root_key.clone(), SETR, setr_foundation_alloc),
				(root_key.clone(), SETUSD, setusd_foundation_alloc),
				// Treasury allocation
				(treasury_fund.clone(), SETM, setm_treasury_alloc),
				(treasury_fund.clone(), SERP, serp_treasury_alloc),
				(treasury_fund.clone(), DNAR, dnar_treasury_alloc),
				(treasury_fund.clone(), SETR, setr_treasury_alloc),
				(treasury_fund.clone(), SETUSD, setusd_treasury_alloc),
				// CashDropFund allocation
				(spf_fund.clone(), SETM, setm_spf_alloc),
				(spf_fund.clone(), SERP, serp_spf_alloc),
				(spf_fund.clone(), DNAR, dnar_spf_alloc),
				(spf_fund.clone(), SETR, setr_spf_alloc),
				(spf_fund.clone(), SETUSD, setusd_spf_alloc),
				// PublicFundTreasury allocation
				(spf_fund.clone(), SETM, setm_spf_alloc),
				(spf_fund.clone(), SERP, serp_spf_alloc),
				(spf_fund.clone(), DNAR, dnar_spf_alloc),
				(spf_fund.clone(), SETR, setr_spf_alloc),
				(spf_fund.clone(), SETUSD, setusd_spf_alloc),
				// Team allocation
				(team_fund.clone(), SETM, setm_team_alloc),
				(team_fund.clone(), SERP, serp_team_alloc),
				(team_fund.clone(), DNAR, dnar_team_alloc),
				(team_fund.clone(), SETR, setr_team_alloc),
				(team_fund.clone(), SETUSD, setusd_team_alloc),
				// Advisors and Partners allocation
				(advisors_and_partners_fund.clone(), SETM, setm_advisors_n_partners_alloc),
				(advisors_and_partners_fund.clone(), SERP, serp_advisors_n_partners_alloc),
				(advisors_and_partners_fund.clone(), DNAR, dnar_advisors_n_partners_alloc),
				(advisors_and_partners_fund.clone(), SETR, setr_advisors_n_partners_alloc),
				(advisors_and_partners_fund.clone(), SETUSD, setusd_advisors_n_partners_alloc),
			],
		}),
		serp_treasury: SerpTreasuryConfig {
			stable_currency_inflation_rate: vec![
				(SETR, 4_000 * dollar(SETR)), 	  // (currency_id, inflation rate of a setcurrency)
				(SETUSD, 8_000 * dollar(SETUSD)), // (currency_id, inflation rate of a setcurrency)
			],
		},
		module_cdp_treasury: CdpTreasuryConfig {
			expected_collateral_auction_size: vec![
				(SETM, dollar(SETM)), 		// (currency_id, max size of a collateral auction)
				(SERP, dollar(SERP)), 		// (currency_id, max size of a collateral auction)
				(DNAR, dollar(DNAR)), 		// (currency_id, max size of a collateral auction)
				(SETR, 5 * dollar(SETR)), 	// (currency_id, max size of a collateral auction)
				(RENBTC, dollar(RENBTC)), 	// (currency_id, max size of a collateral auction)
			],
		},
		module_cdp_engine: CdpEngineConfig {
			collaterals_params: vec![
				(
					SETM,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					SERP,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					DNAR,
					Some(FixedU128::saturating_from_rational(105, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(5, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(110, 100)), // required liquidation ratio
					25_800_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					SETR,
					Some(FixedU128::saturating_from_rational(103, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(3, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(106, 100)), // required liquidation ratio
					33_000_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
				(
					RENBTC,
					Some(FixedU128::saturating_from_rational(115, 100)), // liquidation ratio
					Some(FixedU128::saturating_from_rational(10, 100)),   // liquidation penalty rate
					Some(FixedU128::saturating_from_rational(125, 100)), // required liquidation ratio
					31_300_000 * dollar(SETUSD),                         // maximum debit value in SETUSD (cap)
				),
			],
		},
		module_airdrop: AirDropConfig {
			airdrop_accounts: {
				let setter_airdrop_accounts_json = &include_bytes!("../../../../resources/mainnet-airdrop-SETR.json")[..];
				let setter_airdrop_accounts: Vec<(AccountId, Balance)> =
					serde_json::from_slice(setter_airdrop_accounts_json).unwrap();
				let setdollar_airdrop_accounts_json = &include_bytes!("../../../../resources/mainnet-airdrop-SETUSD.json")[..];
				let setdollar_airdrop_accounts: Vec<(AccountId, Balance)> =
					serde_json::from_slice(setdollar_airdrop_accounts_json).unwrap();

				setter_airdrop_accounts
					.iter()
					.map(|(account_id, setter_amount)| (account_id.clone(), AirDropCurrencyId::SETR, *setter_amount))
					.chain(
						setdollar_airdrop_accounts
							.iter()
							.map(|(account_id, setdollar_amount)| (account_id.clone(), AirDropCurrencyId::SETUSD, *setdollar_amount)),
					)
					.collect::<Vec<_>>()
			},
		},
		// TODO: Add `SerpTreasuryConfig`
		module_evm: Some(EvmConfig {
			accounts: evm_genesis_accounts,
		}),
		// TODO: Update key for RenVM
		// setheum_renvm_bridge: RenVmBridgeConfig {
		// 	ren_vm_public_key: hex!["4b939fc8ade87cb50b78987b1dda927460dc456a"],
		// },
		orml_nft: OrmlNFTConfig { tokens: vec![] },
		module_dex: DexConfig {
			initial_listing_trading_pairs: vec![],
			initial_enabled_trading_pairs: EnabledTradingPairs::get(),
			initial_added_liquidity_pools: vec![],
		},
		module_dex: DexConfig {
			initial_listing_trading_pairs: vec![],
			initial_enabled_trading_pairs: EnabledTradingPairs::get(),
			initial_added_liquidity_pools: vec![
				(
					team_fund.clone(),
					vec![
						(TradingPair::new(SETUSD, SETM), (51_600 * dollar(SETUSD), 4_000_000 * dollar(SETM))),
						(TradingPair::new(SETUSD, SERP), (51_600 * dollar(SETUSD), 516_000 * dollar(SERP))),
						(TradingPair::new(SETUSD, DNAR), (51_600 * dollar(SETUSD), 51_600 * dollar(DNAR))),
						(TradingPair::new(SETUSD, SETR), (400_000 * dollar(SETUSD), 200_000 * dollar(SETR))),
					],
				),
				(
					root_key.clone(),
					vec![
						(TradingPair::new(SETUSD, SETM), (51_600 * dollar(SETUSD), 4_000_000 * dollar(SETM))),
						(TradingPair::new(SETUSD, SERP), (51_600 * dollar(SETUSD), 516_000 * dollar(SERP))),
						(TradingPair::new(SETUSD, DNAR), (51_600 * dollar(SETUSD), 51_600 * dollar(DNAR))),
						(TradingPair::new(SETUSD, SETR), (400_000 * dollar(SETUSD), 200_000 * dollar(SETR))),
					],
				),
			],
		},
		pallet_treasury: Default::default(),
		pallet_sudo: Some(SudoConfig { key: root_key }),
		pallet_collective_Instance1: Default::default(),
		pallet_membership_Instance1: ShuraCouncilMembershipConfig {
			members: vec![
				(root_key.clone()), 		// Setheum Foundation
				(labs.clone()), 			// Setheum Labs
				(team_fund.clone()), 		// Slixon Technologies
				(founder_khalifa.clone()), 	// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA)
			],
			phantom: Default::default(),
		},
		pallet_collective_Instance2: Default::default(),
		pallet_membership_Instance2: FinancialCouncilMembershipConfig {
			members: vec![
				(root_key.clone()), 		// Setheum Foundation
				(labs.clone()), 			// Setheum Labs
				(team_fund.clone()), 		// Slixon Technologies
				(founder_khalifa.clone()), 	// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA)
			],
			phantom: Default::default(),
		},
		pallet_collective_Instance3: Default::default(),
		pallet_membership_Instance3: PublicFundCouncilMembershipConfig {
			members: vec![
				(root_key.clone()), 		// Setheum Foundation
				(labs.clone()), 			// Setheum Labs
				(team_fund.clone()), 		// Slixon Technologies
				(founder_khalifa.clone()), 	// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA)
			],
			phantom: Default::default(),
		},
		pallet_collective_Instance4: Default::default(),
		pallet_membership_Instance4: TechnicalCommitteeMembershipConfig {
			members: vec![
				(root_key.clone()), 		// Setheum Foundation
				(labs.clone()), 			// Setheum Labs
				(team_fund.clone()), 		// Slixon Technologies
				(founder_khalifa.clone()), 	// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA)
			],
			phantom: Default::default(),
		},
		pallet_membership_Instance5: OperatorMembershipSetheumConfig {
			members: vec![
				(root_key.clone()), 		// Setheum Foundation
				(labs.clone()), 			// Setheum Labs
				(team_fund.clone()), 		// Slixon Technologies
				(founder_khalifa.clone()), 	// Muhammad-Jibril Bin Abdullah-Bashir Al-Sharif. (Khalifa MBA)
			],
			phantom: Default::default(),
		},
	}
}


/// Tokens
// TODO: Add SERP!
pub fn setheum_properties() -> Properties {
	let mut properties = Map::new();
	let mut token_symbol: Vec<String> = vec![];
	let mut token_decimals: Vec<u32> = vec![];
	[SETM, SERP, DNAR, SETR, SETUSD].iter().for_each(|token| {
		token_symbol.push(token.symbol().unwrap().to_string());
		token_decimals.push(token.decimals().unwrap() as u32);
	});
	properties.insert("tokenSymbol".into(), token_symbol.into());
	properties.insert("tokenDecimals".into(), token_decimals.into());
	properties.insert("ss58Format".into(), SS58Prefix::get().into());

	properties
}


/// Predeployed contract addresses
pub fn evm_genesis() -> BTreeMap<H160, module_evm::GenesisAccount<Balance, Nonce>> {
	let existential_deposit = NativeTokenExistentialDeposit::get();

	let contracts_json = &include_bytes!("../../predeploy-contracts/resources/bytecodes.json")[..];
	let contracts: Vec<(String, String, String)> = serde_json::from_slice(contracts_json).unwrap();
	let mut accounts = BTreeMap::new();
	for (_, address, code_string) in contracts {
		let account = module_evm::GenesisAccount {
			nonce: 0,
			balance: existential_deposit,
			storage: Default::default(),
			code: Bytes::from_str(&code_string).unwrap().0,
		};
		let addr = H160::from_slice(
			from_hex(address.as_str())
				.expect("predeploy-contracts must specify address")
				.as_slice(),
		);
		accounts.insert(addr, account);
	}
	accounts
}