#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2018::*;
#[macro_use]
extern crate std;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use error::CommonError;
use stake_wrapper::StakeWrapper;
use std::{
    convert::{TryFrom, TryInto},
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};
use ticket_account::TicketAccountData;
pub mod calc {
    //! Common calculations
    use crate::error::CommonError;
    use std::convert::TryFrom;
    /// calculate amount*numerator/denominator
    /// as value  = shares * share_price where share_price=total_value/total_shares
    /// or shares = amount_value / share_price where share_price=total_value/total_shares
    ///     => shares = amount_value * 1/share_price where 1/share_price=total_shares/total_value
    pub fn proportional(amount: u64, numerator: u64, denominator: u64) -> Result<u64, CommonError> {
        if denominator == 0 {
            return Ok(amount);
        }
        u64::try_from((amount as u128) * (numerator as u128) / (denominator as u128))
            .map_err(|_| CommonError::CalculationFailure)
    }
    #[inline]
    pub fn value_from_shares(
        shares: u64,
        total_value: u64,
        total_shares: u64,
    ) -> Result<u64, CommonError> {
        proportional(shares, total_value, total_shares)
    }
    pub fn shares_from_value(
        value: u64,
        total_value: u64,
        total_shares: u64,
    ) -> Result<u64, CommonError> {
        if total_shares == 0 {
            Ok(value)
        } else {
            proportional(value, total_shares, total_value)
        }
    }
}
pub mod checks {
    use crate::CommonError;
    use anchor_lang::prelude::*;
    use anchor_lang::solana_program::stake::state::StakeState;
    use anchor_spl::token::{Mint, TokenAccount};
    pub fn check_min_amount(amount: u64, min_amount: u64, action_name: &str) -> ProgramResult {
        if amount >= min_amount {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["", ": Number too low ", " (min is ", ")"],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&action_name),
                        ::core::fmt::ArgumentV1::new_display(&amount),
                        ::core::fmt::ArgumentV1::new_display(&min_amount),
                    ],
                ));
                res
            });
            Err(CommonError::NumberTooLow.into())
        }
    }
    pub fn check_address(
        actual_address: &Pubkey,
        reference_address: &Pubkey,
        field_name: &str,
    ) -> ProgramResult {
        if actual_address == reference_address {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Invalid ", " address: expected ", " got "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(&reference_address),
                        ::core::fmt::ArgumentV1::new_display(&actual_address),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidArgument)
        }
    }
    pub fn check_owner_program<'info, A: ToAccountInfo<'info>>(
        account: &A,
        owner: &Pubkey,
        field_name: &str,
    ) -> ProgramResult {
        let actual_owner = account.to_account_info().owner;
        if actual_owner == owner {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Invalid ", " owner_program: expected ", " got "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(&owner),
                        ::core::fmt::ArgumentV1::new_display(&actual_owner),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidArgument)
        }
    }
    pub fn check_mint_authority(
        mint: &Mint,
        mint_authority: Pubkey,
        field_name: &str,
    ) -> ProgramResult {
        if mint.mint_authority.contains(&mint_authority) {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Invalid ", " mint authority ", ". Expected "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(
                            &mint.mint_authority.unwrap_or_default(),
                        ),
                        ::core::fmt::ArgumentV1::new_display(&mint_authority),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidAccountData)
        }
    }
    pub fn check_freeze_authority(mint: &Mint, field_name: &str) -> ProgramResult {
        if mint.freeze_authority.is_none() {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Mint ", " must have freeze authority not set"],
                    &[::core::fmt::ArgumentV1::new_display(&field_name)],
                ));
                res
            });
            Err(ProgramError::InvalidAccountData)
        }
    }
    pub fn check_mint_empty(mint: &Mint, field_name: &str) -> ProgramResult {
        if mint.supply == 0 {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Non empty mint ", " supply: "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(&mint.supply),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidArgument)
        }
    }
    pub fn check_token_mint(token: &TokenAccount, mint: Pubkey, field_name: &str) -> ProgramResult {
        if token.mint == mint {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Invalid token ", " mint ", ". Expected "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(&token.mint),
                        ::core::fmt::ArgumentV1::new_display(&mint),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidAccountData)
        }
    }
    pub fn check_token_owner(
        token: &TokenAccount,
        owner: &Pubkey,
        field_name: &str,
    ) -> ProgramResult {
        if token.owner == *owner {
            Ok(())
        } else {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["Invalid token account ", " owner ", ". Expected "],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&field_name),
                        ::core::fmt::ArgumentV1::new_display(&token.owner),
                        ::core::fmt::ArgumentV1::new_display(&owner),
                    ],
                ));
                res
            });
            Err(ProgramError::InvalidAccountData)
        }
    }
    pub fn check_stake_amount_and_validator(
        stake_state: &StakeState,
        expected_stake_amount: u64,
        validator_vote_pubkey: &Pubkey,
    ) -> ProgramResult {
        let currently_staked = if let Some(delegation) = stake_state.delegation() {
            if delegation.voter_pubkey != *validator_vote_pubkey {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Invalid stake validator index. Need to point into validator "],
                        &[::core::fmt::ArgumentV1::new_display(&validator_vote_pubkey)],
                    ));
                    res
                });
                return Err(ProgramError::InvalidInstructionData);
            }
            delegation.stake
        } else {
            return Err(CommonError::StakeNotDelegated.into());
        };
        if currently_staked != expected_stake_amount {
            ::solana_program::log::sol_log(&{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &[
                        "Operation on a stake account not yet updated. expected stake:",
                        ", current:",
                    ],
                    &[
                        ::core::fmt::ArgumentV1::new_display(&expected_stake_amount),
                        ::core::fmt::ArgumentV1::new_display(&currently_staked),
                    ],
                ));
                res
            });
            return Err(CommonError::StakeAccountNotUpdatedYet.into());
        }
        Ok(())
    }
}
pub mod error {
    use anchor_lang::prelude::*;
    /// Anchor generated Result to be used as the return type for the
    /// program.
    pub type Result<T> = std::result::Result<T, Error>;
    /// Anchor generated error allowing one to easily return a
    /// `ProgramError` or a custom, user defined error code by utilizing
    /// its `From` implementation.
    #[doc(hidden)]
    pub enum Error {
        #[error(transparent)]
        ProgramError(#[from] anchor_lang::solana_program::program_error::ProgramError),
        #[error(transparent)]
        ErrorCode(#[from] CommonError),
    }
    #[allow(unused_qualifications)]
    impl std::error::Error for Error {
        fn source(&self) -> std::option::Option<&(dyn std::error::Error + 'static)> {
            use thiserror::private::AsDynError;
            #[allow(deprecated)]
            match self {
                Error::ProgramError { 0: transparent } => {
                    std::error::Error::source(transparent.as_dyn_error())
                }
                Error::ErrorCode { 0: transparent } => {
                    std::error::Error::source(transparent.as_dyn_error())
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::fmt::Display for Error {
        fn fmt(&self, __formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
            match self {
                Error::ProgramError(_0) => std::fmt::Display::fmt(_0, __formatter),
                Error::ErrorCode(_0) => std::fmt::Display::fmt(_0, __formatter),
            }
        }
    }
    #[allow(unused_qualifications)]
    impl std::convert::From<anchor_lang::solana_program::program_error::ProgramError> for Error {
        #[allow(deprecated)]
        fn from(source: anchor_lang::solana_program::program_error::ProgramError) -> Self {
            Error::ProgramError { 0: source }
        }
    }
    #[allow(unused_qualifications)]
    impl std::convert::From<CommonError> for Error {
        #[allow(deprecated)]
        fn from(source: CommonError) -> Self {
            Error::ErrorCode { 0: source }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Error {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Error::ProgramError(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "ProgramError");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
                (&Error::ErrorCode(ref __self_0),) => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_tuple(f, "ErrorCode");
                    let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &&(*__self_0));
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }
        }
    }
    #[repr(u32)]
    pub enum CommonError {
        WrongReserveOwner,
        NonEmptyReserveData,
        InvalidInitialReserveLamports,
        ZeroValidatorChunkSize,
        TooBigValidatorChunkSize,
        ZeroCreditChunkSize,
        TooBigCreditChunkSize,
        TooLowCreditFee,
        InvalidMintAuthority,
        MintHasInitialSupply,
        InvalidOwnerFeeState,
        InvalidProgramId = 6116,
        UnexpectedAccount = 65140,
        CalculationFailure = 51619,
        AccountWithLockup = 45694,
        NumberTooLow = 7892,
        NumberTooHigh = 7893,
        FeeTooHigh = 4052,
        FeesWrongWayRound = 4053,
        LiquidityTargetTooLow = 4054,
        TicketNotDue = 4055,
        TicketNotReady = 4056,
        WrongBeneficiary = 4057,
        StakeAccountNotUpdatedYet = 4058,
        StakeNotDelegated = 4059,
        StakeAccountIsEmergencyUnstaking = 4060,
        InsufficientLiquidity = 4205,
        InvalidValidator = 47525,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for CommonError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&CommonError::WrongReserveOwner,) => {
                    ::core::fmt::Formatter::write_str(f, "WrongReserveOwner")
                }
                (&CommonError::NonEmptyReserveData,) => {
                    ::core::fmt::Formatter::write_str(f, "NonEmptyReserveData")
                }
                (&CommonError::InvalidInitialReserveLamports,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidInitialReserveLamports")
                }
                (&CommonError::ZeroValidatorChunkSize,) => {
                    ::core::fmt::Formatter::write_str(f, "ZeroValidatorChunkSize")
                }
                (&CommonError::TooBigValidatorChunkSize,) => {
                    ::core::fmt::Formatter::write_str(f, "TooBigValidatorChunkSize")
                }
                (&CommonError::ZeroCreditChunkSize,) => {
                    ::core::fmt::Formatter::write_str(f, "ZeroCreditChunkSize")
                }
                (&CommonError::TooBigCreditChunkSize,) => {
                    ::core::fmt::Formatter::write_str(f, "TooBigCreditChunkSize")
                }
                (&CommonError::TooLowCreditFee,) => {
                    ::core::fmt::Formatter::write_str(f, "TooLowCreditFee")
                }
                (&CommonError::InvalidMintAuthority,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidMintAuthority")
                }
                (&CommonError::MintHasInitialSupply,) => {
                    ::core::fmt::Formatter::write_str(f, "MintHasInitialSupply")
                }
                (&CommonError::InvalidOwnerFeeState,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidOwnerFeeState")
                }
                (&CommonError::InvalidProgramId,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidProgramId")
                }
                (&CommonError::UnexpectedAccount,) => {
                    ::core::fmt::Formatter::write_str(f, "UnexpectedAccount")
                }
                (&CommonError::CalculationFailure,) => {
                    ::core::fmt::Formatter::write_str(f, "CalculationFailure")
                }
                (&CommonError::AccountWithLockup,) => {
                    ::core::fmt::Formatter::write_str(f, "AccountWithLockup")
                }
                (&CommonError::NumberTooLow,) => {
                    ::core::fmt::Formatter::write_str(f, "NumberTooLow")
                }
                (&CommonError::NumberTooHigh,) => {
                    ::core::fmt::Formatter::write_str(f, "NumberTooHigh")
                }
                (&CommonError::FeeTooHigh,) => ::core::fmt::Formatter::write_str(f, "FeeTooHigh"),
                (&CommonError::FeesWrongWayRound,) => {
                    ::core::fmt::Formatter::write_str(f, "FeesWrongWayRound")
                }
                (&CommonError::LiquidityTargetTooLow,) => {
                    ::core::fmt::Formatter::write_str(f, "LiquidityTargetTooLow")
                }
                (&CommonError::TicketNotDue,) => {
                    ::core::fmt::Formatter::write_str(f, "TicketNotDue")
                }
                (&CommonError::TicketNotReady,) => {
                    ::core::fmt::Formatter::write_str(f, "TicketNotReady")
                }
                (&CommonError::WrongBeneficiary,) => {
                    ::core::fmt::Formatter::write_str(f, "WrongBeneficiary")
                }
                (&CommonError::StakeAccountNotUpdatedYet,) => {
                    ::core::fmt::Formatter::write_str(f, "StakeAccountNotUpdatedYet")
                }
                (&CommonError::StakeNotDelegated,) => {
                    ::core::fmt::Formatter::write_str(f, "StakeNotDelegated")
                }
                (&CommonError::StakeAccountIsEmergencyUnstaking,) => {
                    ::core::fmt::Formatter::write_str(f, "StakeAccountIsEmergencyUnstaking")
                }
                (&CommonError::InsufficientLiquidity,) => {
                    ::core::fmt::Formatter::write_str(f, "InsufficientLiquidity")
                }
                (&CommonError::InvalidValidator,) => {
                    ::core::fmt::Formatter::write_str(f, "InvalidValidator")
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for CommonError {
        #[inline]
        fn clone(&self) -> CommonError {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for CommonError {}
    impl std::fmt::Display for CommonError {
        fn fmt(
            &self,
            fmt: &mut std::fmt::Formatter<'_>,
        ) -> std::result::Result<(), std::fmt::Error> {
            match self {
                CommonError::WrongReserveOwner => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Wrong reserve owner. Must be a system account"],
                        &[],
                    ));
                    result
                }
                CommonError::NonEmptyReserveData => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Reserve must have no data, but has data"],
                        &[],
                    ));
                    result
                }
                CommonError::InvalidInitialReserveLamports => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Invalid initial reserve lamports"],
                        &[],
                    ));
                    result
                }
                CommonError::ZeroValidatorChunkSize => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Zero validator chunk size"],
                        &[],
                    ));
                    result
                }
                CommonError::TooBigValidatorChunkSize => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Too big validator chunk size"],
                        &[],
                    ));
                    result
                }
                CommonError::ZeroCreditChunkSize => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Zero credit chunk size"],
                        &[],
                    ));
                    result
                }
                CommonError::TooBigCreditChunkSize => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Too big credit chunk size"],
                        &[],
                    ));
                    result
                }
                CommonError::TooLowCreditFee => {
                    let result =
                        fmt.write_fmt(::core::fmt::Arguments::new_v1(&["Too low credit fee"], &[]));
                    result
                }
                CommonError::InvalidMintAuthority => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Invalid mint authority"],
                        &[],
                    ));
                    result
                }
                CommonError::MintHasInitialSupply => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Non empty initial mint supply"],
                        &[],
                    ));
                    result
                }
                CommonError::InvalidOwnerFeeState => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["Invalid owner fee state"],
                        &[],
                    ));
                    result
                }
                CommonError::InvalidProgramId => {
                    let result = fmt . write_fmt (:: core :: fmt :: Arguments :: new_v1 (& ["1910 Invalid program id. For using program from another account please update id in the code"] , & [])) ;
                    result
                }
                CommonError::UnexpectedAccount => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["FFA0 Unexpected account"],
                        &[],
                    ));
                    result
                }
                CommonError::CalculationFailure => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["CACF Calculation failure"],
                        &[],
                    ));
                    result
                }
                CommonError::AccountWithLockup => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["B3AA You can\'t deposit a stake-account with lockup"],
                        &[],
                    ));
                    result
                }
                CommonError::NumberTooLow => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["2000 Number too low"],
                        &[],
                    ));
                    result
                }
                CommonError::NumberTooHigh => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["2001 Number too high"],
                        &[],
                    ));
                    result
                }
                CommonError::FeeTooHigh => {
                    let result =
                        fmt.write_fmt(::core::fmt::Arguments::new_v1(&["1100 Fee too high"], &[]));
                    result
                }
                CommonError::FeesWrongWayRound => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1101 Min fee > max fee"],
                        &[],
                    ));
                    result
                }
                CommonError::LiquidityTargetTooLow => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1102 Liquidity target too low"],
                        &[],
                    ));
                    result
                }
                CommonError::TicketNotDue => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1103 Ticket not due. Wait more epochs"],
                        &[],
                    ));
                    result
                }
                CommonError::TicketNotReady => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1104 Ticket not ready. Wait a few hours and try again"],
                        &[],
                    ));
                    result
                }
                CommonError::WrongBeneficiary => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1105 Wrong Ticket Beneficiary"],
                        &[],
                    ));
                    result
                }
                CommonError::StakeAccountNotUpdatedYet => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1106 Stake Account not updated yet"],
                        &[],
                    ));
                    result
                }
                CommonError::StakeNotDelegated => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1107 Stake Account not delegated"],
                        &[],
                    ));
                    result
                }
                CommonError::StakeAccountIsEmergencyUnstaking => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1108 Stake Account is emergency unstaking"],
                        &[],
                    ));
                    result
                }
                CommonError::InsufficientLiquidity => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["1199 Insufficient Liquidity in the Liquidity Pool"],
                        &[],
                    ));
                    result
                }
                CommonError::InvalidValidator => {
                    let result = fmt.write_fmt(::core::fmt::Arguments::new_v1(
                        &["BAD1 Invalid validator"],
                        &[],
                    ));
                    result
                }
            }
        }
    }
    impl std::error::Error for CommonError {}
    impl std::convert::From<Error> for anchor_lang::solana_program::program_error::ProgramError {
        fn from(e: Error) -> anchor_lang::solana_program::program_error::ProgramError {
            match e {
                Error::ProgramError(e) => e,
                Error::ErrorCode(c) => {
                    anchor_lang::solana_program::program_error::ProgramError::Custom(
                        c as u32 + anchor_lang::__private::ERROR_CODE_OFFSET,
                    )
                }
            }
        }
    }
    impl std::convert::From<CommonError> for anchor_lang::solana_program::program_error::ProgramError {
        fn from(e: CommonError) -> anchor_lang::solana_program::program_error::ProgramError {
            let err: Error = e.into();
            err.into()
        }
    }
}
pub mod liq_pool {
    use crate::{
        calc::proportional, checks::check_address, error::CommonError, located::Located, Fee,
        State, ID,
    };
    use anchor_lang::prelude::*;
    pub mod add_liquidity {
        use crate::AddLiquidity;
        use super::LiqPoolHelpers;
        use crate::{calc::shares_from_value, checks::*};
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke, system_instruction, system_program};
        use anchor_spl::token::{mint_to, MintTo};
        impl<'info> AddLiquidity<'info> {
            fn check_transfer_from(&self, lamports: u64) -> ProgramResult {
                check_owner_program(&self.transfer_from, &system_program::ID, "transfer_from")?;
                if self.transfer_from.lamports() < lamports {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["", " balance is ", " but expected "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.transfer_from.key),
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.transfer_from.lamports(),
                                ),
                                ::core::fmt::ArgumentV1::new_display(&lamports),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InsufficientFunds);
                }
                Ok(())
            }
            fn check_mint_to(&self) -> ProgramResult {
                check_token_mint(&self.mint_to, self.state.liq_pool.lp_mint, "mint_to")?;
                Ok(())
            }
            pub fn process(&mut self, lamports: u64) -> ProgramResult {
                ::solana_program::log::sol_log("add-liq pre check");
                check_min_amount(lamports, self.state.min_deposit, "add_liquidity")?;
                self.state
                    .liq_pool
                    .check_lp_mint(self.lp_mint.to_account_info().key)?;
                self.state
                    .check_lp_mint_authority(self.lp_mint_authority.key)?;
                self.state
                    .liq_pool
                    .check_liq_pool_msol_leg(self.liq_pool_msol_leg.to_account_info().key)?;
                self.state
                    .check_liq_pool_sol_leg_pda(self.liq_pool_sol_leg_pda.key)?;
                self.check_transfer_from(lamports)?;
                self.state
                    .liq_pool
                    .check_liquidity_cap(lamports, self.liq_pool_sol_leg_pda.lamports())?;
                self.check_mint_to()?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(self.token_program.key, &spl_token::ID, "token_program")?;
                ::solana_program::log::sol_log("add-liq after check");
                if self.lp_mint.supply > self.state.liq_pool.lp_supply {
                    ::solana_program::log::sol_log(
                        "Someone minted lp tokens without our permission or bug found",
                    );
                    return Err(ProgramError::InvalidAccountData);
                }
                self.state.liq_pool.lp_supply = self.lp_mint.supply;
                let sol_leg_lamports = self
                    .liq_pool_sol_leg_pda
                    .lamports()
                    .checked_sub(self.state.rent_exempt_for_token_acc)
                    .expect("sol_leg_lamports");
                let msol_leg_value = self
                    .state
                    .calc_lamports_from_msol_amount(self.liq_pool_msol_leg.amount)
                    .expect("msol_leg_value");
                let total_liq_pool_value = sol_leg_lamports + msol_leg_value;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &[
                            "liq_pool SOL:",
                            ", liq_pool mSOL value:",
                            " liq_pool_value:",
                        ],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&sol_leg_lamports),
                            ::core::fmt::ArgumentV1::new_display(&msol_leg_value),
                            ::core::fmt::ArgumentV1::new_display(&total_liq_pool_value),
                        ],
                    ));
                    res
                });
                let shares_for_user = shares_from_value(
                    lamports,
                    total_liq_pool_value,
                    self.state.liq_pool.lp_supply,
                )?;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["LP for user "],
                        &[::core::fmt::ArgumentV1::new_display(&shares_for_user)],
                    ));
                    res
                });
                invoke(
                    &system_instruction::transfer(
                        self.transfer_from.key,
                        self.liq_pool_sol_leg_pda.key,
                        lamports,
                    ),
                    &[
                        self.transfer_from.clone(),
                        self.liq_pool_sol_leg_pda.clone(),
                        self.system_program.clone(),
                    ],
                )?;
                self.state.with_lp_mint_authority_seeds(|mint_seeds| {
                    mint_to(
                        CpiContext::new_with_signer(
                            self.token_program.clone(),
                            MintTo {
                                mint: self.lp_mint.to_account_info(),
                                to: self.mint_to.to_account_info(),
                                authority: self.lp_mint_authority.clone(),
                            },
                            &[mint_seeds],
                        ),
                        shares_for_user,
                    )
                })?;
                self.state.liq_pool.on_lp_mint(shares_for_user);
                Ok(())
            }
        }
    }
    pub mod initialize {
        use super::LiqPool;
        use crate::{
            checks::{
                check_address, check_freeze_authority, check_mint_authority, check_mint_empty,
                check_owner_program, check_token_mint, check_token_owner,
            },
            CommonError, Fee, Initialize, LiqPoolInitialize, LiqPoolInitializeData,
        };
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::system_program;
        impl<'info> LiqPoolInitialize<'info> {
            pub fn check_liq_mint(parent: &mut Initialize) -> ProgramResult {
                check_owner_program(&parent.liq_pool.lp_mint, &spl_token::ID, "lp_mint")?;
                if parent.liq_pool.lp_mint.to_account_info().key
                    == parent.msol_mint.to_account_info().key
                {
                    ::solana_program::log::sol_log(
                        "Use different mints for stake and liquidity pool",
                    );
                    return Err(ProgramError::InvalidAccountData);
                }
                let (authority_address, authority_bump_seed) =
                    LiqPool::find_lp_mint_authority(parent.state_address());
                check_mint_authority(&parent.liq_pool.lp_mint, authority_address, "lp_mint")?;
                parent.state.liq_pool.lp_mint_authority_bump_seed = authority_bump_seed;
                check_mint_empty(&parent.liq_pool.lp_mint, "lp_mint")?;
                check_freeze_authority(&parent.liq_pool.lp_mint, "lp_mint")?;
                Ok(())
            }
            pub fn check_sol_account_pda(parent: &mut Initialize) -> ProgramResult {
                check_owner_program(
                    &parent.liq_pool.sol_leg_pda,
                    &system_program::ID,
                    "liq_sol_account_pda",
                )?;
                let (address, bump) = LiqPool::find_sol_leg_address(parent.state_address());
                check_address(
                    parent.liq_pool.sol_leg_pda.key,
                    &address,
                    "liq_sol_account_pda",
                )?;
                parent.state.liq_pool.sol_leg_bump_seed = bump;
                {
                    let lamports = parent.liq_pool.sol_leg_pda.lamports();
                    if lamports != parent.state.rent_exempt_for_token_acc {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &[
                                    "Invalid initial liq_sol_account_pda lamports ",
                                    " expected ",
                                ],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(&lamports),
                                    ::core::fmt::ArgumentV1::new_display(
                                        &parent.state.rent_exempt_for_token_acc,
                                    ),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InvalidArgument);
                    }
                }
                Ok(())
            }
            pub fn check_msol_account(parent: &mut Initialize) -> ProgramResult {
                check_owner_program(&parent.liq_pool.msol_leg, &spl_token::ID, "liq_msol_leg")?;
                check_token_mint(
                    &parent.liq_pool.msol_leg,
                    *parent.msol_mint.to_account_info().key,
                    "liq_msol",
                )?;
                let (msol_authority, msol_authority_bump_seed) =
                    LiqPool::find_msol_leg_authority(parent.state_address());
                check_token_owner(&parent.liq_pool.msol_leg, &msol_authority, "liq_msol_leg")?;
                parent.state.liq_pool.msol_leg_authority_bump_seed = msol_authority_bump_seed;
                Ok(())
            }
            pub fn check_fees(min_fee: Fee, max_fee: Fee) -> ProgramResult {
                min_fee.check()?;
                max_fee.check()?;
                if max_fee.basis_points > 1000 {
                    return Err(CommonError::FeeTooHigh.into());
                }
                if min_fee > max_fee {
                    return Err(CommonError::FeesWrongWayRound.into());
                }
                Ok(())
            }
            pub fn process(parent: &mut Initialize, data: LiqPoolInitializeData) -> ProgramResult {
                Self::check_liq_mint(parent)?;
                Self::check_sol_account_pda(parent)?;
                Self::check_msol_account(parent)?;
                Self::check_fees(data.lp_min_fee, data.lp_max_fee)?;
                data.lp_treasury_cut.check()?;
                parent.state.liq_pool.lp_mint = *parent.liq_pool.lp_mint.to_account_info().key;
                parent.state.liq_pool.msol_leg = *parent.liq_pool.msol_leg.to_account_info().key;
                parent.state.liq_pool.treasury_cut = data.lp_treasury_cut;
                parent.state.liq_pool.lp_liquidity_target = data.lp_liquidity_target;
                parent.state.liq_pool.lp_min_fee = data.lp_min_fee;
                parent.state.liq_pool.lp_max_fee = data.lp_max_fee;
                parent.state.liq_pool.liquidity_sol_cap = std::u64::MAX;
                Ok(())
            }
        }
    }
    pub mod remove_liquidity {
        use crate::{
            calc::proportional,
            checks::{check_address, check_min_amount, check_owner_program, check_token_mint},
            liq_pool::LiqPoolHelpers,
            RemoveLiquidity,
        };
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke_signed, system_instruction, system_program};
        use anchor_spl::token::{burn, transfer, Burn, Transfer};
        impl<'info> RemoveLiquidity<'info> {
            fn check_burn_from(&self, tokens: u64) -> ProgramResult {
                check_token_mint(&self.burn_from, self.state.liq_pool.lp_mint, "burn_from")?;
                if *self.burn_from_authority.key == self.burn_from.owner {
                    if self.burn_from.amount < tokens {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Requested to remove ", " liquidity but have only "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(&tokens),
                                    ::core::fmt::ArgumentV1::new_display(&self.burn_from.amount),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else if self
                    .burn_from
                    .delegate
                    .contains(self.burn_from_authority.key)
                {
                    if self.burn_from.delegated_amount < tokens {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Delegated ", " liquidity. Requested "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.burn_from.delegated_amount,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&tokens),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Token must be delegated to "],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.burn_from_authority.key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                Ok(())
            }
            fn check_transfer_sol_to(&self) -> ProgramResult {
                check_owner_program(
                    &self.transfer_sol_to,
                    &system_program::ID,
                    "transfer_sol_to",
                )?;
                Ok(())
            }
            fn check_transfer_msol_to(&self) -> ProgramResult {
                check_token_mint(
                    &self.transfer_msol_to,
                    self.state.msol_mint,
                    "transfer_msol_to",
                )?;
                Ok(())
            }
            pub fn process(&mut self, tokens: u64) -> ProgramResult {
                ::solana_program::log::sol_log("rem-liq pre check");
                self.state
                    .liq_pool
                    .check_lp_mint(self.lp_mint.to_account_info().key)?;
                self.check_burn_from(tokens)?;
                self.check_transfer_sol_to()?;
                self.check_transfer_msol_to()?;
                self.state
                    .check_liq_pool_sol_leg_pda(self.liq_pool_sol_leg_pda.key)?;
                self.state
                    .liq_pool
                    .check_liq_pool_msol_leg(self.liq_pool_msol_leg.to_account_info().key)?;
                self.state
                    .check_liq_pool_msol_leg_authority(self.liq_pool_msol_leg_authority.key)?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(self.token_program.key, &spl_token::ID, "token_program")?;
                if self.lp_mint.supply > self.state.liq_pool.lp_supply {
                    ::solana_program::log::sol_log(
                        "Someone minted lp tokens without our permission or bug found",
                    );
                } else {
                    self.state.liq_pool.lp_supply = self.lp_mint.supply;
                }
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["mSOL-SOL-LP total supply:"],
                        &[::core::fmt::ArgumentV1::new_display(&self.lp_mint.supply)],
                    ));
                    res
                });
                let sol_out_amount = proportional(
                    tokens,
                    self.liq_pool_sol_leg_pda
                        .lamports()
                        .checked_sub(self.state.rent_exempt_for_token_acc)
                        .unwrap(),
                    self.state.liq_pool.lp_supply,
                )?;
                let msol_out_amount = proportional(
                    tokens,
                    self.liq_pool_msol_leg.amount,
                    self.state.liq_pool.lp_supply,
                )?;
                check_min_amount(
                    sol_out_amount
                        .checked_add(
                            self.state
                                .calc_lamports_from_msol_amount(msol_out_amount)
                                .expect("Error converting mSOLs to lamports"),
                        )
                        .expect("lamports overflow"),
                    self.state.min_withdraw,
                    "removed liquidity",
                )?;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["SOL out amount:", ", mSOL out amount:"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&sol_out_amount),
                            ::core::fmt::ArgumentV1::new_display(&msol_out_amount),
                        ],
                    ));
                    res
                });
                if sol_out_amount > 0 {
                    ::solana_program::log::sol_log("transfer SOL");
                    self.state.with_liq_pool_sol_leg_seeds(|sol_seeds| {
                        invoke_signed(
                            &system_instruction::transfer(
                                self.liq_pool_sol_leg_pda.key,
                                self.transfer_sol_to.key,
                                sol_out_amount,
                            ),
                            &[
                                self.liq_pool_sol_leg_pda.clone(),
                                self.transfer_sol_to.clone(),
                                self.system_program.clone(),
                            ],
                            &[sol_seeds],
                        )
                    })?;
                }
                if msol_out_amount > 0 {
                    ::solana_program::log::sol_log("transfer mSOL");
                    self.state
                        .with_liq_pool_msol_leg_authority_seeds(|msol_seeds| {
                            transfer(
                                CpiContext::new_with_signer(
                                    self.token_program.clone(),
                                    Transfer {
                                        from: self.liq_pool_msol_leg.to_account_info(),
                                        to: self.transfer_msol_to.to_account_info(),
                                        authority: self.liq_pool_msol_leg_authority.clone(),
                                    },
                                    &[msol_seeds],
                                ),
                                msol_out_amount,
                            )
                        })?;
                }
                burn(
                    CpiContext::new(
                        self.token_program.clone(),
                        Burn {
                            mint: self.lp_mint.to_account_info(),
                            to: self.burn_from.to_account_info(),
                            authority: self.burn_from_authority.clone(),
                        },
                    ),
                    tokens,
                )?;
                self.state.liq_pool.on_lp_burn(tokens)?;
                ::solana_program::log::sol_log("end instruction rem-liq");
                Ok(())
            }
        }
    }
    pub mod set_lp_params {
        use crate::{error::CommonError, Fee, SetLpParams};
        use anchor_lang::prelude::ProgramResult;
        use anchor_lang::solana_program::native_token::sol_to_lamports;
        impl<'info> SetLpParams<'info> {
            fn check_fees(&self, min_fee: Fee, max_fee: Fee) -> ProgramResult {
                min_fee.check()?;
                max_fee.check()?;
                if max_fee.basis_points > 1000 {
                    return Err(CommonError::FeeTooHigh.into());
                }
                if min_fee > max_fee {
                    return Err(CommonError::FeesWrongWayRound.into());
                }
                Ok(())
            }
            fn check_liquidity_target(&self, liquidity_target: u64) -> ProgramResult {
                if liquidity_target < sol_to_lamports(50.0) {
                    Err(CommonError::LiquidityTargetTooLow.into())
                } else {
                    Ok(())
                }
            }
            pub fn process(
                &mut self,
                min_fee: Fee,
                max_fee: Fee,
                liquidity_target: u64,
            ) -> ProgramResult {
                self.state.check_admin_authority(self.admin_authority.key)?;
                self.check_fees(min_fee, max_fee)?;
                self.check_liquidity_target(liquidity_target)?;
                self.state.liq_pool.lp_min_fee = min_fee;
                self.state.liq_pool.lp_max_fee = max_fee;
                self.state.liq_pool.lp_liquidity_target = liquidity_target;
                Ok(())
            }
        }
    }
    pub struct LiqPool {
        pub lp_mint: Pubkey,
        pub lp_mint_authority_bump_seed: u8,
        pub sol_leg_bump_seed: u8,
        pub msol_leg_authority_bump_seed: u8,
        pub msol_leg: Pubkey,
        ///Liquidity target. If the Liquidity reach this amount, the fee reaches lp_min_discount_fee
        pub lp_liquidity_target: u64,
        /// Liquidity pool max fee
        pub lp_max_fee: Fee,
        /// SOL/mSOL Liquidity pool min fee
        pub lp_min_fee: Fee,
        /// Treasury cut
        pub treasury_cut: Fee,
        pub lp_supply: u64,
        pub lent_from_sol_leg: u64,
        pub liquidity_sol_cap: u64,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for LiqPool {
        #[inline]
        fn clone(&self) -> LiqPool {
            match *self {
                Self {
                    lp_mint: ref __self_0_0,
                    lp_mint_authority_bump_seed: ref __self_0_1,
                    sol_leg_bump_seed: ref __self_0_2,
                    msol_leg_authority_bump_seed: ref __self_0_3,
                    msol_leg: ref __self_0_4,
                    lp_liquidity_target: ref __self_0_5,
                    lp_max_fee: ref __self_0_6,
                    lp_min_fee: ref __self_0_7,
                    treasury_cut: ref __self_0_8,
                    lp_supply: ref __self_0_9,
                    lent_from_sol_leg: ref __self_0_10,
                    liquidity_sol_cap: ref __self_0_11,
                } => LiqPool {
                    lp_mint: ::core::clone::Clone::clone(&(*__self_0_0)),
                    lp_mint_authority_bump_seed: ::core::clone::Clone::clone(&(*__self_0_1)),
                    sol_leg_bump_seed: ::core::clone::Clone::clone(&(*__self_0_2)),
                    msol_leg_authority_bump_seed: ::core::clone::Clone::clone(&(*__self_0_3)),
                    msol_leg: ::core::clone::Clone::clone(&(*__self_0_4)),
                    lp_liquidity_target: ::core::clone::Clone::clone(&(*__self_0_5)),
                    lp_max_fee: ::core::clone::Clone::clone(&(*__self_0_6)),
                    lp_min_fee: ::core::clone::Clone::clone(&(*__self_0_7)),
                    treasury_cut: ::core::clone::Clone::clone(&(*__self_0_8)),
                    lp_supply: ::core::clone::Clone::clone(&(*__self_0_9)),
                    lent_from_sol_leg: ::core::clone::Clone::clone(&(*__self_0_10)),
                    liquidity_sol_cap: ::core::clone::Clone::clone(&(*__self_0_11)),
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for LiqPool
    where
        Pubkey: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Fee: borsh::ser::BorshSerialize,
        Fee: borsh::ser::BorshSerialize,
        Fee: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.lp_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_mint_authority_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.sol_leg_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_leg_authority_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_liquidity_target, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_max_fee, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_min_fee, writer)?;
            borsh::BorshSerialize::serialize(&self.treasury_cut, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_supply, writer)?;
            borsh::BorshSerialize::serialize(&self.lent_from_sol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.liquidity_sol_cap, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for LiqPool
    where
        Pubkey: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Fee: borsh::BorshDeserialize,
        Fee: borsh::BorshDeserialize,
        Fee: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                lp_mint: borsh::BorshDeserialize::deserialize(buf)?,
                lp_mint_authority_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                sol_leg_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                msol_leg_authority_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                msol_leg: borsh::BorshDeserialize::deserialize(buf)?,
                lp_liquidity_target: borsh::BorshDeserialize::deserialize(buf)?,
                lp_max_fee: borsh::BorshDeserialize::deserialize(buf)?,
                lp_min_fee: borsh::BorshDeserialize::deserialize(buf)?,
                treasury_cut: borsh::BorshDeserialize::deserialize(buf)?,
                lp_supply: borsh::BorshDeserialize::deserialize(buf)?,
                lent_from_sol_leg: borsh::BorshDeserialize::deserialize(buf)?,
                liquidity_sol_cap: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for LiqPool {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    lp_mint: ref __self_0_0,
                    lp_mint_authority_bump_seed: ref __self_0_1,
                    sol_leg_bump_seed: ref __self_0_2,
                    msol_leg_authority_bump_seed: ref __self_0_3,
                    msol_leg: ref __self_0_4,
                    lp_liquidity_target: ref __self_0_5,
                    lp_max_fee: ref __self_0_6,
                    lp_min_fee: ref __self_0_7,
                    treasury_cut: ref __self_0_8,
                    lp_supply: ref __self_0_9,
                    lent_from_sol_leg: ref __self_0_10,
                    liquidity_sol_cap: ref __self_0_11,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "LiqPool");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_mint",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_mint_authority_bump_seed",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "sol_leg_bump_seed",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_leg_authority_bump_seed",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_leg",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_liquidity_target",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_max_fee",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_min_fee",
                        &&(*__self_0_7),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "treasury_cut",
                        &&(*__self_0_8),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lp_supply",
                        &&(*__self_0_9),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lent_from_sol_leg",
                        &&(*__self_0_10),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "liquidity_sol_cap",
                        &&(*__self_0_11),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl LiqPool {
        pub const LP_MINT_AUTHORITY_SEED: &'static [u8] = b"liq_mint";
        pub const SOL_LEG_SEED: &'static [u8] = b"liq_sol";
        pub const MSOL_LEG_AUTHORITY_SEED: &'static [u8] = b"liq_st_sol_authority";
        pub const MSOL_LEG_SEED: &'static str = "liq_st_sol";
        pub fn find_lp_mint_authority(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(
                &[&state.to_bytes()[..32], Self::LP_MINT_AUTHORITY_SEED],
                &ID,
            )
        }
        pub fn find_sol_leg_address(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[&state.to_bytes()[..32], Self::SOL_LEG_SEED], &ID)
        }
        pub fn find_msol_leg_authority(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(
                &[&state.to_bytes()[..32], Self::MSOL_LEG_AUTHORITY_SEED],
                &ID,
            )
        }
        pub fn default_msol_leg_address(state: &Pubkey) -> Pubkey {
            Pubkey::create_with_seed(state, Self::MSOL_LEG_SEED, &spl_token::ID).unwrap()
        }
        pub fn check_lp_mint(&mut self, lp_mint: &Pubkey) -> ProgramResult {
            check_address(lp_mint, &self.lp_mint, "lp_mint")
        }
        pub fn check_liq_pool_msol_leg(&self, liq_pool_msol_leg: &Pubkey) -> ProgramResult {
            check_address(liq_pool_msol_leg, &self.msol_leg, "liq_pool_msol_leg")
        }
        pub fn delta(&self) -> u32 {
            self.lp_max_fee
                .basis_points
                .saturating_sub(self.lp_min_fee.basis_points)
        }
        ///compute a linear fee based on liquidity amount, it goes from fee(0)=max -> fee(x>=target)=min
        pub fn linear_fee(&self, lamports: u64) -> Fee {
            if lamports >= self.lp_liquidity_target {
                self.lp_min_fee
            } else {
                Fee {
                    basis_points: self.lp_max_fee.basis_points
                        - proportional(self.delta() as u64, lamports, self.lp_liquidity_target)
                            .unwrap() as u32,
                }
            }
        }
        pub fn on_lp_mint(&mut self, amount: u64) {
            self.lp_supply = self
                .lp_supply
                .checked_add(amount)
                .expect("lp_supply overflow");
        }
        pub fn on_lp_burn(&mut self, amount: u64) -> ProgramResult {
            self.lp_supply = self
                .lp_supply
                .checked_sub(amount)
                .ok_or(CommonError::CalculationFailure)?;
            Ok(())
        }
        pub fn check_liquidity_cap(
            &self,
            transfering_lamports: u64,
            sol_leg_balance: u64,
        ) -> ProgramResult {
            let result_amount = sol_leg_balance
                .checked_add(transfering_lamports)
                .ok_or_else(|| {
                    ::solana_program::log::sol_log("SOL overflow");
                    ProgramError::InvalidArgument
                })?;
            if result_amount > self.liquidity_sol_cap {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Liquidity cap reached ", "/"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&result_amount),
                            ::core::fmt::ArgumentV1::new_display(&self.liquidity_sol_cap),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::Custom(3782));
            }
            Ok(())
        }
    }
    pub trait LiqPoolHelpers {
        fn with_lp_mint_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn lp_mint_authority(&self) -> Pubkey;
        fn with_liq_pool_sol_leg_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn liq_pool_sol_leg_address(&self) -> Pubkey;
        fn with_liq_pool_msol_leg_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn liq_pool_msol_leg_authority(&self) -> Pubkey;
        fn check_lp_mint_authority(&self, lp_mint_authority: &Pubkey) -> ProgramResult;
        fn check_liq_pool_sol_leg_pda(&self, liq_pool_sol_leg_pda: &Pubkey) -> ProgramResult;
        fn check_liq_pool_msol_leg_authority(
            &self,
            liq_pool_msol_leg_authority: &Pubkey,
        ) -> ProgramResult;
    }
    impl<T> LiqPoolHelpers for T
    where
        T: Located<State>,
    {
        fn with_lp_mint_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                LiqPool::LP_MINT_AUTHORITY_SEED,
                &[self.as_ref().liq_pool.lp_mint_authority_bump_seed],
            ])
        }
        fn lp_mint_authority(&self) -> Pubkey {
            self.with_lp_mint_authority_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn with_liq_pool_sol_leg_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                LiqPool::SOL_LEG_SEED,
                &[self.as_ref().liq_pool.sol_leg_bump_seed],
            ])
        }
        fn liq_pool_sol_leg_address(&self) -> Pubkey {
            self.with_liq_pool_sol_leg_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn with_liq_pool_msol_leg_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                LiqPool::MSOL_LEG_AUTHORITY_SEED,
                &[self.as_ref().liq_pool.msol_leg_authority_bump_seed],
            ])
        }
        fn liq_pool_msol_leg_authority(&self) -> Pubkey {
            self.with_liq_pool_msol_leg_authority_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn check_lp_mint_authority(&self, lp_mint_authority: &Pubkey) -> ProgramResult {
            check_address(
                lp_mint_authority,
                &self.lp_mint_authority(),
                "lp_mint_authority",
            )
        }
        fn check_liq_pool_sol_leg_pda(&self, liq_pool_sol_leg_pda: &Pubkey) -> ProgramResult {
            check_address(
                liq_pool_sol_leg_pda,
                &self.liq_pool_sol_leg_address(),
                "liq_pool_sol_leg_pda",
            )
        }
        fn check_liq_pool_msol_leg_authority(
            &self,
            liq_pool_msol_leg_authority: &Pubkey,
        ) -> ProgramResult {
            check_address(
                liq_pool_msol_leg_authority,
                &self.liq_pool_msol_leg_authority(),
                "liq_pool_msol_leg_authority",
            )
        }
    }
}
pub mod list {
    use std::io::Cursor;
    use anchor_lang::prelude::*;
    use borsh::BorshSchema;
    use std::convert::TryFrom;
    use crate::error::CommonError;
    pub struct List {
        pub account: Pubkey,
        pub item_size: u32,
        pub count: u32,
        pub new_account: Pubkey,
        pub copied_count: u32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for List {
        #[inline]
        fn default() -> List {
            List {
                account: ::core::default::Default::default(),
                item_size: ::core::default::Default::default(),
                count: ::core::default::Default::default(),
                new_account: ::core::default::Default::default(),
                copied_count: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for List {
        #[inline]
        fn clone(&self) -> List {
            match *self {
                Self {
                    account: ref __self_0_0,
                    item_size: ref __self_0_1,
                    count: ref __self_0_2,
                    new_account: ref __self_0_3,
                    copied_count: ref __self_0_4,
                } => List {
                    account: ::core::clone::Clone::clone(&(*__self_0_0)),
                    item_size: ::core::clone::Clone::clone(&(*__self_0_1)),
                    count: ::core::clone::Clone::clone(&(*__self_0_2)),
                    new_account: ::core::clone::Clone::clone(&(*__self_0_3)),
                    copied_count: ::core::clone::Clone::clone(&(*__self_0_4)),
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for List
    where
        Pubkey: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.account, writer)?;
            borsh::BorshSerialize::serialize(&self.item_size, writer)?;
            borsh::BorshSerialize::serialize(&self.count, writer)?;
            borsh::BorshSerialize::serialize(&self.new_account, writer)?;
            borsh::BorshSerialize::serialize(&self.copied_count, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for List
    where
        Pubkey: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                account: borsh::BorshDeserialize::deserialize(buf)?,
                item_size: borsh::BorshDeserialize::deserialize(buf)?,
                count: borsh::BorshDeserialize::deserialize(buf)?,
                new_account: borsh::BorshDeserialize::deserialize(buf)?,
                copied_count: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl borsh::BorshSchema for List
    where
        Pubkey: borsh::BorshSchema,
        u32: borsh::BorshSchema,
        u32: borsh::BorshSchema,
        Pubkey: borsh::BorshSchema,
        u32: borsh::BorshSchema,
    {
        fn declaration() -> borsh::schema::Declaration {
            "List".to_string()
        }
        fn add_definitions_recursively(
            definitions: &mut borsh::maybestd::collections::HashMap<
                borsh::schema::Declaration,
                borsh::schema::Definition,
            >,
        ) {
            let fields = borsh::schema::Fields::NamedFields(<[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([
                    ("account".to_string(), <Pubkey>::declaration()),
                    ("item_size".to_string(), <u32>::declaration()),
                    ("count".to_string(), <u32>::declaration()),
                    ("new_account".to_string(), <Pubkey>::declaration()),
                    ("copied_count".to_string(), <u32>::declaration()),
                ]),
            ));
            let definition = borsh::schema::Definition::Struct { fields };
            Self::add_definition(Self::declaration(), definition, definitions);
            <Pubkey>::add_definitions_recursively(definitions);
            <u32>::add_definitions_recursively(definitions);
            <u32>::add_definitions_recursively(definitions);
            <Pubkey>::add_definitions_recursively(definitions);
            <u32>::add_definitions_recursively(definitions);
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for List {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    account: ref __self_0_0,
                    item_size: ref __self_0_1,
                    count: ref __self_0_2,
                    new_account: ref __self_0_3,
                    copied_count: ref __self_0_4,
                } => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "List");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "account",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "item_size",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "count",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "new_account",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "copied_count",
                        &&(*__self_0_4),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl List {
        pub fn new(
            discriminator: &[u8; 8],
            item_size: u32,
            account: Pubkey,
            data: &mut [u8],
            list_name: &str,
        ) -> Result<Self, ProgramError> {
            let result = Self {
                account,
                item_size,
                count: 0,
                new_account: Pubkey::default(),
                copied_count: 0,
            };
            result.init_account(discriminator, data, list_name)?;
            Ok(result)
        }
        pub fn bytes_for(item_size: u32, count: u32) -> u32 {
            8 + count * item_size
        }
        pub fn capacity_of(item_size: u32, account_len: usize) -> u32 {
            (account_len as u32 - 8) / item_size
        }
        fn init_account(
            &self,
            discriminator: &[u8; 8],
            data: &mut [u8],
            list_name: &str,
        ) -> ProgramResult {
            match (&self.count, &0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            if data.len() < 8 {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["", " account must have at least 8 bytes of storage"],
                        &[::core::fmt::ArgumentV1::new_display(&list_name)],
                    ));
                    res
                });
                return Err(ProgramError::AccountDataTooSmall);
            }
            if data[0..8] != [0; 8] {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["", " account is already initialized"],
                        &[::core::fmt::ArgumentV1::new_display(&list_name)],
                    ));
                    res
                });
                return Err(ProgramError::AccountAlreadyInitialized);
            }
            data[0..8].copy_from_slice(discriminator);
            Ok(())
        }
        pub fn item_size(&self) -> u32 {
            self.item_size
        }
        pub fn len(&self) -> u32 {
            self.count
        }
        pub fn is_empty(&self) -> bool {
            self.count == 0
        }
        pub fn is_changing_account(&self) -> bool {
            self.new_account != Pubkey::default()
        }
        pub fn capacity(&self, account_len: usize) -> Result<u32, ProgramError> {
            Ok(u32::try_from(
                account_len
                    .checked_sub(8)
                    .ok_or(ProgramError::AccountDataTooSmall)?,
            )
            .map_err(|_| ProgramError::from(CommonError::CalculationFailure))?
            .checked_div(self.item_size())
            .unwrap_or(std::u32::MAX))
        }
        pub fn get<I: AnchorDeserialize>(
            &self,
            data: &[u8],
            index: u32,
            list_name: &str,
        ) -> Result<I, ProgramError> {
            if index >= self.len() {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["list ", " index out of bounds (", "/", ")"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&list_name),
                            ::core::fmt::ArgumentV1::new_display(&index),
                            ::core::fmt::ArgumentV1::new_display(&self.len()),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::InvalidArgument);
            }
            let start = 8 + (index * self.item_size()) as usize;
            I::deserialize(&mut &data[start..(start + self.item_size() as usize)])
                .map_err(|err| ProgramError::BorshIoError(err.to_string()))
        }
        pub fn set<I: AnchorSerialize>(
            &self,
            data: &mut [u8],
            index: u32,
            item: I,
            list_name: &str,
        ) -> ProgramResult {
            if self.new_account != Pubkey::default() {
                ::solana_program::log::sol_log(
                    "Can not modify list {} while changing list's account",
                );
                return Err(ProgramError::InvalidAccountData);
            }
            if index >= self.len() {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["list ", " index out of bounds (", "/", ")"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&list_name),
                            ::core::fmt::ArgumentV1::new_display(&index),
                            ::core::fmt::ArgumentV1::new_display(&self.len()),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::InvalidArgument);
            }
            let start = 8 + (index * self.item_size()) as usize;
            let mut cursor = Cursor::new(&mut data[start..(start + self.item_size() as usize)]);
            item.serialize(&mut cursor)?;
            Ok(())
        }
        pub fn push<I: AnchorSerialize>(
            &mut self,
            data: &mut [u8],
            item: I,
            list_name: &str,
        ) -> ProgramResult {
            if self.new_account != Pubkey::default() {
                ::solana_program::log::sol_log(
                    "Can not modify list {} while changing list's account",
                );
                return Err(ProgramError::InvalidAccountData);
            }
            let capacity = self.capacity(data.len())?;
            if self.len() >= capacity {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["list ", " with capacity ", " is full"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&list_name),
                            ::core::fmt::ArgumentV1::new_display(&capacity),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::AccountDataTooSmall);
            }
            let start = 8 + (self.len() * self.item_size()) as usize;
            let mut cursor = Cursor::new(&mut data[start..(start + self.item_size() as usize)]);
            item.serialize(&mut cursor)?;
            self.count += 1;
            Ok(())
        }
        pub fn remove(&mut self, data: &mut [u8], index: u32, list_name: &str) -> ProgramResult {
            if self.new_account != Pubkey::default() {
                ::solana_program::log::sol_log(
                    "Can not modify list {} while changing list's account",
                );
                return Err(ProgramError::InvalidAccountData);
            }
            if index >= self.len() {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["list ", " remove out of bounds (", "/", ")"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&list_name),
                            ::core::fmt::ArgumentV1::new_display(&index),
                            ::core::fmt::ArgumentV1::new_display(&self.len()),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::InvalidArgument);
            }
            self.count -= 1;
            if index == self.count {
                return Ok(());
            }
            let start = 8 + (index * self.item_size()) as usize;
            let last_item_start = 8 + (self.count * self.item_size()) as usize;
            data.copy_within(
                last_item_start..last_item_start + self.item_size() as usize,
                start,
            );
            Ok(())
        }
    }
}
pub mod located {
    use std::ops::DerefMut;
    use anchor_lang::prelude::*;
    pub trait Located<T> {
        fn as_ref(&self) -> &T;
        fn as_mut(&mut self) -> &mut T;
        fn key(&self) -> Pubkey;
    }
    impl<'info, T, A> Located<T> for A
    where
        A: ToAccountInfo<'info> + DerefMut<Target = T>,
    {
        fn as_ref(&self) -> &T {
            self.deref()
        }
        fn as_mut(&mut self) -> &mut T {
            self.deref_mut()
        }
        fn key(&self) -> Pubkey {
            *self.to_account_info().key
        }
    }
}
pub mod stake_system {
    use crate::{checks::check_address, list::List, located::Located, State, ID};
    use anchor_lang::prelude::*;
    use anchor_lang::solana_program::clock::Epoch;
    pub mod deactivate_stake {
        use crate::error::CommonError;
        use crate::{checks::check_owner_program, stake_system::StakeSystemHelpers};
        use std::convert::TryFrom;
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{
            program::{invoke, invoke_signed},
            stake::program as stake_program,
            stake::{self, state::StakeState},
            system_instruction, system_program,
        };
        use crate::{
            checks::{check_address, check_stake_amount_and_validator},
            state::StateHelpers,
            DeactivateStake,
        };
        impl<'info> DeactivateStake<'info> {
            pub fn process(&mut self, stake_index: u32, validator_index: u32) -> ProgramResult {
                self.state.check_reserve_address(self.reserve_pda.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_owner_program(&self.stake_account, &stake::program::ID, "stake_account")?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                let mut stake = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    stake_index,
                    self.stake_account.to_account_info().key,
                )?;
                if stake.is_emergency_unstaking != 0 {
                    return Err(crate::CommonError::StakeAccountIsEmergencyUnstaking.into());
                }
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                if self.clock.slot
                    < self
                        .epoch_schedule
                        .get_last_slot_in_epoch(self.clock.epoch)
                        .saturating_sub(self.state.stake_system.slots_for_stake_delta)
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Stake delta is available only last ", " slots of epoch"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.state.stake_system.slots_for_stake_delta,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::Custom(332));
                }
                let total_stake_delta_i128 = self.state.stake_delta(self.reserve_pda.lamports());
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["total_stake_delta_i128 "],
                        &[::core::fmt::ArgumentV1::new_display(
                            &total_stake_delta_i128,
                        )],
                    ));
                    res
                });
                if total_stake_delta_i128 >= 0 {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Must stake ", " instead of unstaking"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &total_stake_delta_i128,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let total_unstake_delta =
                    u64::try_from(-total_stake_delta_i128).expect("Unstake delta overflow");
                let total_stake_target = self
                    .state
                    .validator_system
                    .total_active_balance
                    .saturating_sub(total_unstake_delta);
                check_stake_amount_and_validator(
                    &self.stake_account.inner,
                    stake.last_update_delegated_lamports,
                    &validator.validator_account,
                )?;
                let validator_stake_target = self
                    .state
                    .validator_system
                    .validator_stake_target(&validator, total_stake_target)?;
                if validator.active_balance <= validator_stake_target {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Validator ", " has already reached unstake target "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                                ::core::fmt::ArgumentV1::new_display(&validator_stake_target),
                            ],
                        ));
                        res
                    });
                    return Ok(());
                }
                let unstake_from_validator = validator.active_balance - validator_stake_target;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["unstake ", " from_validator "],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&unstake_from_validator),
                            ::core::fmt::ArgumentV1::new_display(&&validator.validator_account),
                        ],
                    ));
                    res
                });
                let stake_account_target = stake.last_update_delegated_lamports.saturating_sub(
                    if unstake_from_validator > total_unstake_delta {
                        total_unstake_delta
                    } else {
                        unstake_from_validator
                    },
                );
                let unstaked_amount = if stake_account_target
                    < 2 * self.state.stake_system.min_stake
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deactivate whole stake "],
                            &[::core::fmt::ArgumentV1::new_display(&stake.stake_account)],
                        ));
                        res
                    });
                    self.state.with_stake_deposit_authority_seeds(|seeds| {
                        invoke_signed(
                            &stake::instruction::deactivate_stake(
                                self.stake_account.to_account_info().key,
                                self.stake_deposit_authority.key,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.clock.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                    if self.split_stake_account.owner == &stake::program::ID {
                        let correct = match bincode::deserialize(
                            &self.split_stake_account.data.as_ref().borrow(),
                        ) {
                            Ok(StakeState::Uninitialized) => true,
                            _ => {
                                ::solana_program::log::sol_log(&{
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Split stake ", " rent return problem"],
                                        &[::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        )],
                                    ));
                                    res
                                });
                                false
                            }
                        };
                        if correct {
                            invoke(
                                &stake::instruction::withdraw(
                                    self.split_stake_account.key,
                                    self.split_stake_account.key,
                                    self.split_stake_rent_payer.key,
                                    self.split_stake_account.lamports(),
                                    None,
                                ),
                                &[
                                    self.stake_program.clone(),
                                    self.split_stake_account.clone(),
                                    self.split_stake_rent_payer.clone(),
                                    self.clock.to_account_info(),
                                    self.stake_history.to_account_info(),
                                ],
                            )?;
                        }
                    }
                    stake.last_update_delegated_lamports
                } else {
                    if validator.last_stake_delta_epoch == self.clock.epoch {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Double delta stake command for validator ", " in epoch "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &validator.validator_account,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&self.clock.epoch),
                                ],
                            ));
                            res
                        });
                        return Ok(());
                    }
                    validator.last_stake_delta_epoch = self.clock.epoch;
                    let split_amount = stake.last_update_delegated_lamports - stake_account_target;
                    if !(stake_account_target < stake.last_update_delegated_lamports
                        && split_amount <= total_unstake_delta)
                    {
                        :: core :: panicking :: panic ("assertion failed: stake_account_target < stake.last_update_delegated_lamports &&\\n    split_amount <= total_unstake_delta")
                    };
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deactivate split ", " (", " lamports) from stake "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.split_stake_account.key),
                                ::core::fmt::ArgumentV1::new_display(&split_amount),
                                ::core::fmt::ArgumentV1::new_display(&stake.stake_account),
                            ],
                        ));
                        res
                    });
                    self.state.stake_system.add(
                        &mut self.stake_list.data.as_ref().borrow_mut(),
                        self.split_stake_account.key,
                        split_amount,
                        &self.clock,
                        0,
                    )?;
                    let stake_accout_len = std::mem::size_of::<StakeState>();
                    if self.split_stake_account.owner == &system_program::ID {
                        invoke(
                            &system_instruction::create_account(
                                self.split_stake_rent_payer.key,
                                self.split_stake_account.key,
                                self.rent.minimum_balance(stake_accout_len),
                                stake_accout_len as u64,
                                &stake_program::ID,
                            ),
                            &[
                                self.system_program.clone(),
                                self.split_stake_rent_payer.clone(),
                                self.split_stake_account.clone(),
                            ],
                        )?;
                    } else {
                        check_owner_program(
                            &self.split_stake_account,
                            &stake::program::ID,
                            "split_stake_account",
                        )?;
                        if self.split_stake_account.data_len() < stake_accout_len {
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &[
                                        "Split stake account ",
                                        " must have at least ",
                                        " bytes (got ",
                                        ")",
                                    ],
                                    &[
                                        ::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        ),
                                        ::core::fmt::ArgumentV1::new_display(&stake_accout_len),
                                        ::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.data_len(),
                                        ),
                                    ],
                                ));
                                res
                            });
                            return Err(ProgramError::InvalidAccountData);
                        }
                        if !self.rent.is_exempt(
                            self.split_stake_account.lamports(),
                            self.split_stake_account.data_len(),
                        ) {
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Split stake account ", " must be rent-exempt"],
                                    &[::core::fmt::ArgumentV1::new_display(
                                        &self.split_stake_account.key,
                                    )],
                                ));
                                res
                            });
                            return Err(ProgramError::InsufficientFunds);
                        }
                        match bincode::deserialize(&self.split_stake_account.data.as_ref().borrow())
                            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?
                        {
                            StakeState::Uninitialized => (),
                            _ => {
                                ::solana_program::log::sol_log(&{
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Split stake ", " must be uninitialized"],
                                        &[::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        )],
                                    ));
                                    res
                                });
                                return Err(ProgramError::InvalidAccountData);
                            }
                        }
                    }
                    self.state.with_stake_deposit_authority_seeds(|seeds| {
                        let split_instruction = stake::instruction::split(
                            self.stake_account.to_account_info().key,
                            self.stake_deposit_authority.key,
                            split_amount,
                            self.split_stake_account.key,
                        )
                        .last()
                        .unwrap()
                        .clone();
                        invoke_signed(
                            &split_instruction,
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.split_stake_account.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )?;
                        invoke_signed(
                            &stake::instruction::deactivate_stake(
                                self.split_stake_account.to_account_info().key,
                                self.stake_deposit_authority.key,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.split_stake_account.to_account_info(),
                                self.clock.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                    stake.last_update_delegated_lamports -= split_amount;
                    split_amount
                };
                validator.active_balance = validator
                    .active_balance
                    .checked_sub(unstaked_amount)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.stake_system.last_stake_delta_epoch = self.clock.epoch;
                self.state.validator_system.total_active_balance = self
                    .state
                    .validator_system
                    .total_active_balance
                    .checked_sub(unstaked_amount)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.stake_system.delayed_unstake_cooling_down = self
                    .state
                    .stake_system
                    .delayed_unstake_cooling_down
                    .checked_add(unstaked_amount)
                    .expect("Cooling down overflow");
                self.state.stake_system.set(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    stake_index,
                    stake,
                )?;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                Ok(())
            }
        }
    }
    pub mod deposit_stake_account {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::stake::instruction::LockupArgs;
        use anchor_lang::solana_program::{
            program::{invoke, invoke_signed},
            stake,
            stake::state::StakeAuthorize,
            system_instruction, system_program,
        };
        use anchor_spl::token::{mint_to, MintTo};
        use crate::error::CommonError;
        use crate::{
            checks::{check_address, check_owner_program, check_token_mint},
            stake_system::StakeSystemHelpers,
            state::StateHelpers,
            DepositStakeAccount, ID,
        };
        impl<'info> DepositStakeAccount<'info> {
            pub const WAIT_EPOCHS: u64 = 2;
            pub fn process(&mut self, validator_index: u32) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .check_msol_mint(self.msol_mint.to_account_info().key)?;
                self.state
                    .check_msol_mint_authority(self.msol_mint_authority.key)?;
                check_owner_program(&self.stake_account, &stake::program::ID, "stake")?;
                check_token_mint(&self.mint_to, self.state.msol_mint, "mint_to")?;
                check_owner_program(&self.rent_payer, &system_program::ID, "rent_payer")?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(
                    self.token_program.to_account_info().key,
                    &spl_token::ID,
                    "token_program",
                )?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                if self.msol_mint.supply > self.state.msol_supply {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Warning: mSOL minted ", " lamports outside of marinade"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &(self.msol_mint.supply - self.state.msol_supply),
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let delegation = self.stake_account.delegation().ok_or_else(|| {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deposited stake ", " must be delegated"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.stake_account.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    ProgramError::InvalidAccountData
                })?;
                if delegation.deactivation_epoch != std::u64::MAX {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deposited stake ", " must not be cooling down"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.stake_account.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                if self.clock.epoch
                    < delegation
                        .activation_epoch
                        .checked_add(Self::WAIT_EPOCHS)
                        .unwrap()
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Deposited stake ",
                                " is not activated yet. Wait for #",
                                " epoch",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.stake_account.to_account_info().key,
                                ),
                                ::core::fmt::ArgumentV1::new_display(
                                    &delegation
                                        .activation_epoch
                                        .checked_add(Self::WAIT_EPOCHS)
                                        .unwrap(),
                                ),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                if delegation.stake < self.state.stake_system.min_stake {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Deposited stake ",
                                " has low amount of lamports ",
                                ". Need at least ",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.stake_account.to_account_info().key,
                                ),
                                ::core::fmt::ArgumentV1::new_display(&delegation.stake),
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.state.stake_system.min_stake,
                                ),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InsufficientFunds);
                }
                if self.stake_account.to_account_info().lamports()
                    > delegation.stake + self.stake_account.meta().unwrap().rent_exempt_reserve
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Stake account has ",
                                " extra lamports. Please withdraw it and try again",
                            ],
                            &[::core::fmt::ArgumentV1::new_display(
                                &(self.stake_account.to_account_info().lamports()
                                    - (delegation.stake
                                        + self.stake_account.meta().unwrap().rent_exempt_reserve)),
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::Custom(6212));
                }
                self.state.check_staking_cap(delegation.stake)?;
                let lockup = self.stake_account.lockup().unwrap();
                if lockup.is_in_force(&self.clock, None) {
                    ::solana_program::log::sol_log("Can not deposit stake account with lockup");
                    return Err(CommonError::AccountWithLockup.into());
                }
                if validator_index == self.state.validator_system.validator_count() {
                    if self.state.validator_system.auto_add_validator_enabled == 0 {
                        return Err(CommonError::InvalidValidator.into());
                    }
                    check_owner_program(
                        &self.duplication_flag,
                        &system_program::ID,
                        "duplication_flag",
                    )?;
                    if !self.rent.is_exempt(self.rent_payer.lamports(), 0) {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Rent payer must have at least ", " lamports"],
                                &[::core::fmt::ArgumentV1::new_display(
                                    &self.rent.minimum_balance(0),
                                )],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                    let state_address = *self.state.to_account_info().key;
                    self.state.validator_system.add_with_balance(
                        &mut self.validator_list.data.as_ref().borrow_mut(),
                        delegation.voter_pubkey,
                        0,
                        delegation.stake,
                        &state_address,
                        self.duplication_flag.key,
                    )?;
                    let validator_record = self.state.validator_system.get(
                        &self.validator_list.data.as_ref().borrow(),
                        self.state.validator_system.validator_count() - 1,
                    )?;
                    validator_record.with_duplication_flag_seeds(
                        self.state.to_account_info().key,
                        |seeds| {
                            invoke_signed(
                                &system_instruction::create_account(
                                    self.rent_payer.key,
                                    self.duplication_flag.key,
                                    self.rent.minimum_balance(0),
                                    0,
                                    &ID,
                                ),
                                &[
                                    self.system_program.clone(),
                                    self.rent_payer.clone(),
                                    self.duplication_flag.clone(),
                                ],
                                &[seeds],
                            )
                        },
                    )?;
                } else {
                    let mut validator = self
                        .state
                        .validator_system
                        .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                    if delegation.voter_pubkey != validator.validator_account {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &[
                                    "Deposited stake ",
                                    " is delegated to ",
                                    " but must be delegated to validator ",
                                    ". Probably validator list is changed",
                                ],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.stake_account.to_account_info().key,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&delegation.voter_pubkey),
                                    ::core::fmt::ArgumentV1::new_display(
                                        &validator.validator_account,
                                    ),
                                ],
                            ));
                            res
                        });
                        return Err(CommonError::InvalidValidator.into());
                    }
                    validator.active_balance = validator
                        .active_balance
                        .checked_add(delegation.stake)
                        .ok_or(CommonError::CalculationFailure)?;
                    self.state.validator_system.set(
                        &mut self.validator_list.data.as_ref().borrow_mut(),
                        validator_index,
                        validator,
                    )?;
                }
                {
                    let new_staker = self.state.stake_deposit_authority();
                    let old_staker = self.stake_account.meta().unwrap().authorized.staker;
                    if old_staker == new_staker {
                        ::solana_program::log::sol_log(&{
                            let res = :: alloc :: fmt :: format (:: core :: fmt :: Arguments :: new_v1 (& ["Can not deposited stake " , " already under marinade control. Expected staker differs from "] , & [:: core :: fmt :: ArgumentV1 :: new_display (& self . stake_account . to_account_info () . key) , :: core :: fmt :: ArgumentV1 :: new_display (& new_staker)])) ;
                            res
                        });
                        return Err(ProgramError::InvalidAccountData);
                    }
                    if lockup.custodian != Pubkey::default() {
                        invoke(
                            &stake::instruction::set_lockup(
                                &self.stake_account.key(),
                                &LockupArgs {
                                    unix_timestamp: Some(0),
                                    epoch: Some(0),
                                    custodian: Some(Pubkey::default()),
                                },
                                self.stake_authority.key,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.stake_authority.clone(),
                            ],
                        )?;
                    }
                    invoke(
                        &stake::instruction::authorize(
                            self.stake_account.to_account_info().key,
                            self.stake_authority.key,
                            &new_staker,
                            StakeAuthorize::Staker,
                            None,
                        ),
                        &[
                            self.stake_program.clone(),
                            self.stake_account.to_account_info(),
                            self.clock.to_account_info(),
                            self.stake_authority.clone(),
                        ],
                    )?;
                }
                {
                    let new_withdrawer = self.state.stake_withdraw_authority();
                    let old_withdrawer = self.stake_account.meta().unwrap().authorized.withdrawer;
                    if old_withdrawer == new_withdrawer {
                        ::solana_program::log::sol_log(&{
                            let res = :: alloc :: fmt :: format (:: core :: fmt :: Arguments :: new_v1 (& ["Can not deposited stake " , " already under marinade control. Expected withdrawer differs from "] , & [:: core :: fmt :: ArgumentV1 :: new_display (& self . stake_account . to_account_info () . key) , :: core :: fmt :: ArgumentV1 :: new_display (& new_withdrawer)])) ;
                            res
                        });
                        return Err(ProgramError::InvalidAccountData);
                    }
                    invoke(
                        &stake::instruction::authorize(
                            self.stake_account.to_account_info().key,
                            self.stake_authority.key,
                            &new_withdrawer,
                            StakeAuthorize::Withdrawer,
                            None,
                        ),
                        &[
                            self.stake_program.clone(),
                            self.stake_account.to_account_info(),
                            self.clock.to_account_info(),
                            self.stake_authority.clone(),
                        ],
                    )?;
                }
                self.state.stake_system.add(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    self.stake_account.to_account_info().key,
                    delegation.stake,
                    &self.clock,
                    0,
                )?;
                let msol_to_mint = self.state.calc_msol_from_lamports(delegation.stake)?;
                self.state.with_msol_mint_authority_seeds(|mint_seeds| {
                    mint_to(
                        CpiContext::new_with_signer(
                            self.token_program.clone(),
                            MintTo {
                                mint: self.msol_mint.to_account_info(),
                                to: self.mint_to.to_account_info(),
                                authority: self.msol_mint_authority.clone(),
                            },
                            &[mint_seeds],
                        ),
                        msol_to_mint,
                    )
                })?;
                self.state.on_msol_mint(msol_to_mint);
                self.state.validator_system.total_active_balance = self
                    .state
                    .validator_system
                    .total_active_balance
                    .checked_add(delegation.stake)
                    .ok_or(CommonError::CalculationFailure)?;
                Ok(())
            }
        }
    }
    pub mod emergency_unstake {
        use crate::{
            checks::{check_owner_program, check_stake_amount_and_validator},
            error::CommonError,
            stake_system::StakeSystemHelpers,
        };
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke_signed, stake::self};
        use crate::{checks::check_address, EmergencyUnstake};
        impl<'info> EmergencyUnstake<'info> {
            pub fn process(&mut self, stake_index: u32, validator_index: u32) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.validator_manager_authority.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_owner_program(&self.stake_account, &stake::program::ID, "stake_account")?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                let mut stake = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    stake_index,
                    self.stake_account.to_account_info().key,
                )?;
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                if validator.score != 0 {
                    ::solana_program::log::sol_log("Emergency unstake validator must have 0 score");
                    return Err(ProgramError::InvalidAccountData);
                }
                check_stake_amount_and_validator(
                    &self.stake_account.inner,
                    stake.last_update_delegated_lamports,
                    &validator.validator_account,
                )?;
                let unstake_amount = stake.last_update_delegated_lamports;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Deactivate whole stake "],
                        &[::core::fmt::ArgumentV1::new_display(&stake.stake_account)],
                    ));
                    res
                });
                self.state.with_stake_deposit_authority_seeds(|seeds| {
                    invoke_signed(
                        &stake::instruction::deactivate_stake(
                            self.stake_account.to_account_info().key,
                            self.stake_deposit_authority.key,
                        ),
                        &[
                            self.stake_program.clone(),
                            self.stake_account.to_account_info(),
                            self.clock.to_account_info(),
                            self.stake_deposit_authority.clone(),
                        ],
                        &[seeds],
                    )
                })?;
                if stake.is_emergency_unstaking != 0 {
                    return Err(crate::CommonError::StakeAccountIsEmergencyUnstaking.into());
                }
                stake.is_emergency_unstaking = 1;
                validator.active_balance = validator
                    .active_balance
                    .checked_sub(unstake_amount)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.validator_system.total_active_balance = self
                    .state
                    .validator_system
                    .total_active_balance
                    .checked_sub(unstake_amount)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.emergency_cooling_down = self
                    .state
                    .emergency_cooling_down
                    .checked_add(unstake_amount)
                    .expect("Cooling down overflow");
                self.state.stake_system.set(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    stake_index,
                    stake,
                )?;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                Ok(())
            }
        }
    }
    pub mod merge {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{
            program::invoke_signed,
            stake::{self, state::StakeState},
        };
        use crate::{
            checks::{check_address, check_owner_program},
            error::CommonError,
            stake_system::StakeSystemHelpers,
            MergeStakes,
        };
        impl<'info> MergeStakes<'info> {
            pub fn process(
                &mut self,
                destination_stake_index: u32,
                source_stake_index: u32,
                validator_index: u32,
            ) -> ProgramResult {
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                check_owner_program(
                    &self.destination_stake,
                    &stake::program::ID,
                    "destination_stake",
                )?;
                check_owner_program(&self.source_stake, &stake::program::ID, "source_stake")?;
                self.state.check_stake_deposit_authority(
                    self.stake_deposit_authority.to_account_info().key,
                )?;
                self.state.check_stake_withdraw_authority(
                    self.stake_withdraw_authority.to_account_info().key,
                )?;
                self.state
                    .check_operational_sol_account(self.operational_sol_account.key)?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                let mut destination_stake_info = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    destination_stake_index,
                    self.destination_stake.to_account_info().key,
                )?;
                let destination_delegation =
                    if let Some(delegation) = self.destination_stake.delegation() {
                        delegation
                    } else {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Destination stake ", " must be delegated"],
                                &[::core::fmt::ArgumentV1::new_display(
                                    &self.destination_stake.to_account_info().key,
                                )],
                            ));
                            res
                        });
                        return Err(ProgramError::InvalidArgument);
                    };
                if destination_delegation.deactivation_epoch != std::u64::MAX {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Destination stake ", " must not be deactivating"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.destination_stake.to_account_info().key,
                            )],
                        ));
                        res
                    });
                }
                if destination_stake_info.last_update_delegated_lamports
                    != destination_delegation.stake
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Destination stake ", " is not updated"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.destination_stake.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                if destination_delegation.voter_pubkey != validator.validator_account {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Destination validator ", " doesn\'t match "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(
                                    &destination_delegation.voter_pubkey,
                                ),
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                let source_stake_info = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    source_stake_index,
                    self.source_stake.to_account_info().key,
                )?;
                let source_delegation = if let Some(delegation) = self.source_stake.delegation() {
                    delegation
                } else {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Source stake ", " must be delegated"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.source_stake.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                };
                if source_delegation.deactivation_epoch != std::u64::MAX {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Source stake ", " must not be deactivating"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.source_stake.to_account_info().key,
                            )],
                        ));
                        res
                    });
                }
                if source_stake_info.last_update_delegated_lamports != source_delegation.stake
                    || self.source_stake.to_account_info().lamports()
                        != source_delegation
                            .stake
                            .checked_add(self.source_stake.meta().unwrap().rent_exempt_reserve)
                            .ok_or(CommonError::CalculationFailure)?
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Source stake ", " is not updated"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.source_stake.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                if source_delegation.voter_pubkey != validator.validator_account {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Source validator ", " doesn\'t match "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(
                                    &source_delegation.voter_pubkey,
                                ),
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                self.state.with_stake_deposit_authority_seeds(|seeds| {
                    invoke_signed(
                        &stake::instruction::merge(
                            self.destination_stake.to_account_info().key,
                            self.source_stake.to_account_info().key,
                            self.stake_deposit_authority.to_account_info().key,
                        )[0],
                        &[
                            self.stake_program.clone(),
                            self.destination_stake.to_account_info(),
                            self.source_stake.to_account_info(),
                            self.clock.to_account_info(),
                            self.stake_history.to_account_info(),
                            self.stake_deposit_authority.to_account_info(),
                        ],
                        &[seeds],
                    )
                })?;
                let result_stake: StakeState = self
                    .destination_stake
                    .to_account_info()
                    .deserialize_data()
                    .map_err(|err| ProgramError::BorshIoError(err.to_string()))?;
                let extra_delegated = result_stake
                    .delegation()
                    .unwrap()
                    .stake
                    .checked_sub(destination_stake_info.last_update_delegated_lamports)
                    .ok_or(CommonError::CalculationFailure)?
                    .checked_sub(source_stake_info.last_update_delegated_lamports)
                    .ok_or(CommonError::CalculationFailure)?;
                let returned_stake_rent = self
                    .source_stake
                    .meta()
                    .unwrap()
                    .rent_exempt_reserve
                    .checked_sub(extra_delegated)
                    .ok_or(CommonError::CalculationFailure)?;
                validator.active_balance = validator
                    .active_balance
                    .checked_add(extra_delegated)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                self.state.validator_system.total_active_balance = self
                    .state
                    .validator_system
                    .total_active_balance
                    .checked_add(extra_delegated)
                    .ok_or(CommonError::CalculationFailure)?;
                destination_stake_info.last_update_delegated_lamports =
                    result_stake.delegation().unwrap().stake;
                self.state.stake_system.set(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    destination_stake_index,
                    destination_stake_info,
                )?;
                self.state.stake_system.remove(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    source_stake_index,
                )?;
                if returned_stake_rent > 0 {
                    self.state.with_stake_withdraw_authority_seeds(|seeds| {
                        invoke_signed(
                            &stake::instruction::withdraw(
                                self.destination_stake.to_account_info().key,
                                self.stake_withdraw_authority.key,
                                self.operational_sol_account.key,
                                returned_stake_rent,
                                None,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.destination_stake.to_account_info(),
                                self.operational_sol_account.clone(),
                                self.clock.to_account_info(),
                                self.stake_history.to_account_info(),
                                self.stake_withdraw_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                }
                if extra_delegated > 0 {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Extra delegation of ",
                                " lamports. TODO: mint some mSOLs for admin in return",
                            ],
                            &[::core::fmt::ArgumentV1::new_display(&extra_delegated)],
                        ));
                        res
                    });
                }
                Ok(())
            }
        }
    }
    pub mod partial_unstake {
        use crate::{
            checks::{check_owner_program, check_stake_amount_and_validator},
            stake_system::StakeSystemHelpers,
        };
        use std::convert::TryFrom;
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{
            program::{invoke, invoke_signed},
            stake::program as stake_program,
            stake::{self, state::StakeState},
            system_instruction, system_program,
        };
        use crate::{checks::check_address, PartialUnstake};
        impl<'info> PartialUnstake<'info> {
            pub fn process(
                &mut self,
                stake_index: u32,
                validator_index: u32,
                desired_unstake_amount: u64,
            ) -> ProgramResult {
                if !(desired_unstake_amount >= self.state.stake_system.min_stake) {
                    {
                        ::std::rt::begin_panic("desired_unstake_amount too low")
                    }
                };
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.validator_manager_authority.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_owner_program(&self.stake_account, &stake::program::ID, "stake_account")?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                let mut stake = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    stake_index,
                    self.stake_account.to_account_info().key,
                )?;
                if stake.is_emergency_unstaking != 0 {
                    return Err(crate::CommonError::StakeAccountIsEmergencyUnstaking.into());
                }
                check_stake_amount_and_validator(
                    &self.stake_account.inner,
                    stake.last_update_delegated_lamports,
                    &validator.validator_account,
                )?;
                let total_stake_delta_i128 = self.state.stake_delta(self.reserve_pda.lamports());
                let total_stake_target_i128 = self.state.validator_system.total_active_balance
                    as i128
                    + total_stake_delta_i128;
                let total_stake_target =
                    u64::try_from(total_stake_target_i128).expect("total_stake_target+stake_delta");
                let validator_stake_target = self
                    .state
                    .validator_system
                    .validator_stake_target(&validator, total_stake_target)?;
                if validator.active_balance
                    <= validator_stake_target + self.state.stake_system.min_stake
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Current validator ",
                                " stake ",
                                " is <= target ",
                                " +min_stake",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                                ::core::fmt::ArgumentV1::new_display(&validator.active_balance),
                                ::core::fmt::ArgumentV1::new_display(&validator_stake_target),
                            ],
                        ));
                        res
                    });
                    return Ok(());
                }
                let max_unstake_from_validator = validator.active_balance - validator_stake_target;
                let unstake_amount = if desired_unstake_amount > max_unstake_from_validator {
                    max_unstake_from_validator
                } else {
                    desired_unstake_amount
                };
                let stake_account_after = stake
                    .last_update_delegated_lamports
                    .saturating_sub(unstake_amount);
                let unstaked_from_account = if stake_account_after
                    < self.state.stake_system.min_stake
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deactivate whole stake "],
                            &[::core::fmt::ArgumentV1::new_display(&stake.stake_account)],
                        ));
                        res
                    });
                    self.state.with_stake_deposit_authority_seeds(|seeds| {
                        invoke_signed(
                            &stake::instruction::deactivate_stake(
                                self.stake_account.to_account_info().key,
                                self.stake_deposit_authority.key,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.clock.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                    stake.is_emergency_unstaking = 1;
                    if self.split_stake_account.owner == &stake::program::ID {
                        let correct = match bincode::deserialize(
                            &self.split_stake_account.data.as_ref().borrow(),
                        ) {
                            Ok(StakeState::Uninitialized) => true,
                            _ => {
                                ::solana_program::log::sol_log(&{
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Split stake ", " rent return problem"],
                                        &[::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        )],
                                    ));
                                    res
                                });
                                false
                            }
                        };
                        if correct {
                            invoke(
                                &stake::instruction::withdraw(
                                    self.split_stake_account.key,
                                    self.split_stake_account.key,
                                    self.split_stake_rent_payer.key,
                                    self.split_stake_account.lamports(),
                                    None,
                                ),
                                &[
                                    self.stake_program.clone(),
                                    self.split_stake_account.clone(),
                                    self.split_stake_rent_payer.clone(),
                                    self.clock.to_account_info(),
                                    self.stake_history.to_account_info(),
                                ],
                            )?;
                        }
                    }
                    stake.last_update_delegated_lamports
                } else {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Deactivate split ", " (", " lamports) from stake "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.split_stake_account.key),
                                ::core::fmt::ArgumentV1::new_display(&unstake_amount),
                                ::core::fmt::ArgumentV1::new_display(&stake.stake_account),
                            ],
                        ));
                        res
                    });
                    self.state.stake_system.add(
                        &mut self.stake_list.data.as_ref().borrow_mut(),
                        self.split_stake_account.key,
                        unstake_amount,
                        &self.clock,
                        1,
                    )?;
                    let stake_account_len = std::mem::size_of::<StakeState>();
                    if self.split_stake_account.owner == &system_program::ID {
                        invoke(
                            &system_instruction::create_account(
                                self.split_stake_rent_payer.key,
                                self.split_stake_account.key,
                                self.rent.minimum_balance(stake_account_len),
                                stake_account_len as u64,
                                &stake_program::ID,
                            ),
                            &[
                                self.system_program.clone(),
                                self.split_stake_rent_payer.clone(),
                                self.split_stake_account.clone(),
                            ],
                        )?;
                    } else {
                        check_owner_program(
                            &self.split_stake_account,
                            &stake::program::ID,
                            "split_stake_account",
                        )?;
                        if self.split_stake_account.data_len() < stake_account_len {
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &[
                                        "Split stake account ",
                                        " must have at least ",
                                        " bytes (got ",
                                        ")",
                                    ],
                                    &[
                                        ::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        ),
                                        ::core::fmt::ArgumentV1::new_display(&stake_account_len),
                                        ::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.data_len(),
                                        ),
                                    ],
                                ));
                                res
                            });
                            return Err(ProgramError::InvalidAccountData);
                        }
                        if !self.rent.is_exempt(
                            self.split_stake_account.lamports(),
                            self.split_stake_account.data_len(),
                        ) {
                            ::solana_program::log::sol_log(&{
                                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                    &["Split stake account ", " must be rent-exempt"],
                                    &[::core::fmt::ArgumentV1::new_display(
                                        &self.split_stake_account.key,
                                    )],
                                ));
                                res
                            });
                            return Err(ProgramError::InsufficientFunds);
                        }
                        match bincode::deserialize(&self.split_stake_account.data.as_ref().borrow())
                            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?
                        {
                            StakeState::Uninitialized => (),
                            _ => {
                                ::solana_program::log::sol_log(&{
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Split stake ", " must be uninitialized"],
                                        &[::core::fmt::ArgumentV1::new_display(
                                            &self.split_stake_account.key,
                                        )],
                                    ));
                                    res
                                });
                                return Err(ProgramError::InvalidAccountData);
                            }
                        }
                    }
                    self.state.with_stake_deposit_authority_seeds(|seeds| {
                        let split_instruction = stake::instruction::split(
                            self.stake_account.to_account_info().key,
                            self.stake_deposit_authority.key,
                            unstake_amount,
                            self.split_stake_account.key,
                        )
                        .last()
                        .unwrap()
                        .clone();
                        invoke_signed(
                            &split_instruction,
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.split_stake_account.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )?;
                        invoke_signed(
                            &stake::instruction::deactivate_stake(
                                self.split_stake_account.to_account_info().key,
                                self.stake_deposit_authority.key,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.split_stake_account.to_account_info(),
                                self.clock.to_account_info(),
                                self.stake_deposit_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                    stake.last_update_delegated_lamports -= unstake_amount;
                    unstake_amount
                };
                validator.active_balance -= unstaked_from_account;
                self.state.validator_system.total_active_balance -= unstaked_from_account;
                self.state.emergency_cooling_down += unstaked_from_account;
                self.state.stake_system.set(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    stake_index,
                    stake,
                )?;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                Ok(())
            }
        }
    }
    pub mod stake_reserve {
        use crate::{
            checks::{check_address, check_owner_program},
            error::CommonError,
            stake_system::StakeSystemHelpers,
            stake_wrapper::StakeWrapper,
            state::StateHelpers,
            StakeReserve,
        };
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{
            log::sol_log_compute_units,
            program::{invoke, invoke_signed},
            stake::{
                self,
                state::{Authorized, Lockup, StakeState},
            },
            system_instruction, system_program,
            sysvar::stake_history,
        };
        use std::convert::TryFrom;
        use std::ops::Deref;
        impl<'info> StakeReserve<'info> {
            fn check_stake_history(&self) -> ProgramResult {
                if !stake_history::check_id(self.stake_history.key) {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Stake history sysvar must be ", ". Got "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&stake_history::ID),
                                ::core::fmt::ArgumentV1::new_display(&self.stake_history.key),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                Ok(())
            }
            ///
            /// called by the bot
            /// Receives self.stake_account where to stake, normally an empty account (new keypair)
            /// stakes from available delta-stake in data.validator_index
            /// pub fn stake_reserve()
            pub fn process(&mut self, validator_index: u32) -> ProgramResult {
                sol_log_compute_units();
                ::solana_program::log::sol_log("Stake reserve");
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state.check_reserve_address(self.reserve_pda.key)?;
                self.check_stake_history()?;
                self.state
                    .check_stake_deposit_authority(self.stake_deposit_authority.key)?;
                check_owner_program(&self.stake_account, &stake::program::ID, "stake")?;
                match StakeWrapper::deref(&self.stake_account) {
                    StakeState::Uninitialized => (),
                    _ => {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Stake ", " must be uninitialized"],
                                &[::core::fmt::ArgumentV1::new_display(
                                    &self.stake_account.key(),
                                )],
                            ));
                            res
                        });
                        return Err(ProgramError::InvalidAccountData);
                    }
                }
                if self.stake_account.to_account_info().lamports()
                    != StakeState::get_rent_exempt_reserve(&self.rent)
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Stake ", " must have balance ", " but has ", " lamports"],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.stake_account.key()),
                                ::core::fmt::ArgumentV1::new_display(
                                    &StakeState::get_rent_exempt_reserve(&self.rent),
                                ),
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.stake_account.to_account_info().lamports(),
                                ),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                check_address(self.stake_config.key, &stake::config::ID, "stake_config")?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                let staker = self.state.stake_deposit_authority();
                let withdrawer = self.state.stake_withdraw_authority();
                let stake_delta = self.state.stake_delta(self.reserve_pda.lamports());
                if stake_delta <= 0 {
                    if stake_delta < 0 {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Must unstake ", " instead of staking"],
                                &[::core::fmt::ArgumentV1::new_display(
                                    &u64::try_from(-stake_delta).expect("Stake delta overflow"),
                                )],
                            ));
                            res
                        });
                    } else {
                        ::solana_program::log::sol_log("Noting to do");
                    }
                    return Ok(());
                }
                let stake_delta = u64::try_from(stake_delta).expect("Stake delta overflow");
                let total_stake_target = self
                    .state
                    .validator_system
                    .total_active_balance
                    .saturating_add(stake_delta);
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                check_address(
                    &self.validator_vote.key,
                    &validator.validator_account,
                    "validator_vote",
                )?;
                if validator.last_stake_delta_epoch == self.clock.epoch {
                    if self.state.stake_system.extra_stake_delta_runs == 0 {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Double delta stake command for validator ", " in epoch "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &validator.validator_account,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&self.clock.epoch),
                                ],
                            ));
                            res
                        });
                        return Ok(());
                    } else {
                        self.state.stake_system.extra_stake_delta_runs -= 1;
                    }
                } else {
                    validator.last_stake_delta_epoch = self.clock.epoch;
                }
                let last_slot = self.epoch_schedule.get_last_slot_in_epoch(self.clock.epoch);
                if self.clock.slot
                    < last_slot.saturating_sub(self.state.stake_system.slots_for_stake_delta)
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Stake delta is available only last ", " slots of epoch"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.state.stake_system.slots_for_stake_delta,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::Custom(332));
                }
                let validator_stake_target = self
                    .state
                    .validator_system
                    .validator_stake_target(&validator, total_stake_target)?;
                if validator.active_balance >= validator_stake_target {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Validator ",
                                " has already reached stake target ",
                                ". Please stake into another validator",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                                ::core::fmt::ArgumentV1::new_display(&validator_stake_target),
                            ],
                        ));
                        res
                    });
                    return Ok(());
                }
                let stake_target = validator_stake_target
                    .saturating_sub(validator.active_balance)
                    .max(self.state.stake_system.min_stake)
                    .min(stake_delta);
                let stake_target = if stake_delta - stake_target < self.state.stake_system.min_stake
                {
                    stake_delta
                } else {
                    stake_target
                };
                self.state.with_reserve_seeds(|seeds| {
                    sol_log_compute_units();
                    ::solana_program::log::sol_log("Transfer to stake account");
                    invoke_signed(
                        &system_instruction::transfer(
                            self.reserve_pda.key,
                            &self.stake_account.key(),
                            stake_target,
                        ),
                        &[
                            self.system_program.clone(),
                            self.reserve_pda.clone(),
                            self.stake_account.to_account_info(),
                        ],
                        &[seeds],
                    )
                })?;
                self.state.on_transfer_from_reserve(stake_target)?;
                sol_log_compute_units();
                ::solana_program::log::sol_log("Initialize stake");
                invoke(
                    &stake::instruction::initialize(
                        &self.stake_account.key(),
                        &Authorized { staker, withdrawer },
                        &Lockup::default(),
                    ),
                    &[
                        self.stake_program.clone(),
                        self.stake_account.to_account_info(),
                        self.rent.to_account_info(),
                    ],
                )?;
                self.state.with_stake_deposit_authority_seeds(|seeds| {
                    sol_log_compute_units();
                    ::solana_program::log::sol_log("Delegate stake");
                    invoke_signed(
                        &stake::instruction::delegate_stake(
                            &self.stake_account.key(),
                            &staker,
                            self.validator_vote.key,
                        ),
                        &[
                            self.stake_program.clone(),
                            self.stake_account.to_account_info(),
                            self.stake_deposit_authority.clone(),
                            self.validator_vote.clone(),
                            self.clock.to_account_info(),
                            self.stake_history.clone(),
                            self.stake_config.to_account_info(),
                        ],
                        &[seeds],
                    )
                })?;
                self.state.stake_system.add(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    &self.stake_account.key(),
                    stake_target,
                    &self.clock,
                    0,
                )?;
                validator.active_balance = validator
                    .active_balance
                    .checked_add(stake_target)
                    .ok_or(CommonError::CalculationFailure)?;
                validator.last_stake_delta_epoch = self.clock.epoch;
                self.state.stake_system.last_stake_delta_epoch = self.clock.epoch;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                self.state.validator_system.total_active_balance = self
                    .state
                    .validator_system
                    .total_active_balance
                    .checked_add(stake_target)
                    .ok_or(CommonError::CalculationFailure)?;
                Ok(())
            }
        }
    }
    pub struct StakeRecord {
        pub stake_account: Pubkey,
        pub last_update_delegated_lamports: u64,
        pub last_update_epoch: u64,
        pub is_emergency_unstaking: u8,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for StakeRecord {
        #[inline]
        fn clone(&self) -> StakeRecord {
            {
                let _: ::core::clone::AssertParamIsClone<Pubkey>;
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for StakeRecord {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for StakeRecord {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    stake_account: ref __self_0_0,
                    last_update_delegated_lamports: ref __self_0_1,
                    last_update_epoch: ref __self_0_2,
                    is_emergency_unstaking: ref __self_0_3,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "StakeRecord");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stake_account",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "last_update_delegated_lamports",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "last_update_epoch",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "is_emergency_unstaking",
                        &&(*__self_0_3),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for StakeRecord {
        #[inline]
        fn default() -> StakeRecord {
            StakeRecord {
                stake_account: ::core::default::Default::default(),
                last_update_delegated_lamports: ::core::default::Default::default(),
                last_update_epoch: ::core::default::Default::default(),
                is_emergency_unstaking: ::core::default::Default::default(),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for StakeRecord {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for StakeRecord {
        #[inline]
        fn eq(&self, other: &StakeRecord) -> bool {
            match *other {
                Self {
                    stake_account: ref __self_1_0,
                    last_update_delegated_lamports: ref __self_1_1,
                    last_update_epoch: ref __self_1_2,
                    is_emergency_unstaking: ref __self_1_3,
                } => match *self {
                    Self {
                        stake_account: ref __self_0_0,
                        last_update_delegated_lamports: ref __self_0_1,
                        last_update_epoch: ref __self_0_2,
                        is_emergency_unstaking: ref __self_0_3,
                    } => {
                        (*__self_0_0) == (*__self_1_0)
                            && (*__self_0_1) == (*__self_1_1)
                            && (*__self_0_2) == (*__self_1_2)
                            && (*__self_0_3) == (*__self_1_3)
                    }
                },
            }
        }
        #[inline]
        fn ne(&self, other: &StakeRecord) -> bool {
            match *other {
                Self {
                    stake_account: ref __self_1_0,
                    last_update_delegated_lamports: ref __self_1_1,
                    last_update_epoch: ref __self_1_2,
                    is_emergency_unstaking: ref __self_1_3,
                } => match *self {
                    Self {
                        stake_account: ref __self_0_0,
                        last_update_delegated_lamports: ref __self_0_1,
                        last_update_epoch: ref __self_0_2,
                        is_emergency_unstaking: ref __self_0_3,
                    } => {
                        (*__self_0_0) != (*__self_1_0)
                            || (*__self_0_1) != (*__self_1_1)
                            || (*__self_0_2) != (*__self_1_2)
                            || (*__self_0_3) != (*__self_1_3)
                    }
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for StakeRecord
    where
        Pubkey: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.last_update_delegated_lamports, writer)?;
            borsh::BorshSerialize::serialize(&self.last_update_epoch, writer)?;
            borsh::BorshSerialize::serialize(&self.is_emergency_unstaking, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for StakeRecord
    where
        Pubkey: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_account: borsh::BorshDeserialize::deserialize(buf)?,
                last_update_delegated_lamports: borsh::BorshDeserialize::deserialize(buf)?,
                last_update_epoch: borsh::BorshDeserialize::deserialize(buf)?,
                is_emergency_unstaking: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl StakeRecord {
        pub const DISCRIMINATOR: &'static [u8; 8] = b"staker__";
        pub fn new(
            stake_account: &Pubkey,
            delegated_lamports: u64,
            clock: &Clock,
            is_emergency_unstaking: u8,
        ) -> Self {
            Self {
                stake_account: *stake_account,
                last_update_delegated_lamports: delegated_lamports,
                last_update_epoch: clock.epoch,
                is_emergency_unstaking,
            }
        }
    }
    pub struct StakeSystem {
        pub stake_list: List,
        pub delayed_unstake_cooling_down: u64,
        pub stake_deposit_bump_seed: u8,
        pub stake_withdraw_bump_seed: u8,
        /// set by admin, how much slots before the end of the epoch, stake-delta can start
        pub slots_for_stake_delta: u64,
        /// Marks the start of stake-delta operations, meaning that if somebody starts a delayed-unstake ticket
        /// after this var is set with epoch_num the ticket will have epoch_created = current_epoch+1
        /// (the user must wait one more epoch, because their unstake-delta will be execute in this epoch)
        pub last_stake_delta_epoch: u64,
        pub min_stake: u64,
        /// can be set by validator-manager-auth to allow a second run of stake-delta to stake late stakers in the last minute of the epoch
        /// so we maximize user's rewards
        pub extra_stake_delta_runs: u32,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for StakeSystem {
        #[inline]
        fn clone(&self) -> StakeSystem {
            match *self {
                Self {
                    stake_list: ref __self_0_0,
                    delayed_unstake_cooling_down: ref __self_0_1,
                    stake_deposit_bump_seed: ref __self_0_2,
                    stake_withdraw_bump_seed: ref __self_0_3,
                    slots_for_stake_delta: ref __self_0_4,
                    last_stake_delta_epoch: ref __self_0_5,
                    min_stake: ref __self_0_6,
                    extra_stake_delta_runs: ref __self_0_7,
                } => StakeSystem {
                    stake_list: ::core::clone::Clone::clone(&(*__self_0_0)),
                    delayed_unstake_cooling_down: ::core::clone::Clone::clone(&(*__self_0_1)),
                    stake_deposit_bump_seed: ::core::clone::Clone::clone(&(*__self_0_2)),
                    stake_withdraw_bump_seed: ::core::clone::Clone::clone(&(*__self_0_3)),
                    slots_for_stake_delta: ::core::clone::Clone::clone(&(*__self_0_4)),
                    last_stake_delta_epoch: ::core::clone::Clone::clone(&(*__self_0_5)),
                    min_stake: ::core::clone::Clone::clone(&(*__self_0_6)),
                    extra_stake_delta_runs: ::core::clone::Clone::clone(&(*__self_0_7)),
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for StakeSystem
    where
        List: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.delayed_unstake_cooling_down, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_withdraw_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.slots_for_stake_delta, writer)?;
            borsh::BorshSerialize::serialize(&self.last_stake_delta_epoch, writer)?;
            borsh::BorshSerialize::serialize(&self.min_stake, writer)?;
            borsh::BorshSerialize::serialize(&self.extra_stake_delta_runs, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for StakeSystem
    where
        List: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_list: borsh::BorshDeserialize::deserialize(buf)?,
                delayed_unstake_cooling_down: borsh::BorshDeserialize::deserialize(buf)?,
                stake_deposit_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                stake_withdraw_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                slots_for_stake_delta: borsh::BorshDeserialize::deserialize(buf)?,
                last_stake_delta_epoch: borsh::BorshDeserialize::deserialize(buf)?,
                min_stake: borsh::BorshDeserialize::deserialize(buf)?,
                extra_stake_delta_runs: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for StakeSystem {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    stake_list: ref __self_0_0,
                    delayed_unstake_cooling_down: ref __self_0_1,
                    stake_deposit_bump_seed: ref __self_0_2,
                    stake_withdraw_bump_seed: ref __self_0_3,
                    slots_for_stake_delta: ref __self_0_4,
                    last_stake_delta_epoch: ref __self_0_5,
                    min_stake: ref __self_0_6,
                    extra_stake_delta_runs: ref __self_0_7,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "StakeSystem");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stake_list",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "delayed_unstake_cooling_down",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stake_deposit_bump_seed",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stake_withdraw_bump_seed",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "slots_for_stake_delta",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "last_stake_delta_epoch",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "min_stake",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "extra_stake_delta_runs",
                        &&(*__self_0_7),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl StakeSystem {
        pub const STAKE_WITHDRAW_SEED: &'static [u8] = b"withdraw";
        pub const STAKE_DEPOSIT_SEED: &'static [u8] = b"deposit";
        pub fn bytes_for_list(count: u32, additional_record_space: u32) -> u32 {
            List::bytes_for(
                StakeRecord::default().try_to_vec().unwrap().len() as u32 + additional_record_space,
                count,
            )
        }
        pub fn find_stake_withdraw_authority(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[&state.to_bytes()[..32], Self::STAKE_WITHDRAW_SEED], &ID)
        }
        pub fn find_stake_deposit_authority(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[&state.to_bytes()[..32], Self::STAKE_DEPOSIT_SEED], &ID)
        }
        pub fn new(
            state: &Pubkey,
            stake_list_account: Pubkey,
            stake_list_data: &mut [u8],
            slots_for_stake_delta: u64,
            min_stake: u64,
            extra_stake_delta_runs: u32,
            additional_record_space: u32,
        ) -> Result<Self, ProgramError> {
            let stake_list = List::new(
                StakeRecord::DISCRIMINATOR,
                StakeRecord::default().try_to_vec().unwrap().len() as u32 + additional_record_space,
                stake_list_account,
                stake_list_data,
                "stake_list",
            )?;
            Ok(Self {
                stake_list,
                delayed_unstake_cooling_down: 0,
                stake_deposit_bump_seed: Self::find_stake_deposit_authority(state).1,
                stake_withdraw_bump_seed: Self::find_stake_withdraw_authority(state).1,
                slots_for_stake_delta,
                last_stake_delta_epoch: Epoch::MAX,
                min_stake,
                extra_stake_delta_runs,
            })
        }
        pub fn stake_list_address(&self) -> &Pubkey {
            &self.stake_list.account
        }
        pub fn stake_count(&self) -> u32 {
            self.stake_list.len()
        }
        pub fn stake_list_capacity(&self, stake_list_len: usize) -> Result<u32, ProgramError> {
            self.stake_list.capacity(stake_list_len)
        }
        pub fn stake_record_size(&self) -> u32 {
            self.stake_list.item_size()
        }
        pub fn add(
            &mut self,
            stake_list_data: &mut [u8],
            stake_account: &Pubkey,
            delegated_lamports: u64,
            clock: &Clock,
            is_emergency_unstaking: u8,
        ) -> ProgramResult {
            self.stake_list.push(
                stake_list_data,
                StakeRecord::new(
                    stake_account,
                    delegated_lamports,
                    clock,
                    is_emergency_unstaking,
                ),
                "stake_list",
            )?;
            Ok(())
        }
        fn get(&self, stake_list_data: &[u8], index: u32) -> Result<StakeRecord, ProgramError> {
            self.stake_list.get(stake_list_data, index, "stake_list")
        }
        /// get the stake account record from an index, and check that the account is the same passed as parameter to the instruction
        pub fn get_checked(
            &self,
            stake_list_data: &[u8],
            index: u32,
            received_pubkey: &Pubkey,
        ) -> Result<StakeRecord, ProgramError> {
            let stake_record = self.get(stake_list_data, index)?;
            if stake_record.stake_account != *received_pubkey {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &[
                            "Stake account ",
                            " must match stake_list[",
                            "] = ",
                            ". Maybe list layout was changed",
                        ],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&received_pubkey),
                            ::core::fmt::ArgumentV1::new_display(&index),
                            ::core::fmt::ArgumentV1::new_display(&stake_record.stake_account),
                        ],
                    ));
                    res
                });
                Err(ProgramError::InvalidAccountData)
            } else {
                Ok(stake_record)
            }
        }
        pub fn set(
            &self,
            stake_list_data: &mut [u8],
            index: u32,
            stake: StakeRecord,
        ) -> ProgramResult {
            self.stake_list
                .set(stake_list_data, index, stake, "stake_list")
        }
        pub fn remove(&mut self, stake_list_data: &mut [u8], index: u32) -> ProgramResult {
            self.stake_list.remove(stake_list_data, index, "stake_list")
        }
        pub fn check_stake_list<'info>(&self, stake_list: &AccountInfo<'info>) -> ProgramResult {
            check_address(stake_list.key, self.stake_list_address(), "stake_list")?;
            if &stake_list.data.borrow().as_ref()[0..8] != StakeRecord::DISCRIMINATOR {
                ::solana_program::log::sol_log("Wrong stake list account discriminator");
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(())
        }
    }
    pub trait StakeSystemHelpers {
        fn stake_withdraw_authority(&self) -> Pubkey;
        fn with_stake_withdraw_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn check_stake_withdraw_authority(
            &self,
            stake_withdraw_authority: &Pubkey,
        ) -> ProgramResult;
        fn stake_deposit_authority(&self) -> Pubkey;
        fn with_stake_deposit_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn check_stake_deposit_authority(&self, stake_deposit_authority: &Pubkey) -> ProgramResult;
    }
    impl<T> StakeSystemHelpers for T
    where
        T: Located<State>,
    {
        fn stake_withdraw_authority(&self) -> Pubkey {
            self.with_stake_withdraw_authority_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn with_stake_withdraw_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                StakeSystem::STAKE_WITHDRAW_SEED,
                &[self.as_ref().stake_system.stake_withdraw_bump_seed],
            ])
        }
        fn check_stake_withdraw_authority(
            &self,
            stake_withdraw_authority: &Pubkey,
        ) -> ProgramResult {
            check_address(
                stake_withdraw_authority,
                &self.stake_withdraw_authority(),
                "stake_withdraw_authority",
            )
        }
        fn stake_deposit_authority(&self) -> Pubkey {
            self.with_stake_deposit_authority_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn with_stake_deposit_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                StakeSystem::STAKE_DEPOSIT_SEED,
                &[self.as_ref().stake_system.stake_deposit_bump_seed],
            ])
        }
        fn check_stake_deposit_authority(&self, stake_deposit_authority: &Pubkey) -> ProgramResult {
            check_address(
                stake_deposit_authority,
                &self.stake_deposit_authority(),
                "stake_deposit_authority",
            )
        }
    }
}
pub mod stake_wrapper {
    use std::ops::Deref;
    use anchor_lang::prelude::ProgramError;
    use anchor_lang::solana_program::stake::state::StakeState;
    pub struct StakeWrapper {
        pub inner: StakeState,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for StakeWrapper {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    inner: ref __self_0_0,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "StakeWrapper");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "inner",
                        &&(*__self_0_0),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for StakeWrapper {
        #[inline]
        fn clone(&self) -> StakeWrapper {
            match *self {
                Self {
                    inner: ref __self_0_0,
                } => StakeWrapper {
                    inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                },
            }
        }
    }
    impl anchor_lang::AccountDeserialize for StakeWrapper {
        fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
            let result = Self {
                inner: bincode::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData)?,
            };
            *buf = &buf[std::mem::size_of::<StakeState>()..];
            Ok(result)
        }
    }
    impl Deref for StakeWrapper {
        type Target = StakeState;
        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }
}
pub mod state {
    use crate::{
        calc::{shares_from_value, value_from_shares},
        checks::check_address,
        error::CommonError,
        liq_pool::LiqPool,
        located::Located,
        stake_system::StakeSystem,
        validator_system::ValidatorSystem,
        Fee, ID,
    };
    use anchor_lang::prelude::*;
    use anchor_lang::solana_program::program_pack::Pack;
    use std::mem::MaybeUninit;
    pub mod change_authority {
        use anchor_lang::prelude::*;
        use crate::{ChangeAuthority, ChangeAuthorityData};
        impl<'info> ChangeAuthority<'info> {
            pub fn process(&mut self, data: ChangeAuthorityData) -> ProgramResult {
                self.state.check_admin_authority(self.admin_authority.key)?;
                if let Some(admin) = data.admin {
                    self.state.admin_authority = admin;
                }
                if let Some(validator_manager) = data.validator_manager {
                    self.state.validator_system.manager_authority = validator_manager;
                }
                if let Some(operational_sol_account) = data.operational_sol_account {
                    self.state.operational_sol_account = operational_sol_account;
                }
                if let Some(treasury_msol_account) = data.treasury_msol_account {
                    self.state.treasury_msol_account = treasury_msol_account;
                }
                Ok(())
            }
        }
    }
    pub mod claim {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke_signed, system_instruction, system_program};
        use crate::{
            checks::{check_address, check_owner_program},
            state::StateHelpers,
            Claim, CommonError,
        };
        ///How many epochs to wats for ticket. e.g.: Ticket created on epoch 14, ticket is due on epoch 15
        const WAIT_EPOCHS: u64 = 1;
        ///Wait 30 extra minutes from epochs start so the bot has time to withdraw SOL from inactive stake-accounts
        const EXTRA_WAIT_SECONDS: i64 = 30 * 60;
        /// Claim instruction: a user claims a Ticket-account
        /// This is done once tickets are due, meaning enough time has passed for the
        /// bot to complete the unstake process and transfer the requested SOL to reserve_pda.
        /// Checks that transfer request amount is less than total requested for unstake
        impl<'info> Claim<'info> {
            fn check_ticket_account(&self) -> ProgramResult {
                check_owner_program(&self.ticket_account, &crate::ID, "ticket_account")?;
                if &self.ticket_account.state_address != self.state.to_account_info().key {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Ticket has wrong marinade instance "],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.ticket_account.state_address,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                if self.ticket_account.lamports_amount == 0 {
                    ::solana_program::log::sol_log("Used ticket");
                    return Err(ProgramError::InvalidAccountData);
                };
                if self.clock.epoch < self.ticket_account.created_epoch + WAIT_EPOCHS {
                    ::solana_program::log::sol_log("Ticket not due yet");
                    return Err(CommonError::TicketNotDue.into());
                }
                if self.ticket_account.created_epoch + WAIT_EPOCHS == self.clock.epoch
                    && self.clock.unix_timestamp - self.clock.epoch_start_timestamp
                        < EXTRA_WAIT_SECONDS
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Ticket not ready ", " "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.clock.epoch_start_timestamp,
                                ),
                                ::core::fmt::ArgumentV1::new_display(&self.clock.unix_timestamp),
                            ],
                        ));
                        res
                    });
                    return Err(CommonError::TicketNotReady.into());
                }
                if self.ticket_account.beneficiary != *self.transfer_sol_to.key {
                    ::solana_program::log::sol_log("wrong beneficiary");
                    return Err(CommonError::WrongBeneficiary.into());
                };
                Ok(())
            }
            pub fn process(&mut self) -> ProgramResult {
                check_address(
                    self.system_program.to_account_info().key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_owner_program(
                    &self.transfer_sol_to,
                    &system_program::ID,
                    "transfer_sol_to",
                )?;
                self.state.check_reserve_address(self.reserve_pda.key)?;
                self.check_ticket_account()?;
                let lamports = self.ticket_account.lamports_amount;
                if lamports > self.state.circulating_ticket_balance {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Requested to withdraw ",
                                " when only ",
                                " is total circulating_ticket_balance",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&lamports),
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.state.circulating_ticket_balance,
                                ),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let available_for_claim =
                    self.reserve_pda.lamports() - self.state.rent_exempt_for_token_acc;
                if lamports > available_for_claim {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Requested to claim ",
                                " when only ",
                                " ready. Wait a few hours and retry",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&lamports),
                                ::core::fmt::ArgumentV1::new_display(&available_for_claim),
                            ],
                        ));
                        res
                    });
                    return Err(CommonError::TicketNotReady.into());
                }
                self.state.circulating_ticket_balance -= lamports;
                self.state.circulating_ticket_count -= 1;
                self.ticket_account.lamports_amount = 0;
                self.state.with_reserve_seeds(|seeds| {
                    invoke_signed(
                        &system_instruction::transfer(
                            self.reserve_pda.key,
                            self.transfer_sol_to.key,
                            lamports,
                        ),
                        &[
                            self.system_program.clone(),
                            self.reserve_pda.clone(),
                            self.transfer_sol_to.clone(),
                        ],
                        &[seeds],
                    )
                })?;
                self.state.on_transfer_from_reserve(lamports)?;
                let source_account_info = self.ticket_account.to_account_info();
                let dest_account_info = self.transfer_sol_to.to_account_info();
                let dest_starting_lamports = dest_account_info.lamports();
                **dest_account_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(source_account_info.lamports())
                    .ok_or(ProgramError::InvalidAccountData)?;
                **source_account_info.lamports.borrow_mut() = 0;
                Ok(())
            }
        }
    }
    pub mod config_marinade {
        use crate::{CommonError, ConfigMarinade, ConfigMarinadeParams, MAX_REWARD_FEE};
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
        impl<'info> ConfigMarinade<'info> {
            const MIN_WITHDRAW_CAP: u64 = LAMPORTS_PER_SOL / 10;
            pub fn process(
                &mut self,
                ConfigMarinadeParams {
                    rewards_fee,
                    slots_for_stake_delta,
                    min_stake,
                    min_deposit,
                    min_withdraw,
                    staking_sol_cap,
                    liquidity_sol_cap,
                    auto_add_validator_enabled,
                }: ConfigMarinadeParams,
            ) -> ProgramResult {
                self.state.check_admin_authority(self.admin_authority.key)?;
                if let Some(rewards_fee) = rewards_fee {
                    rewards_fee.check_max(MAX_REWARD_FEE)?;
                    self.state.reward_fee = rewards_fee;
                }
                if let Some(slots_for_stake_delta) = slots_for_stake_delta {
                    const MIN_UPDATE_WINDOW: u64 = 3_000;
                    if slots_for_stake_delta < MIN_UPDATE_WINDOW {
                        return Err(CommonError::NumberTooLow.into());
                    };
                    self.state.stake_system.slots_for_stake_delta = slots_for_stake_delta;
                }
                if let Some(min_stake) = min_stake {
                    let min_accepted = 5 * self.state.rent_exempt_for_token_acc;
                    if min_stake < min_accepted {
                        return Err(CommonError::NumberTooLow.into());
                    };
                    self.state.stake_system.min_stake = min_stake;
                }
                if let Some(min_deposit) = min_deposit {
                    self.state.min_deposit = min_deposit;
                }
                if let Some(min_withdraw) = min_withdraw {
                    if min_withdraw > Self::MIN_WITHDRAW_CAP {
                        return Err(CommonError::NumberTooHigh.into());
                    }
                    self.state.min_withdraw = min_withdraw;
                }
                if let Some(staking_sol_cap) = staking_sol_cap {
                    self.state.staking_sol_cap = staking_sol_cap;
                }
                if let Some(liquidity_sol_cap) = liquidity_sol_cap {
                    self.state.liq_pool.liquidity_sol_cap = liquidity_sol_cap;
                }
                if let Some(auto_add_validator_enabled) = auto_add_validator_enabled {
                    self.state.validator_system.auto_add_validator_enabled =
                        if auto_add_validator_enabled { 1 } else { 0 };
                }
                Ok(())
            }
        }
    }
    pub mod deposit {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke, system_instruction, system_program};
        use anchor_spl::token::{mint_to, transfer, MintTo, Transfer};
        use crate::{
            checks::{check_address, check_min_amount, check_owner_program, check_token_mint},
            liq_pool::LiqPoolHelpers,
            state::StateHelpers,
            Deposit,
        };
        impl<'info> Deposit<'info> {
            fn check_transfer_from(&self, lamports: u64) -> ProgramResult {
                check_owner_program(&self.transfer_from, &system_program::ID, "transfer_from")?;
                if self.transfer_from.lamports() < lamports {
                    return Err(ProgramError::InsufficientFunds);
                }
                Ok(())
            }
            fn check_mint_to(&self) -> ProgramResult {
                check_token_mint(&self.mint_to, self.state.msol_mint, "mint_to")?;
                Ok(())
            }
            pub fn process(&mut self, lamports: u64) -> ProgramResult {
                check_min_amount(lamports, self.state.min_deposit, "deposit SOL")?;
                self.state.check_reserve_address(self.reserve_pda.key)?;
                self.state
                    .check_msol_mint(self.msol_mint.to_account_info().key)?;
                self.state
                    .check_liq_pool_sol_leg_pda(self.liq_pool_sol_leg_pda.key)?;
                self.state
                    .liq_pool
                    .check_liq_pool_msol_leg(self.liq_pool_msol_leg.to_account_info().key)?;
                self.check_transfer_from(lamports)?;
                self.check_mint_to()?;
                self.state
                    .check_msol_mint_authority(self.msol_mint_authority.key)?;
                check_address(
                    self.system_program.to_account_info().key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(
                    self.token_program.to_account_info().key,
                    &spl_token::ID,
                    "token_program",
                )?;
                if self.msol_mint.supply > self.state.msol_supply {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Warning: mSOL minted ", " lamports outside of marinade"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &(self.msol_mint.supply - self.state.msol_supply),
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let user_lamports = lamports;
                let user_msol_buy_order = self.state.calc_msol_from_lamports(user_lamports)?;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["--- user_m_sol_buy_order "],
                        &[::core::fmt::ArgumentV1::new_display(&user_msol_buy_order)],
                    ));
                    res
                });
                let swap_msol_max: u64 = user_msol_buy_order.min(self.liq_pool_msol_leg.amount);
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["--- swap_m_sol_max "],
                        &[::core::fmt::ArgumentV1::new_display(&swap_msol_max)],
                    ));
                    res
                });
                let user_lamports = if swap_msol_max > 0 {
                    let lamports_for_the_liq_pool = if user_msol_buy_order == swap_msol_max {
                        user_lamports
                    } else {
                        self.state.calc_lamports_from_msol_amount(swap_msol_max)?
                    };
                    self.state.with_liq_pool_msol_leg_authority_seeds(|seeds| {
                        transfer(
                            CpiContext::new_with_signer(
                                self.token_program.clone(),
                                Transfer {
                                    from: self.liq_pool_msol_leg.to_account_info(),
                                    to: self.mint_to.to_account_info(),
                                    authority: self.liq_pool_msol_leg_authority.clone(),
                                },
                                &[seeds],
                            ),
                            swap_msol_max,
                        )
                    })?;
                    invoke(
                        &system_instruction::transfer(
                            self.transfer_from.key,
                            self.liq_pool_sol_leg_pda.key,
                            lamports_for_the_liq_pool,
                        ),
                        &[
                            self.transfer_from.clone(),
                            self.liq_pool_sol_leg_pda.clone(),
                            self.system_program.clone(),
                        ],
                    )?;
                    user_lamports.saturating_sub(lamports_for_the_liq_pool)
                } else {
                    user_lamports
                };
                if user_lamports > 0 {
                    self.state.check_staking_cap(user_lamports)?;
                    let msol_to_mint = self.state.calc_msol_from_lamports(user_lamports)?;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["--- msol_to_mint "],
                            &[::core::fmt::ArgumentV1::new_display(&msol_to_mint)],
                        ));
                        res
                    });
                    invoke(
                        &system_instruction::transfer(
                            self.transfer_from.key,
                            self.reserve_pda.key,
                            user_lamports,
                        ),
                        &[
                            self.transfer_from.clone(),
                            self.reserve_pda.clone(),
                            self.system_program.clone(),
                        ],
                    )?;
                    self.state.on_transfer_to_reserve(user_lamports);
                    if msol_to_mint > 0 {
                        self.state.with_msol_mint_authority_seeds(|mint_seeds| {
                            mint_to(
                                CpiContext::new_with_signer(
                                    self.token_program.clone(),
                                    MintTo {
                                        mint: self.msol_mint.to_account_info(),
                                        to: self.mint_to.to_account_info(),
                                        authority: self.msol_mint_authority.clone(),
                                    },
                                    &[mint_seeds],
                                ),
                                msol_to_mint,
                            )
                        })?;
                        self.state.on_msol_mint(msol_to_mint);
                    }
                }
                Ok(())
            }
        }
    }
    pub mod initialize {
        use crate::{
            checks::{
                check_address, check_freeze_authority, check_mint_authority, check_mint_empty,
                check_owner_program, check_token_mint,
            },
            stake_system::StakeSystem,
            validator_system::ValidatorSystem,
            Initialize, InitializeData, LiqPoolInitialize, ID, MAX_REWARD_FEE,
        };
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program_pack::Pack, system_program};
        use super::State;
        impl<'info> Initialize<'info> {
            pub const CREATOR_AUTHORITY: Pubkey = Pubkey::new_from_array([
                130, 33, 92, 198, 248, 0, 48, 210, 221, 172, 150, 104, 107, 227, 44, 217, 3, 61,
                74, 58, 179, 76, 35, 104, 39, 67, 130, 92, 93, 25, 180, 107,
            ]);
            pub fn state(&self) -> &State {
                &self.state
            }
            pub fn state_address(&self) -> &Pubkey {
                self.state.to_account_info().key
            }
            fn check_state(&self) -> ProgramResult {
                Ok(())
            }
            fn check_reserve_pda(&mut self) -> ProgramResult {
                check_owner_program(&self.reserve_pda, &system_program::ID, "reserve_pda")?;
                let (address, bump) = State::find_reserve_address(self.state_address());
                check_address(self.reserve_pda.key, &address, "reserve_pda")?;
                self.state.reserve_bump_seed = bump;
                {
                    let lamports = self.reserve_pda.lamports();
                    if lamports != self.state.rent_exempt_for_token_acc {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Invalid initial reserve lamports ", " expected "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(&lamports),
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.state.rent_exempt_for_token_acc,
                                    ),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InvalidArgument);
                    }
                }
                Ok(())
            }
            fn check_msol_mint(&mut self) -> ProgramResult {
                check_owner_program(&self.msol_mint, &spl_token::ID, "msol_mint")?;
                let (authority_address, authority_bump_seed) =
                    State::find_msol_mint_authority(self.state_address());
                check_mint_authority(&self.msol_mint, authority_address, "msol_mint")?;
                self.state.msol_mint_authority_bump_seed = authority_bump_seed;
                check_mint_empty(&self.msol_mint, "msol_mint")?;
                check_freeze_authority(&self.msol_mint, "msol_mint")?;
                Ok(())
            }
            fn check_treasury_accounts(&self) -> ProgramResult {
                check_owner_program(
                    &self.treasury_msol_account,
                    &anchor_spl::token::ID,
                    "treasury_msol_account",
                )?;
                check_token_mint(
                    &self.treasury_msol_account,
                    *self.msol_mint.to_account_info().key,
                    "treasury_msol_account",
                )?;
                Ok(())
            }
            pub fn process(&mut self, data: InitializeData) -> ProgramResult {
                check_address(
                    self.creator_authority.key,
                    &Initialize::CREATOR_AUTHORITY,
                    "creator_authority",
                )?;
                data.reward_fee.check_max(MAX_REWARD_FEE)?;
                self.state.rent_exempt_for_token_acc =
                    self.rent.minimum_balance(spl_token::state::Account::LEN);
                self.check_state()?;
                self.check_reserve_pda()?;
                self.check_msol_mint()?;
                self.check_treasury_accounts()?;
                check_owner_program(
                    &self.operational_sol_account,
                    &system_program::ID,
                    "operational_sol",
                )?;
                check_owner_program(&self.stake_list, &ID, "stake_list")?;
                check_owner_program(&self.validator_list, &ID, "validator_list")?;
                self.state.msol_mint = *self.msol_mint.to_account_info().key;
                self.state.admin_authority = data.admin_authority;
                self.state.operational_sol_account = *self.operational_sol_account.key;
                self.state.reward_fee = data.reward_fee;
                self.state.stake_system = StakeSystem::new(
                    self.state_address(),
                    *self.stake_list.key,
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    data.slots_for_stake_delta,
                    data.min_stake,
                    0,
                    data.additional_stake_record_space,
                )?;
                self.state.validator_system = ValidatorSystem::new(
                    *self.validator_list.key,
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    data.validator_manager_authority,
                    data.additional_validator_record_space,
                )?;
                self.state.msol_price = State::PRICE_DENOMINATOR;
                self.state.treasury_msol_account =
                    *self.treasury_msol_account.to_account_info().key;
                self.state.min_deposit = 1;
                self.state.min_withdraw = 1;
                self.state.staking_sol_cap = std::u64::MAX;
                LiqPoolInitialize::process(self, data.liq_pool)?;
                Ok(())
            }
        }
    }
    pub mod liquid_unstake {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke_signed, system_instruction, system_program};
        use anchor_spl::token::{transfer, Transfer};
        use crate::checks::check_min_amount;
        use crate::{
            checks::{check_address, check_owner_program, check_token_mint},
            liq_pool::LiqPoolHelpers,
            CommonError, LiquidUnstake,
        };
        impl<'info> LiquidUnstake<'info> {
            fn check_get_msol_from(&self, msol_amount: u64) -> ProgramResult {
                check_token_mint(&self.get_msol_from, self.state.msol_mint, "get_msol_from")?;
                if *self.get_msol_from_authority.key == self.get_msol_from.owner {
                    if self.get_msol_from.amount < msol_amount {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Requested to unstake ", " mSOL lamports but have only "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(&msol_amount),
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.get_msol_from.amount,
                                    ),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else if self
                    .get_msol_from
                    .delegate
                    .contains(self.get_msol_from_authority.key)
                {
                    if self.get_msol_from.delegated_amount < msol_amount {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Delegated ", " mSOL lamports. Requested "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.get_msol_from.delegated_amount,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&msol_amount),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Token must be delegated to "],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.get_msol_from_authority.key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                Ok(())
            }
            fn check_transfer_sol_to(&self) -> ProgramResult {
                check_owner_program(&self.transfer_sol_to, &system_program::ID, "transfer_from")?;
                Ok(())
            }
            pub fn process(&mut self, msol_amount: u64) -> ProgramResult {
                ::solana_program::log::sol_log("enter LiquidUnstake");
                self.state
                    .check_msol_mint(self.msol_mint.to_account_info().key)?;
                self.state
                    .check_liq_pool_sol_leg_pda(self.liq_pool_sol_leg_pda.key)?;
                self.state
                    .liq_pool
                    .check_liq_pool_msol_leg(self.liq_pool_msol_leg.to_account_info().key)?;
                self.check_get_msol_from(msol_amount)?;
                self.check_transfer_sol_to()?;
                let is_treasury_msol_ready_for_transfer = self
                    .state
                    .check_treasury_msol_account(&self.treasury_msol_account)?;
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                check_address(self.token_program.key, &spl_token::ID, "token_program")?;
                let max_lamports = self
                    .liq_pool_sol_leg_pda
                    .lamports()
                    .saturating_sub(self.state.rent_exempt_for_token_acc);
                let user_remove_lamports =
                    self.state.calc_lamports_from_msol_amount(msol_amount)?;
                let liquid_unstake_fee = if user_remove_lamports >= max_lamports {
                    self.state.liq_pool.lp_max_fee
                } else {
                    let after_lamports = max_lamports - user_remove_lamports;
                    self.state.liq_pool.linear_fee(after_lamports)
                };
                let msol_fee = liquid_unstake_fee.apply(msol_amount);
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["msol_fee "],
                        &[::core::fmt::ArgumentV1::new_display(&msol_fee)],
                    ));
                    res
                });
                let working_lamports_value = self
                    .state
                    .calc_lamports_from_msol_amount(msol_amount - msol_fee)?;
                if working_lamports_value + self.state.rent_exempt_for_token_acc
                    > self.liq_pool_sol_leg_pda.lamports()
                {
                    return Err(CommonError::InsufficientLiquidity.into());
                }
                check_min_amount(
                    working_lamports_value,
                    self.state.min_withdraw,
                    "withdraw SOL",
                )?;
                if working_lamports_value > 0 {
                    self.state.with_liq_pool_sol_leg_seeds(|sol_seeds| {
                        invoke_signed(
                            &system_instruction::transfer(
                                self.liq_pool_sol_leg_pda.key,
                                self.transfer_sol_to.key,
                                working_lamports_value,
                            ),
                            &[
                                self.liq_pool_sol_leg_pda.clone(),
                                self.transfer_sol_to.clone(),
                                self.system_program.clone(),
                            ],
                            &[sol_seeds],
                        )
                    })?;
                }
                let treasury_msol_cut = if is_treasury_msol_ready_for_transfer {
                    self.state.liq_pool.treasury_cut.apply(msol_fee)
                } else {
                    0
                };
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["treasury_msol_cut "],
                        &[::core::fmt::ArgumentV1::new_display(&treasury_msol_cut)],
                    ));
                    res
                });
                transfer(
                    CpiContext::new(
                        self.token_program.clone(),
                        Transfer {
                            from: self.get_msol_from.to_account_info(),
                            to: self.liq_pool_msol_leg.to_account_info(),
                            authority: self.get_msol_from_authority.clone(),
                        },
                    ),
                    msol_amount - treasury_msol_cut,
                )?;
                if treasury_msol_cut > 0 {
                    transfer(
                        CpiContext::new(
                            self.token_program.clone(),
                            Transfer {
                                from: self.get_msol_from.to_account_info(),
                                to: self.treasury_msol_account.to_account_info(),
                                authority: self.get_msol_from_authority.clone(),
                            },
                        ),
                        treasury_msol_cut,
                    )?;
                }
                Ok(())
            }
        }
    }
    pub mod order_unstake {
        use anchor_lang::prelude::*;
        use anchor_spl::token::{burn, Burn};
        use crate::{
            checks::{check_address, check_min_amount, check_owner_program, check_token_mint},
            OrderUnstake,
        };
        impl<'info> OrderUnstake<'info> {
            fn check_burn_msol_from(&self, msol_amount: u64) -> ProgramResult {
                check_token_mint(&self.burn_msol_from, self.state.msol_mint, "burn_msol_from")?;
                if msol_amount == 0 {
                    return Err(ProgramError::InvalidAccountData);
                }
                if *self.burn_msol_authority.key == self.burn_msol_from.owner {
                    if self.burn_msol_from.amount < msol_amount {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Requested to unstake ", " mSOL lamports but have only "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(&msol_amount),
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.burn_msol_from.amount,
                                    ),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else if self
                    .burn_msol_from
                    .delegate
                    .contains(self.burn_msol_authority.key)
                {
                    if self.burn_msol_from.delegated_amount < msol_amount {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Delegated ", " mSOL lamports. Requested "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &self.burn_msol_from.delegated_amount,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&msol_amount),
                                ],
                            ));
                            res
                        });
                        return Err(ProgramError::InsufficientFunds);
                    }
                } else {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Token must be delegated to "],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.burn_msol_authority.key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                Ok(())
            }
            fn check_new_ticket_account(&self) -> ProgramResult {
                check_owner_program(&self.new_ticket_account, &crate::ID, "new_ticket_account")?;
                Ok(())
            }
            pub fn process(&mut self, msol_amount: u64) -> ProgramResult {
                check_address(self.token_program.key, &spl_token::ID, "token_program")?;
                self.check_new_ticket_account()?;
                self.state
                    .check_msol_mint(self.msol_mint.to_account_info().key)?;
                self.check_burn_msol_from(msol_amount)?;
                let ticket_beneficiary = self.burn_msol_from.owner;
                let lamports_amount = self.state.calc_lamports_from_msol_amount(msol_amount)?;
                check_min_amount(lamports_amount, self.state.min_withdraw, "withdraw SOL")?;
                self.state.circulating_ticket_balance = self
                    .state
                    .circulating_ticket_balance
                    .checked_add(lamports_amount)
                    .expect("circulating_ticket_balance overflow");
                self.state.circulating_ticket_count += 1;
                burn(
                    CpiContext::new(
                        self.token_program.clone(),
                        Burn {
                            mint: self.msol_mint.to_account_info(),
                            to: self.burn_msol_from.to_account_info(),
                            authority: self.burn_msol_authority.clone(),
                        },
                    ),
                    msol_amount,
                )?;
                self.state.on_msol_burn(msol_amount)?;
                self.new_ticket_account.state_address = *self.state.to_account_info().key;
                self.new_ticket_account.beneficiary = ticket_beneficiary;
                self.new_ticket_account.lamports_amount = lamports_amount;
                self.new_ticket_account.created_epoch = self.clock.epoch
                    + if self.clock.epoch == self.state.stake_system.last_stake_delta_epoch {
                        1
                    } else {
                        0
                    };
                Ok(())
            }
        }
    }
    pub mod update {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{
            program::invoke_signed, stake, system_instruction, system_program,
        };
        use anchor_spl::token::{mint_to, MintTo};
        use crate::error::CommonError;
        use crate::{
            checks::check_address,
            stake_system::{StakeRecord, StakeSystemHelpers},
            state::StateHelpers,
            State, UpdateActive, UpdateCommon, UpdateDeactivated,
        };
        struct BeginOutput {
            stake: StakeRecord,
            is_treasury_msol_ready_for_transfer: bool,
        }
        impl<'info> UpdateCommon<'info> {
            fn begin(&mut self, stake_index: u32) -> Result<BeginOutput, ProgramError> {
                self.state.stake_system.check_stake_list(&self.stake_list)?;
                self.state
                    .check_msol_mint(self.msol_mint.to_account_info().key)?;
                self.state
                    .check_msol_mint_authority(self.msol_mint_authority.key)?;
                let is_treasury_msol_ready_for_transfer = self
                    .state
                    .check_treasury_msol_account(&self.treasury_msol_account)?;
                self.state
                    .check_stake_withdraw_authority(self.stake_withdraw_authority.key)?;
                self.state.check_reserve_address(self.reserve_pda.key)?;
                check_address(self.stake_program.key, &stake::program::ID, "stake_program")?;
                check_address(self.token_program.key, &spl_token::ID, "token_program")?;
                let virtual_reserve_balance = self
                    .state
                    .available_reserve_balance
                    .checked_add(self.state.rent_exempt_for_token_acc)
                    .expect("reserve balance overflow");
                if self.reserve_pda.lamports() < virtual_reserve_balance {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Warning: Reserve must have ", " lamports but got "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&virtual_reserve_balance),
                                ::core::fmt::ArgumentV1::new_display(&self.reserve_pda.lamports()),
                            ],
                        ));
                        res
                    });
                }
                self.state.available_reserve_balance = self
                    .reserve_pda
                    .lamports()
                    .saturating_sub(self.state.rent_exempt_for_token_acc);
                if self.msol_mint.supply > self.state.msol_supply {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Warning: mSOL minted ", " lamports outside of marinade"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &(self.msol_mint.supply - self.state.msol_supply),
                            )],
                        ));
                        res
                    });
                    self.state.staking_sol_cap = 0;
                }
                self.state.msol_supply = self.msol_mint.supply;
                let stake = self.state.stake_system.get_checked(
                    &self.stake_list.data.as_ref().borrow(),
                    stake_index,
                    self.stake_account.to_account_info().key,
                )?;
                Ok(BeginOutput {
                    stake,
                    is_treasury_msol_ready_for_transfer,
                })
            }
            pub fn withdraw_to_reserve(&mut self, amount: u64) -> ProgramResult {
                if amount > 0 {
                    self.state.with_stake_withdraw_authority_seeds(|seeds| {
                        invoke_signed(
                            &stake::instruction::withdraw(
                                self.stake_account.to_account_info().key,
                                self.stake_withdraw_authority.key,
                                self.reserve_pda.key,
                                amount,
                                None,
                            ),
                            &[
                                self.stake_program.clone(),
                                self.stake_account.to_account_info(),
                                self.reserve_pda.clone(),
                                self.clock.to_account_info(),
                                self.stake_history.clone(),
                                self.stake_withdraw_authority.clone(),
                            ],
                            &[seeds],
                        )
                    })?;
                    self.state.on_transfer_to_reserve(amount);
                }
                Ok(())
            }
            pub fn mint_to_treasury(&mut self, msol_lamports: u64) -> ProgramResult {
                if msol_lamports > 0 {
                    self.state.with_msol_mint_authority_seeds(|seeds| {
                        mint_to(
                            CpiContext::new_with_signer(
                                self.token_program.clone(),
                                MintTo {
                                    mint: self.msol_mint.to_account_info(),
                                    to: self.treasury_msol_account.to_account_info(),
                                    authority: self.msol_mint_authority.clone(),
                                },
                                &[seeds],
                            ),
                            msol_lamports,
                        )
                    })?;
                    self.state.on_msol_mint(msol_lamports);
                }
                Ok(())
            }
        }
        impl<'info> UpdateActive<'info> {
            /// Compute rewards for a single stake account
            /// take 1% protocol fee for treasury & add the rest to validator_system.total_balance
            /// update mSOL price accordingly
            /// Future optional expansion: Partial: If the stake-account is a fully-deactivated stake account ready to withdraw,
            /// (cool-down period is complete) delete-withdraw the stake-account, send SOL to reserve-account
            pub fn process(&mut self, stake_index: u32, validator_index: u32) -> ProgramResult {
                let BeginOutput {
                    mut stake,
                    is_treasury_msol_ready_for_transfer,
                } = self.begin(stake_index)?;
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.as_ref().borrow(), validator_index)?;
                let delegation = self.stake_account.delegation().ok_or_else(|| {
                    ::solana_program::log::sol_log("Undelegated stake under marinade control!");
                    ProgramError::InvalidAccountData
                })?;
                if delegation.voter_pubkey != validator.validator_account {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Invalid stake validator index. Need to point into validator "],
                            &[::core::fmt::ArgumentV1::new_display(
                                &validator.validator_account,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidInstructionData);
                }
                if delegation.deactivation_epoch != std::u64::MAX {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Cooling down stake ", ". Please use UpdateCoolingDown"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.stake_account.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let delegated_lamports = delegation.stake;
                let stake_balance_without_rent = self.stake_account.to_account_info().lamports()
                    - self.stake_account.meta().unwrap().rent_exempt_reserve;
                let extra_lamports = stake_balance_without_rent.saturating_sub(delegated_lamports);
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Extra lamports in stake balance: "],
                        &[::core::fmt::ArgumentV1::new_display(&extra_lamports)],
                    ));
                    res
                });
                self.withdraw_to_reserve(extra_lamports)?;
                if is_treasury_msol_ready_for_transfer {
                    let msol_amount = self.state.calc_msol_from_lamports(extra_lamports)?;
                    self.mint_to_treasury(msol_amount)?;
                }
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["current staked lamports "],
                        &[::core::fmt::ArgumentV1::new_display(&delegated_lamports)],
                    ));
                    res
                });
                if delegated_lamports >= stake.last_update_delegated_lamports {
                    let rewards = delegated_lamports - stake.last_update_delegated_lamports;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Staking rewards: "],
                            &[::core::fmt::ArgumentV1::new_display(&rewards)],
                        ));
                        res
                    });
                    if is_treasury_msol_ready_for_transfer {
                        let protocol_rewards_fee = self.state.reward_fee.apply(rewards);
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["protocol_rewards_fee "],
                                &[::core::fmt::ArgumentV1::new_display(&protocol_rewards_fee)],
                            ));
                            res
                        });
                        let fee_as_msol_amount =
                            self.state.calc_msol_from_lamports(protocol_rewards_fee)?;
                        self.mint_to_treasury(fee_as_msol_amount)?;
                    }
                    validator.active_balance += rewards;
                    self.state.validator_system.total_active_balance += rewards;
                } else {
                    let slashed = stake.last_update_delegated_lamports - delegated_lamports;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["slashed "],
                            &[::core::fmt::ArgumentV1::new_display(&slashed)],
                        ));
                        res
                    });
                    validator.active_balance = validator.active_balance.saturating_sub(slashed);
                    self.state.validator_system.total_active_balance = self
                        .state
                        .validator_system
                        .total_active_balance
                        .saturating_sub(slashed);
                }
                stake.last_update_epoch = self.clock.epoch;
                stake.last_update_delegated_lamports = delegated_lamports;
                self.state.validator_system.set(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    validator_index,
                    validator,
                )?;
                self.state.msol_price = self
                    .state
                    .calc_lamports_from_msol_amount(State::PRICE_DENOMINATOR)?;
                self.state.stake_system.set(
                    &mut self.stake_list.data.as_ref().borrow_mut(),
                    stake_index,
                    stake,
                )?;
                match (
                    &(self.state.available_reserve_balance + self.state.rent_exempt_for_token_acc),
                    &self.reserve_pda.lamports(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                Ok(())
            }
        }
        impl<'info> UpdateDeactivated<'info> {
            /// Compute rewards for a single deactivated stake-account
            /// take 1% protocol fee for treasury & add the rest to validator_system.total_balance
            /// update mSOL price accordingly
            /// Optional Future Expansion: Partial: If the stake-account is a fully-deactivated stake account ready to withdraw,
            /// (cool-down period is complete) delete-withdraw the stake-account, send SOL to reserve-account
            pub fn process(&mut self, stake_index: u32) -> ProgramResult {
                let BeginOutput {
                    stake,
                    is_treasury_msol_ready_for_transfer,
                } = self.begin(stake_index)?;
                check_address(
                    self.system_program.to_account_info().key,
                    &system_program::ID,
                    "system_program",
                )?;
                self.state
                    .check_operational_sol_account(self.operational_sol_account.key)?;
                let delegation = self
                    .stake_account
                    .delegation()
                    .expect("Undelegated stake under control");
                if delegation.deactivation_epoch == std::u64::MAX {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Stake ", " is active"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.stake_account.to_account_info().key,
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidAccountData);
                }
                let delegated_lamports = delegation.stake;
                let rent = self.stake_account.meta().unwrap().rent_exempt_reserve;
                let stake_balance_without_rent =
                    self.stake_account.to_account_info().lamports() - rent;
                let extra_lamports = stake_balance_without_rent.saturating_sub(delegated_lamports);
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Extra lamports in stake balance: "],
                        &[::core::fmt::ArgumentV1::new_display(&extra_lamports)],
                    ));
                    res
                });
                if is_treasury_msol_ready_for_transfer {
                    let msol_amount = self.state.calc_msol_from_lamports(extra_lamports)?;
                    self.mint_to_treasury(msol_amount)?;
                }
                if delegated_lamports >= stake.last_update_delegated_lamports {
                    let rewards = delegated_lamports - stake.last_update_delegated_lamports;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Staking rewards: "],
                            &[::core::fmt::ArgumentV1::new_display(&rewards)],
                        ));
                        res
                    });
                    if is_treasury_msol_ready_for_transfer {
                        let protocol_rewards_fee = self.state.reward_fee.apply(rewards);
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["protocol_rewards_fee "],
                                &[::core::fmt::ArgumentV1::new_display(&protocol_rewards_fee)],
                            ));
                            res
                        });
                        let fee_as_msol_amount =
                            self.state.calc_msol_from_lamports(protocol_rewards_fee)?;
                        self.mint_to_treasury(fee_as_msol_amount)?;
                    }
                } else {
                    let slashed = stake.last_update_delegated_lamports - delegated_lamports;
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Slashed "],
                            &[::core::fmt::ArgumentV1::new_display(&slashed)],
                        ));
                        res
                    });
                }
                self.common
                    .withdraw_to_reserve(self.stake_account.to_account_info().lamports())?;
                self.state.with_reserve_seeds(|seeds| {
                    invoke_signed(
                        &system_instruction::transfer(
                            self.reserve_pda.key,
                            self.operational_sol_account.key,
                            rent,
                        ),
                        &[
                            self.system_program.clone(),
                            self.reserve_pda.clone(),
                            self.operational_sol_account.clone(),
                        ],
                        &[seeds],
                    )
                })?;
                self.state.on_transfer_from_reserve(rent)?;
                if stake.is_emergency_unstaking == 0 {
                    self.state.stake_system.delayed_unstake_cooling_down = self
                        .state
                        .stake_system
                        .delayed_unstake_cooling_down
                        .checked_sub(stake.last_update_delegated_lamports)
                        .ok_or(CommonError::CalculationFailure)?;
                } else {
                    self.state.emergency_cooling_down = self
                        .state
                        .emergency_cooling_down
                        .checked_sub(stake.last_update_delegated_lamports)
                        .ok_or(CommonError::CalculationFailure)?;
                }
                self.state.msol_price = self
                    .state
                    .calc_lamports_from_msol_amount(State::PRICE_DENOMINATOR)?;
                self.common.state.stake_system.remove(
                    &mut self.common.stake_list.data.as_ref().borrow_mut(),
                    stake_index,
                )?;
                Ok(())
            }
        }
    }
    pub struct State {
        pub msol_mint: Pubkey,
        pub admin_authority: Pubkey,
        pub operational_sol_account: Pubkey,
        pub treasury_msol_account: Pubkey,
        pub reserve_bump_seed: u8,
        pub msol_mint_authority_bump_seed: u8,
        pub rent_exempt_for_token_acc: u64,
        pub reward_fee: Fee,
        pub stake_system: StakeSystem,
        pub validator_system: ValidatorSystem,
        pub liq_pool: LiqPool,
        pub available_reserve_balance: u64,
        pub msol_supply: u64,
        pub msol_price: u64,
        ///count tickets for delayed-unstake
        pub circulating_ticket_count: u64,
        ///total lamports amount of generated and not claimed yet tickets
        pub circulating_ticket_balance: u64,
        pub lent_from_reserve: u64,
        pub min_deposit: u64,
        pub min_withdraw: u64,
        pub staking_sol_cap: u64,
        pub emergency_cooling_down: u64,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for State {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    msol_mint: ref __self_0_0,
                    admin_authority: ref __self_0_1,
                    operational_sol_account: ref __self_0_2,
                    treasury_msol_account: ref __self_0_3,
                    reserve_bump_seed: ref __self_0_4,
                    msol_mint_authority_bump_seed: ref __self_0_5,
                    rent_exempt_for_token_acc: ref __self_0_6,
                    reward_fee: ref __self_0_7,
                    stake_system: ref __self_0_8,
                    validator_system: ref __self_0_9,
                    liq_pool: ref __self_0_10,
                    available_reserve_balance: ref __self_0_11,
                    msol_supply: ref __self_0_12,
                    msol_price: ref __self_0_13,
                    circulating_ticket_count: ref __self_0_14,
                    circulating_ticket_balance: ref __self_0_15,
                    lent_from_reserve: ref __self_0_16,
                    min_deposit: ref __self_0_17,
                    min_withdraw: ref __self_0_18,
                    staking_sol_cap: ref __self_0_19,
                    emergency_cooling_down: ref __self_0_20,
                } => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "State");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_mint",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "admin_authority",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "operational_sol_account",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "treasury_msol_account",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "reserve_bump_seed",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_mint_authority_bump_seed",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rent_exempt_for_token_acc",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "reward_fee",
                        &&(*__self_0_7),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "stake_system",
                        &&(*__self_0_8),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "validator_system",
                        &&(*__self_0_9),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "liq_pool",
                        &&(*__self_0_10),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "available_reserve_balance",
                        &&(*__self_0_11),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_supply",
                        &&(*__self_0_12),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "msol_price",
                        &&(*__self_0_13),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "circulating_ticket_count",
                        &&(*__self_0_14),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "circulating_ticket_balance",
                        &&(*__self_0_15),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lent_from_reserve",
                        &&(*__self_0_16),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "min_deposit",
                        &&(*__self_0_17),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "min_withdraw",
                        &&(*__self_0_18),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "staking_sol_cap",
                        &&(*__self_0_19),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "emergency_cooling_down",
                        &&(*__self_0_20),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl borsh::ser::BorshSerialize for State
    where
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Fee: borsh::ser::BorshSerialize,
        StakeSystem: borsh::ser::BorshSerialize,
        ValidatorSystem: borsh::ser::BorshSerialize,
        LiqPool: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.admin_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.treasury_msol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint_authority_bump_seed, writer)?;
            borsh::BorshSerialize::serialize(&self.rent_exempt_for_token_acc, writer)?;
            borsh::BorshSerialize::serialize(&self.reward_fee, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_system, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_system, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool, writer)?;
            borsh::BorshSerialize::serialize(&self.available_reserve_balance, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_supply, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_price, writer)?;
            borsh::BorshSerialize::serialize(&self.circulating_ticket_count, writer)?;
            borsh::BorshSerialize::serialize(&self.circulating_ticket_balance, writer)?;
            borsh::BorshSerialize::serialize(&self.lent_from_reserve, writer)?;
            borsh::BorshSerialize::serialize(&self.min_deposit, writer)?;
            borsh::BorshSerialize::serialize(&self.min_withdraw, writer)?;
            borsh::BorshSerialize::serialize(&self.staking_sol_cap, writer)?;
            borsh::BorshSerialize::serialize(&self.emergency_cooling_down, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for State
    where
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Fee: borsh::BorshDeserialize,
        StakeSystem: borsh::BorshDeserialize,
        ValidatorSystem: borsh::BorshDeserialize,
        LiqPool: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                msol_mint: borsh::BorshDeserialize::deserialize(buf)?,
                admin_authority: borsh::BorshDeserialize::deserialize(buf)?,
                operational_sol_account: borsh::BorshDeserialize::deserialize(buf)?,
                treasury_msol_account: borsh::BorshDeserialize::deserialize(buf)?,
                reserve_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                msol_mint_authority_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
                rent_exempt_for_token_acc: borsh::BorshDeserialize::deserialize(buf)?,
                reward_fee: borsh::BorshDeserialize::deserialize(buf)?,
                stake_system: borsh::BorshDeserialize::deserialize(buf)?,
                validator_system: borsh::BorshDeserialize::deserialize(buf)?,
                liq_pool: borsh::BorshDeserialize::deserialize(buf)?,
                available_reserve_balance: borsh::BorshDeserialize::deserialize(buf)?,
                msol_supply: borsh::BorshDeserialize::deserialize(buf)?,
                msol_price: borsh::BorshDeserialize::deserialize(buf)?,
                circulating_ticket_count: borsh::BorshDeserialize::deserialize(buf)?,
                circulating_ticket_balance: borsh::BorshDeserialize::deserialize(buf)?,
                lent_from_reserve: borsh::BorshDeserialize::deserialize(buf)?,
                min_deposit: borsh::BorshDeserialize::deserialize(buf)?,
                min_withdraw: borsh::BorshDeserialize::deserialize(buf)?,
                staking_sol_cap: borsh::BorshDeserialize::deserialize(buf)?,
                emergency_cooling_down: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for State {
        #[inline]
        fn clone(&self) -> State {
            match *self {
                Self {
                    msol_mint: ref __self_0_0,
                    admin_authority: ref __self_0_1,
                    operational_sol_account: ref __self_0_2,
                    treasury_msol_account: ref __self_0_3,
                    reserve_bump_seed: ref __self_0_4,
                    msol_mint_authority_bump_seed: ref __self_0_5,
                    rent_exempt_for_token_acc: ref __self_0_6,
                    reward_fee: ref __self_0_7,
                    stake_system: ref __self_0_8,
                    validator_system: ref __self_0_9,
                    liq_pool: ref __self_0_10,
                    available_reserve_balance: ref __self_0_11,
                    msol_supply: ref __self_0_12,
                    msol_price: ref __self_0_13,
                    circulating_ticket_count: ref __self_0_14,
                    circulating_ticket_balance: ref __self_0_15,
                    lent_from_reserve: ref __self_0_16,
                    min_deposit: ref __self_0_17,
                    min_withdraw: ref __self_0_18,
                    staking_sol_cap: ref __self_0_19,
                    emergency_cooling_down: ref __self_0_20,
                } => State {
                    msol_mint: ::core::clone::Clone::clone(&(*__self_0_0)),
                    admin_authority: ::core::clone::Clone::clone(&(*__self_0_1)),
                    operational_sol_account: ::core::clone::Clone::clone(&(*__self_0_2)),
                    treasury_msol_account: ::core::clone::Clone::clone(&(*__self_0_3)),
                    reserve_bump_seed: ::core::clone::Clone::clone(&(*__self_0_4)),
                    msol_mint_authority_bump_seed: ::core::clone::Clone::clone(&(*__self_0_5)),
                    rent_exempt_for_token_acc: ::core::clone::Clone::clone(&(*__self_0_6)),
                    reward_fee: ::core::clone::Clone::clone(&(*__self_0_7)),
                    stake_system: ::core::clone::Clone::clone(&(*__self_0_8)),
                    validator_system: ::core::clone::Clone::clone(&(*__self_0_9)),
                    liq_pool: ::core::clone::Clone::clone(&(*__self_0_10)),
                    available_reserve_balance: ::core::clone::Clone::clone(&(*__self_0_11)),
                    msol_supply: ::core::clone::Clone::clone(&(*__self_0_12)),
                    msol_price: ::core::clone::Clone::clone(&(*__self_0_13)),
                    circulating_ticket_count: ::core::clone::Clone::clone(&(*__self_0_14)),
                    circulating_ticket_balance: ::core::clone::Clone::clone(&(*__self_0_15)),
                    lent_from_reserve: ::core::clone::Clone::clone(&(*__self_0_16)),
                    min_deposit: ::core::clone::Clone::clone(&(*__self_0_17)),
                    min_withdraw: ::core::clone::Clone::clone(&(*__self_0_18)),
                    staking_sol_cap: ::core::clone::Clone::clone(&(*__self_0_19)),
                    emergency_cooling_down: ::core::clone::Clone::clone(&(*__self_0_20)),
                },
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for State {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> std::result::Result<(), ProgramError> {
            writer
                .write_all(&[216, 146, 107, 94, 104, 75, 182, 177])
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            AnchorSerialize::serialize(self, writer)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for State {
        fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            if buf.len() < [216, 146, 107, 94, 104, 75, 182, 177].len() {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..8];
            if &[216, 146, 107, 94, 104, 75, 182, 177] != given_disc {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorMismatch.into());
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            let mut data: &[u8] = &buf[8..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for State {
        fn discriminator() -> [u8; 8] {
            [216, 146, 107, 94, 104, 75, 182, 177]
        }
    }
    impl State {
        pub const PRICE_DENOMINATOR: u64 = 0x1_0000_0000;
        /// Suffix for reserve account seed
        pub const RESERVE_SEED: &'static [u8] = b"reserve";
        pub const MSOL_MINT_AUTHORITY_SEED: &'static [u8] = b"st_mint";
        pub const STAKE_LIST_SEED: &'static str = "stake_list";
        pub const VALIDATOR_LIST_SEED: &'static str = "validator_list";
        pub fn serialized_len() -> usize {
            unsafe { MaybeUninit::<Self>::zeroed().assume_init() }
                .try_to_vec()
                .unwrap()
                .len()
                + 8
        }
        pub fn find_msol_mint_authority(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(
                &[&state.to_bytes()[..32], State::MSOL_MINT_AUTHORITY_SEED],
                &ID,
            )
        }
        pub fn find_reserve_address(state: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[&state.to_bytes()[..32], Self::RESERVE_SEED], &ID)
        }
        pub fn default_stake_list_address(state: &Pubkey) -> Pubkey {
            Pubkey::create_with_seed(state, Self::STAKE_LIST_SEED, &ID).unwrap()
        }
        pub fn default_validator_list_address(state: &Pubkey) -> Pubkey {
            Pubkey::create_with_seed(state, Self::VALIDATOR_LIST_SEED, &ID).unwrap()
        }
        pub fn check_admin_authority(&self, admin_authority: &Pubkey) -> ProgramResult {
            check_address(admin_authority, &self.admin_authority, "admin_authority")?;
            Ok(())
        }
        pub fn check_operational_sol_account(
            &self,
            operational_sol_account: &Pubkey,
        ) -> ProgramResult {
            check_address(
                operational_sol_account,
                &self.operational_sol_account,
                "operational_sol_account",
            )
        }
        pub fn check_treasury_msol_account<'info>(
            &self,
            treasury_msol_account: &AccountInfo<'info>,
        ) -> Result<bool, ProgramError> {
            check_address(
                treasury_msol_account.key,
                &self.treasury_msol_account,
                "treasury_msol_account",
            )?;
            if treasury_msol_account.owner != &spl_token::ID {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["treasury_msol_account ", " is not a token account"],
                        &[::core::fmt::ArgumentV1::new_display(
                            &treasury_msol_account.key,
                        )],
                    ));
                    res
                });
                return Ok(false);
            }
            match spl_token::state::Account::unpack(treasury_msol_account.data.borrow().as_ref()) {
                Ok(token_account) => {
                    if token_account.mint == self.msol_mint {
                        Ok(true)
                    } else {
                        ::solana_program::log::sol_log(&{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["treasury_msol_account ", " has wrong mint ", ". Expected "],
                                &[
                                    ::core::fmt::ArgumentV1::new_display(
                                        &treasury_msol_account.key,
                                    ),
                                    ::core::fmt::ArgumentV1::new_display(&token_account.mint),
                                    ::core::fmt::ArgumentV1::new_display(&self.msol_mint),
                                ],
                            ));
                            res
                        });
                        Ok(false)
                    }
                }
                Err(e) => {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "treasury_msol_account ",
                                " can not be parsed as token account (",
                                ")",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&treasury_msol_account.key),
                                ::core::fmt::ArgumentV1::new_display(&e),
                            ],
                        ));
                        res
                    });
                    Ok(false)
                }
            }
        }
        pub fn check_msol_mint(&mut self, msol_mint: &Pubkey) -> ProgramResult {
            check_address(msol_mint, &self.msol_mint, "msol_mint")
        }
        pub fn total_cooling_down(&self) -> u64 {
            self.stake_system
                .delayed_unstake_cooling_down
                .checked_add(self.emergency_cooling_down)
                .expect("Total cooling down overflow")
        }
        /// total_active_balance + total_cooling_down + available_reserve_balance
        pub fn total_lamports_under_control(&self) -> u64 {
            self.validator_system
                .total_active_balance
                .checked_add(self.total_cooling_down())
                .expect("Stake balance overflow")
                .checked_add(self.available_reserve_balance)
                .expect("Total SOLs under control overflow")
        }
        pub fn check_staking_cap(&self, transfering_lamports: u64) -> ProgramResult {
            let result_amount = self
                .total_lamports_under_control()
                .checked_add(transfering_lamports)
                .ok_or_else(|| {
                    ::solana_program::log::sol_log("SOL overflow");
                    ProgramError::InvalidArgument
                })?;
            if result_amount > self.staking_sol_cap {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Staking cap reached ", "/"],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&result_amount),
                            ::core::fmt::ArgumentV1::new_display(&self.staking_sol_cap),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::Custom(3782));
            }
            Ok(())
        }
        pub fn total_virtual_staked_lamports(&self) -> u64 {
            self.total_lamports_under_control()
                .saturating_sub(self.circulating_ticket_balance)
        }
        /// calculate the amount of msol tokens corresponding to certain lamport amount
        pub fn calc_msol_from_lamports(&self, stake_lamports: u64) -> Result<u64, CommonError> {
            shares_from_value(
                stake_lamports,
                self.total_virtual_staked_lamports(),
                self.msol_supply,
            )
        }
        /// calculate lamports value from some msol_amount
        /// result_lamports = msol_amount * msol_price
        pub fn calc_lamports_from_msol_amount(&self, msol_amount: u64) -> Result<u64, CommonError> {
            value_from_shares(
                msol_amount,
                self.total_virtual_staked_lamports(),
                self.msol_supply,
            )
        }
        pub fn stake_delta(&self, reserve_balance: u64) -> i128 {
            let raw = reserve_balance.saturating_sub(self.rent_exempt_for_token_acc) as i128
                + self.stake_system.delayed_unstake_cooling_down as i128
                - self.circulating_ticket_balance as i128;
            if raw >= 0 {
                raw
            } else {
                let with_emergency = raw + self.emergency_cooling_down as i128;
                with_emergency.min(0)
            }
        }
        pub fn on_transfer_to_reserve(&mut self, amount: u64) {
            self.available_reserve_balance = self
                .available_reserve_balance
                .checked_add(amount)
                .expect("reserve balance overflow");
        }
        pub fn on_transfer_from_reserve(&mut self, amount: u64) -> ProgramResult {
            self.available_reserve_balance = self
                .available_reserve_balance
                .checked_sub(amount)
                .ok_or(CommonError::CalculationFailure)?;
            Ok(())
        }
        pub fn on_msol_mint(&mut self, amount: u64) {
            self.msol_supply = self
                .msol_supply
                .checked_add(amount)
                .expect("msol supply overflow");
        }
        pub fn on_msol_burn(&mut self, amount: u64) -> ProgramResult {
            self.msol_supply = self
                .msol_supply
                .checked_sub(amount)
                .ok_or(CommonError::CalculationFailure)?;
            Ok(())
        }
    }
    pub trait StateHelpers {
        fn msol_mint_authority(&self) -> Pubkey;
        fn with_msol_mint_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn reserve_address(&self) -> Pubkey;
        fn with_reserve_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R;
        fn check_reserve_address(&self, reserve: &Pubkey) -> ProgramResult;
        fn check_msol_mint_authority(&self, msol_mint_authority: &Pubkey) -> ProgramResult;
    }
    impl<T> StateHelpers for T
    where
        T: Located<State>,
    {
        fn msol_mint_authority(&self) -> Pubkey {
            self.with_msol_mint_authority_seeds(|seeds| {
                Pubkey::create_program_address(seeds, &ID).unwrap()
            })
        }
        fn with_msol_mint_authority_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                State::MSOL_MINT_AUTHORITY_SEED,
                &[self.as_ref().msol_mint_authority_bump_seed],
            ])
        }
        fn reserve_address(&self) -> Pubkey {
            self.with_reserve_seeds(|seeds| Pubkey::create_program_address(seeds, &ID).unwrap())
        }
        fn with_reserve_seeds<R, F: FnOnce(&[&[u8]]) -> R>(&self, f: F) -> R {
            f(&[
                &self.key().to_bytes()[..32],
                State::RESERVE_SEED,
                &[self.as_ref().reserve_bump_seed],
            ])
        }
        fn check_reserve_address(&self, reserve: &Pubkey) -> ProgramResult {
            check_address(reserve, &self.reserve_address(), "reserve")
        }
        fn check_msol_mint_authority(&self, msol_mint_authority: &Pubkey) -> ProgramResult {
            check_address(
                msol_mint_authority,
                &self.msol_mint_authority(),
                "msol_mint_authority",
            )
        }
    }
}
pub mod ticket_account {
    use anchor_lang::prelude::*;
    pub struct TicketAccountData {
        pub state_address: Pubkey,
        pub beneficiary: Pubkey,
        pub lamports_amount: u64,
        pub created_epoch: u64,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TicketAccountData {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    state_address: ref __self_0_0,
                    beneficiary: ref __self_0_1,
                    lamports_amount: ref __self_0_2,
                    created_epoch: ref __self_0_3,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "TicketAccountData");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "state_address",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "beneficiary",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "lamports_amount",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "created_epoch",
                        &&(*__self_0_3),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl borsh::ser::BorshSerialize for TicketAccountData
    where
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state_address, writer)?;
            borsh::BorshSerialize::serialize(&self.beneficiary, writer)?;
            borsh::BorshSerialize::serialize(&self.lamports_amount, writer)?;
            borsh::BorshSerialize::serialize(&self.created_epoch, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for TicketAccountData
    where
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                state_address: borsh::BorshDeserialize::deserialize(buf)?,
                beneficiary: borsh::BorshDeserialize::deserialize(buf)?,
                lamports_amount: borsh::BorshDeserialize::deserialize(buf)?,
                created_epoch: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TicketAccountData {
        #[inline]
        fn clone(&self) -> TicketAccountData {
            match *self {
                Self {
                    state_address: ref __self_0_0,
                    beneficiary: ref __self_0_1,
                    lamports_amount: ref __self_0_2,
                    created_epoch: ref __self_0_3,
                } => TicketAccountData {
                    state_address: ::core::clone::Clone::clone(&(*__self_0_0)),
                    beneficiary: ::core::clone::Clone::clone(&(*__self_0_1)),
                    lamports_amount: ::core::clone::Clone::clone(&(*__self_0_2)),
                    created_epoch: ::core::clone::Clone::clone(&(*__self_0_3)),
                },
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for TicketAccountData {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> std::result::Result<(), ProgramError> {
            writer
                .write_all(&[133, 77, 18, 98, 211, 1, 231, 3])
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            AnchorSerialize::serialize(self, writer)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for TicketAccountData {
        fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            if buf.len() < [133, 77, 18, 98, 211, 1, 231, 3].len() {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..8];
            if &[133, 77, 18, 98, 211, 1, 231, 3] != given_disc {
                return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorMismatch.into());
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
            let mut data: &[u8] = &buf[8..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for TicketAccountData {
        fn discriminator() -> [u8; 8] {
            [133, 77, 18, 98, 211, 1, 231, 3]
        }
    }
}
pub mod validator_system {
    use crate::{calc::proportional, checks::check_address, error::CommonError, list::List, ID};
    use anchor_lang::prelude::*;
    pub mod add {
        use anchor_lang::prelude::*;
        use anchor_lang::solana_program::{program::invoke_signed, system_instruction, system_program};
        use crate::{
            checks::{check_address, check_owner_program},
            AddValidator, ID,
        };
        impl<'info> AddValidator<'info> {
            pub fn process(&mut self, score: u32) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.manager_authority.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                check_owner_program(
                    &self.duplication_flag,
                    &system_program::ID,
                    "duplication_flag",
                )?;
                check_owner_program(&self.rent_payer, &system_program::ID, "rent_payer")?;
                if !self.rent.is_exempt(self.rent_payer.lamports(), 0) {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Rent payer must have at least ", " lamports"],
                            &[::core::fmt::ArgumentV1::new_display(
                                &self.rent.minimum_balance(0),
                            )],
                        ));
                        res
                    });
                    return Err(ProgramError::InsufficientFunds);
                }
                check_address(
                    self.system_program.key,
                    &system_program::ID,
                    "system_program",
                )?;
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Add validator "],
                        &[::core::fmt::ArgumentV1::new_display(
                            &self.validator_vote.key,
                        )],
                    ));
                    res
                });
                let state_address = *self.state.to_account_info().key;
                self.state.validator_system.add(
                    &mut self.validator_list.data.borrow_mut(),
                    *self.validator_vote.key,
                    score,
                    &state_address,
                    self.duplication_flag.key,
                )?;
                let validator_record = self.state.validator_system.get(
                    &self.validator_list.data.borrow(),
                    self.state.validator_system.validator_count() - 1,
                )?;
                validator_record.with_duplication_flag_seeds(
                    self.state.to_account_info().key,
                    |seeds| {
                        invoke_signed(
                            &system_instruction::create_account(
                                self.rent_payer.key,
                                self.duplication_flag.key,
                                self.rent.minimum_balance(0),
                                0,
                                &ID,
                            ),
                            &[
                                self.system_program.clone(),
                                self.rent_payer.clone(),
                                self.duplication_flag.clone(),
                            ],
                            &[seeds],
                        )
                    },
                )?;
                Ok(())
            }
        }
    }
    pub mod config_validator_system {
        use anchor_lang::prelude::*;
        impl<'info> crate::ConfigValidatorSystem<'info> {
            pub fn process(&mut self, extra_runs: u32) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.manager_authority.key)?;
                self.state.stake_system.extra_stake_delta_runs = extra_runs;
                Ok(())
            }
        }
    }
    pub mod remove {
        use anchor_lang::prelude::*;
        use crate::RemoveValidator;
        impl<'info> RemoveValidator<'info> {
            pub fn process(&mut self, index: u32, validator_vote: Pubkey) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.manager_authority.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                self.state
                    .check_operational_sol_account(self.operational_sol_account.key)?;
                let validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.borrow(), index)?;
                if validator.validator_account != validator_vote {
                    ::solana_program::log::sol_log("Removing validator index is wrong");
                    return Err(ProgramError::InvalidArgument);
                }
                if self.duplication_flag.key
                    != &validator.duplication_flag_address(self.state.to_account_info().key)
                {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Invalid duplication flag ", ". Expected "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.duplication_flag.key),
                                ::core::fmt::ArgumentV1::new_display(
                                    &validator
                                        .duplication_flag_address(self.state.to_account_info().key),
                                ),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                self.state.validator_system.remove(
                    &mut self.validator_list.data.as_ref().borrow_mut(),
                    index,
                    validator,
                )?;
                let rent_return = self.duplication_flag.lamports();
                **self.duplication_flag.try_borrow_mut_lamports()? = 0;
                **self.operational_sol_account.try_borrow_mut_lamports()? += rent_return;
                Ok(())
            }
        }
    }
    pub mod set_score {
        use anchor_lang::prelude::*;
        use crate::{error::CommonError, SetValidatorScore};
        impl<'info> SetValidatorScore<'info> {
            pub fn process(
                &mut self,
                index: u32,
                validator_vote: Pubkey,
                score: u32,
            ) -> ProgramResult {
                self.state
                    .validator_system
                    .check_validator_manager_authority(self.manager_authority.key)?;
                self.state
                    .validator_system
                    .check_validator_list(&self.validator_list)?;
                let mut validator = self
                    .state
                    .validator_system
                    .get(&self.validator_list.data.borrow(), index)?;
                if validator.validator_account != validator_vote {
                    ::solana_program::log::sol_log(&{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Wrong validator ", ". Validator #", " must be "],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&validator_vote),
                                ::core::fmt::ArgumentV1::new_display(&index),
                                ::core::fmt::ArgumentV1::new_display(&validator.validator_account),
                            ],
                        ));
                        res
                    });
                    return Err(ProgramError::InvalidArgument);
                }
                self.state.validator_system.total_validator_score = self
                    .state
                    .validator_system
                    .total_validator_score
                    .checked_sub(validator.score)
                    .ok_or(CommonError::CalculationFailure)?;
                validator.score = score;
                self.state.validator_system.total_validator_score = self
                    .state
                    .validator_system
                    .total_validator_score
                    .checked_add(score)
                    .ok_or(CommonError::CalculationFailure)?;
                self.state.validator_system.set(
                    &mut self.validator_list.data.borrow_mut(),
                    index,
                    validator,
                )?;
                Ok(())
            }
        }
    }
    pub struct ValidatorRecord {
        /// Validator vote pubkey
        pub validator_account: Pubkey,
        /// Validator total balance in lamports
        pub active_balance: u64,
        pub score: u32,
        pub last_stake_delta_epoch: u64,
        pub duplication_flag_bump_seed: u8,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ValidatorRecord {
        #[inline]
        fn clone(&self) -> ValidatorRecord {
            {
                let _: ::core::clone::AssertParamIsClone<Pubkey>;
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u64>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ValidatorRecord {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ValidatorRecord {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    validator_account: ref __self_0_0,
                    active_balance: ref __self_0_1,
                    score: ref __self_0_2,
                    last_stake_delta_epoch: ref __self_0_3,
                    duplication_flag_bump_seed: ref __self_0_4,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "ValidatorRecord");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "validator_account",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "active_balance",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "score",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "last_stake_delta_epoch",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "duplication_flag_bump_seed",
                        &&(*__self_0_4),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for ValidatorRecord {
        #[inline]
        fn default() -> ValidatorRecord {
            ValidatorRecord {
                validator_account: ::core::default::Default::default(),
                active_balance: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
                last_stake_delta_epoch: ::core::default::Default::default(),
                duplication_flag_bump_seed: ::core::default::Default::default(),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for ValidatorRecord {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ValidatorRecord {
        #[inline]
        fn eq(&self, other: &ValidatorRecord) -> bool {
            match *other {
                Self {
                    validator_account: ref __self_1_0,
                    active_balance: ref __self_1_1,
                    score: ref __self_1_2,
                    last_stake_delta_epoch: ref __self_1_3,
                    duplication_flag_bump_seed: ref __self_1_4,
                } => match *self {
                    Self {
                        validator_account: ref __self_0_0,
                        active_balance: ref __self_0_1,
                        score: ref __self_0_2,
                        last_stake_delta_epoch: ref __self_0_3,
                        duplication_flag_bump_seed: ref __self_0_4,
                    } => {
                        (*__self_0_0) == (*__self_1_0)
                            && (*__self_0_1) == (*__self_1_1)
                            && (*__self_0_2) == (*__self_1_2)
                            && (*__self_0_3) == (*__self_1_3)
                            && (*__self_0_4) == (*__self_1_4)
                    }
                },
            }
        }
        #[inline]
        fn ne(&self, other: &ValidatorRecord) -> bool {
            match *other {
                Self {
                    validator_account: ref __self_1_0,
                    active_balance: ref __self_1_1,
                    score: ref __self_1_2,
                    last_stake_delta_epoch: ref __self_1_3,
                    duplication_flag_bump_seed: ref __self_1_4,
                } => match *self {
                    Self {
                        validator_account: ref __self_0_0,
                        active_balance: ref __self_0_1,
                        score: ref __self_0_2,
                        last_stake_delta_epoch: ref __self_0_3,
                        duplication_flag_bump_seed: ref __self_0_4,
                    } => {
                        (*__self_0_0) != (*__self_1_0)
                            || (*__self_0_1) != (*__self_1_1)
                            || (*__self_0_2) != (*__self_1_2)
                            || (*__self_0_3) != (*__self_1_3)
                            || (*__self_0_4) != (*__self_1_4)
                    }
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for ValidatorRecord
    where
        Pubkey: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.validator_account, writer)?;
            borsh::BorshSerialize::serialize(&self.active_balance, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            borsh::BorshSerialize::serialize(&self.last_stake_delta_epoch, writer)?;
            borsh::BorshSerialize::serialize(&self.duplication_flag_bump_seed, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ValidatorRecord
    where
        Pubkey: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                validator_account: borsh::BorshDeserialize::deserialize(buf)?,
                active_balance: borsh::BorshDeserialize::deserialize(buf)?,
                score: borsh::BorshDeserialize::deserialize(buf)?,
                last_stake_delta_epoch: borsh::BorshDeserialize::deserialize(buf)?,
                duplication_flag_bump_seed: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl ValidatorRecord {
        pub const DISCRIMINATOR: &'static [u8; 8] = b"validatr";
        pub const DUPLICATE_FLAG_SEED: &'static [u8] = b"unique_validator";
        pub fn find_duplication_flag(state: &Pubkey, validator_account: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(
                &[
                    &state.to_bytes()[..32],
                    Self::DUPLICATE_FLAG_SEED,
                    &validator_account.to_bytes()[..32],
                ],
                &ID,
            )
        }
        pub fn with_duplication_flag_seeds<R, F: FnOnce(&[&[u8]]) -> R>(
            &self,
            state: &Pubkey,
            f: F,
        ) -> R {
            f(&[
                &state.to_bytes()[..32],
                Self::DUPLICATE_FLAG_SEED,
                &self.validator_account.to_bytes()[..32],
                &[self.duplication_flag_bump_seed],
            ])
        }
        pub fn duplication_flag_address(&self, state: &Pubkey) -> Pubkey {
            self.with_duplication_flag_seeds(state, |seeds| {
                Pubkey::create_program_address(seeds, &ID)
            })
            .unwrap()
        }
        pub fn new(
            validator_account: Pubkey,
            score: u32,
            state: &Pubkey,
            duplication_flag_address: &Pubkey,
        ) -> Result<Self, ProgramError> {
            let (actual_duplication_flag, duplication_flag_bump_seed) =
                Self::find_duplication_flag(state, &validator_account);
            if duplication_flag_address != &actual_duplication_flag {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Duplication flag ", " does not match "],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&duplication_flag_address),
                            ::core::fmt::ArgumentV1::new_display(&actual_duplication_flag),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::InvalidArgument);
            }
            Ok(Self {
                validator_account,
                active_balance: 0,
                score,
                last_stake_delta_epoch: std::u64::MAX,
                duplication_flag_bump_seed,
            })
        }
    }
    pub struct ValidatorSystem {
        pub validator_list: List,
        pub manager_authority: Pubkey,
        pub total_validator_score: u32,
        /// sum of all active lamports staked
        pub total_active_balance: u64,
        /// allow & auto-add validator when a user deposits a stake-account of a non-listed validator
        pub auto_add_validator_enabled: u8,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ValidatorSystem {
        #[inline]
        fn clone(&self) -> ValidatorSystem {
            match *self {
                Self {
                    validator_list: ref __self_0_0,
                    manager_authority: ref __self_0_1,
                    total_validator_score: ref __self_0_2,
                    total_active_balance: ref __self_0_3,
                    auto_add_validator_enabled: ref __self_0_4,
                } => ValidatorSystem {
                    validator_list: ::core::clone::Clone::clone(&(*__self_0_0)),
                    manager_authority: ::core::clone::Clone::clone(&(*__self_0_1)),
                    total_validator_score: ::core::clone::Clone::clone(&(*__self_0_2)),
                    total_active_balance: ::core::clone::Clone::clone(&(*__self_0_3)),
                    auto_add_validator_enabled: ::core::clone::Clone::clone(&(*__self_0_4)),
                },
            }
        }
    }
    impl borsh::ser::BorshSerialize for ValidatorSystem
    where
        List: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.total_validator_score, writer)?;
            borsh::BorshSerialize::serialize(&self.total_active_balance, writer)?;
            borsh::BorshSerialize::serialize(&self.auto_add_validator_enabled, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ValidatorSystem
    where
        List: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                validator_list: borsh::BorshDeserialize::deserialize(buf)?,
                manager_authority: borsh::BorshDeserialize::deserialize(buf)?,
                total_validator_score: borsh::BorshDeserialize::deserialize(buf)?,
                total_active_balance: borsh::BorshDeserialize::deserialize(buf)?,
                auto_add_validator_enabled: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ValidatorSystem {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Self {
                    validator_list: ref __self_0_0,
                    manager_authority: ref __self_0_1,
                    total_validator_score: ref __self_0_2,
                    total_active_balance: ref __self_0_3,
                    auto_add_validator_enabled: ref __self_0_4,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "ValidatorSystem");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "validator_list",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "manager_authority",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "total_validator_score",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "total_active_balance",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "auto_add_validator_enabled",
                        &&(*__self_0_4),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    impl ValidatorSystem {
        pub fn bytes_for_list(count: u32, additional_record_space: u32) -> u32 {
            List::bytes_for(
                ValidatorRecord::default().try_to_vec().unwrap().len() as u32
                    + additional_record_space,
                count,
            )
        }
        pub fn new(
            validator_list_account: Pubkey,
            validator_list_data: &mut [u8],
            manager_authority: Pubkey,
            additional_record_space: u32,
        ) -> Result<Self, ProgramError> {
            Ok(Self {
                validator_list: List::new(
                    ValidatorRecord::DISCRIMINATOR,
                    ValidatorRecord::default().try_to_vec().unwrap().len() as u32
                        + additional_record_space,
                    validator_list_account,
                    validator_list_data,
                    "validator_list",
                )?,
                manager_authority,
                total_validator_score: 0,
                total_active_balance: 0,
                auto_add_validator_enabled: 0,
            })
        }
        pub fn validator_list_address(&self) -> &Pubkey {
            &self.validator_list.account
        }
        pub fn validator_count(&self) -> u32 {
            self.validator_list.len()
        }
        pub fn validator_list_capacity(
            &self,
            validator_list_len: usize,
        ) -> Result<u32, ProgramError> {
            self.validator_list.capacity(validator_list_len)
        }
        pub fn validator_record_size(&self) -> u32 {
            self.validator_list.item_size()
        }
        pub fn add(
            &mut self,
            validator_list_data: &mut [u8],
            validator_account: Pubkey,
            score: u32,
            state: &Pubkey,
            duplication_flag_address: &Pubkey,
        ) -> ProgramResult {
            self.validator_list.push(
                validator_list_data,
                ValidatorRecord::new(validator_account, score, state, duplication_flag_address)?,
                "validator_list",
            )?;
            self.total_validator_score += score as u32;
            Ok(())
        }
        pub fn add_with_balance(
            &mut self,
            validator_list_data: &mut [u8],
            validator_account: Pubkey,
            score: u32,
            balance: u64,
            state: &Pubkey,
            duplication_flag_address: &Pubkey,
        ) -> ProgramResult {
            let mut validator =
                ValidatorRecord::new(validator_account, score, state, duplication_flag_address)?;
            validator.active_balance = balance;
            self.validator_list
                .push(validator_list_data, validator, "validator_list")?;
            self.total_validator_score += score as u32;
            Ok(())
        }
        pub fn remove(
            &mut self,
            validator_list_data: &mut [u8],
            index: u32,
            record: ValidatorRecord,
        ) -> ProgramResult {
            if record.active_balance > 0 {
                ::solana_program::log::sol_log(&{
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Can not remove validator ", " with balance "],
                        &[
                            ::core::fmt::ArgumentV1::new_display(&record.validator_account),
                            ::core::fmt::ArgumentV1::new_display(&record.active_balance),
                        ],
                    ));
                    res
                });
                return Err(ProgramError::InvalidInstructionData);
            }
            self.total_validator_score = self
                .total_validator_score
                .checked_sub(record.score)
                .ok_or(CommonError::CalculationFailure)?;
            self.validator_list
                .remove(validator_list_data, index, "validator_list")?;
            Ok(())
        }
        pub fn get(
            &self,
            validator_list_data: &[u8],
            index: u32,
        ) -> Result<ValidatorRecord, ProgramError> {
            self.validator_list
                .get(validator_list_data, index, "validator_list")
        }
        pub fn set(
            &self,
            validator_list_data: &mut [u8],
            index: u32,
            validator_record: ValidatorRecord,
        ) -> ProgramResult {
            self.validator_list.set(
                validator_list_data,
                index,
                validator_record,
                "validator_list",
            )
        }
        pub fn validator_stake_target(
            &self,
            validator: &ValidatorRecord,
            total_stake_target: u64,
        ) -> Result<u64, CommonError> {
            if self.total_validator_score == 0 {
                return Ok(0);
            }
            proportional(
                total_stake_target,
                validator.score as u64,
                self.total_validator_score as u64,
            )
        }
        pub fn check_validator_list<'info>(
            &self,
            validator_list: &AccountInfo<'info>,
        ) -> ProgramResult {
            check_address(
                validator_list.key,
                self.validator_list_address(),
                "validator_list",
            )?;
            if &validator_list.data.borrow().as_ref()[0..8] != ValidatorRecord::DISCRIMINATOR {
                ::solana_program::log::sol_log("Wrong validator list account discriminator");
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(())
        }
        pub fn check_validator_manager_authority(
            &self,
            manager_authority: &Pubkey,
        ) -> ProgramResult {
            check_address(
                manager_authority,
                &self.manager_authority,
                "validator_manager_authority",
            )
        }
    }
}
pub use state::State;
/// The static program ID
pub static ID: Pubkey = Pubkey::new_from_array([
    5, 69, 227, 101, 190, 242, 113, 173, 117, 53, 3, 103, 86, 93, 164, 13, 163, 54, 220, 28, 135,
    155, 177, 84, 138, 122, 252, 197, 90, 169, 57, 30,
]);
/// Confirms that a given pubkey is equivalent to the program ID
pub fn check_id(id: &Pubkey) -> bool {
    id == &ID
}
/// Returns the program ID
pub fn id() -> Pubkey {
    ID
}
pub const MAX_REWARD_FEE: u32 = 1_000;
fn check_context<T>(ctx: &Context<T>) -> ProgramResult {
    if !check_id(ctx.program_id) {
        return Err(CommonError::InvalidProgramId.into());
    }
    if !ctx.remaining_accounts.is_empty() {
        return Err(CommonError::UnexpectedAccount.into());
    }
    Ok(())
}
use marinade_finance::*;
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) =
        unsafe { ::solana_program::entrypoint::deserialize(input) };
    match entry(&program_id, &accounts, &instruction_data) {
        Ok(()) => ::solana_program::entrypoint::SUCCESS,
        Err(error) => error.into(),
    }
}
/// The Anchor codegen exposes a programming model where a user defines
/// a set of methods inside of a `#[program]` module in a way similar
/// to writing RPC request handlers. The macro then generates a bunch of
/// code wrapping these user defined methods into something that can be
/// executed on Solana.
///
/// These methods fall into one of three categories, each of which
/// can be considered a different "namespace" of the program.
///
/// 1) Global methods - regular methods inside of the `#[program]`.
/// 2) State methods - associated methods inside a `#[state]` struct.
/// 3) Interface methods - methods inside a strait struct's
///    implementation of an `#[interface]` trait.
///
/// Care must be taken by the codegen to prevent collisions between
/// methods in these different namespaces. For this reason, Anchor uses
/// a variant of sighash to perform method dispatch, rather than
/// something like a simple enum variant discriminator.
///
/// The execution flow of the generated code can be roughly outlined:
///
/// * Start program via the entrypoint.
/// * Strip method identifier off the first 8 bytes of the instruction
///   data and invoke the identified method. The method identifier
///   is a variant of sighash. See docs.rs for `anchor_lang` for details.
/// * If the method identifier is an IDL identifier, execute the IDL
///   instructions, which are a special set of hardcoded instructions
///   baked into every Anchor program. Then exit.
/// * Otherwise, the method identifier is for a user defined
///   instruction, i.e., one of the methods in the user defined
///   `#[program]` module. Perform method dispatch, i.e., execute the
///   big match statement mapping method identifier to method handler
///   wrapper.
/// * Run the method handler wrapper. This wraps the code the user
///   actually wrote, deserializing the accounts, constructing the
///   context, invoking the user's code, and finally running the exit
///   routine, which typically persists account changes.
///
/// The `entry` function here, defines the standard entry to a Solana
/// program, where execution begins.
#[cfg(not(feature = "no-entrypoint"))]
pub fn entry(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 8 {
        return Err(anchor_lang::__private::ErrorCode::InstructionMissing.into());
    }
    dispatch(program_id, accounts, data).map_err(|e| {
        ::solana_program::log::sol_log(&e.to_string());
        e
    })
}
/// Performs method dispatch.
///
/// Each method in an anchor program is uniquely defined by a namespace
/// and a rust identifier (i.e., the name given to the method). These
/// two pieces can be combined to creater a method identifier,
/// specifically, Anchor uses
///
/// Sha256("<namespace>::<rust-identifier>")[..8],
///
/// where the namespace can be one of three types. 1) "global" for a
/// regular instruction, 2) "state" for a state struct instruction
/// handler and 3) a trait namespace (used in combination with the
/// `#[interface]` attribute), which is defined by the trait name, e..
/// `MyTrait`.
///
/// With this 8 byte identifier, Anchor performs method dispatch,
/// matching the given 8 byte identifier to the associated method
/// handler, which leads to user defined code being eventually invoked.
fn dispatch(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let mut ix_data: &[u8] = data;
    let sighash: [u8; 8] = {
        let mut sighash: [u8; 8] = [0; 8];
        sighash.copy_from_slice(&ix_data[..8]);
        ix_data = &ix_data[8..];
        sighash
    };
    if true {
        if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
            return __private::__idl::__idl_dispatch(program_id, accounts, &ix_data);
        }
    }
    match sighash {
        [175, 175, 109, 31, 13, 152, 155, 237] => {
            __private::__global::initialize(program_id, accounts, ix_data)
        }
        [50, 106, 66, 104, 99, 118, 145, 88] => {
            __private::__global::change_authority(program_id, accounts, ix_data)
        }
        [250, 113, 53, 54, 141, 117, 215, 185] => {
            __private::__global::add_validator(program_id, accounts, ix_data)
        }
        [25, 96, 211, 155, 161, 14, 168, 188] => {
            __private::__global::remove_validator(program_id, accounts, ix_data)
        }
        [101, 41, 206, 33, 216, 111, 25, 78] => {
            __private::__global::set_validator_score(program_id, accounts, ix_data)
        }
        [27, 90, 97, 209, 17, 115, 7, 40] => {
            __private::__global::config_validator_system(program_id, accounts, ix_data)
        }
        [242, 35, 198, 137, 82, 225, 242, 182] => {
            __private::__global::deposit(program_id, accounts, ix_data)
        }
        [110, 130, 115, 41, 164, 102, 2, 59] => {
            __private::__global::deposit_stake_account(program_id, accounts, ix_data)
        }
        [30, 30, 119, 240, 191, 227, 12, 16] => {
            __private::__global::liquid_unstake(program_id, accounts, ix_data)
        }
        [181, 157, 89, 67, 143, 182, 52, 72] => {
            __private::__global::add_liquidity(program_id, accounts, ix_data)
        }
        [80, 85, 209, 72, 24, 206, 177, 108] => {
            __private::__global::remove_liquidity(program_id, accounts, ix_data)
        }
        [227, 163, 242, 45, 79, 203, 106, 44] => {
            __private::__global::set_lp_params(program_id, accounts, ix_data)
        }
        [67, 3, 34, 114, 190, 185, 17, 62] => {
            __private::__global::config_marinade(program_id, accounts, ix_data)
        }
        [97, 167, 144, 107, 117, 190, 128, 36] => {
            __private::__global::order_unstake(program_id, accounts, ix_data)
        }
        [62, 198, 214, 193, 213, 159, 108, 210] => {
            __private::__global::claim(program_id, accounts, ix_data)
        }
        [87, 217, 23, 179, 205, 25, 113, 129] => {
            __private::__global::stake_reserve(program_id, accounts, ix_data)
        }
        [4, 67, 81, 64, 136, 245, 93, 152] => {
            __private::__global::update_active(program_id, accounts, ix_data)
        }
        [16, 232, 131, 115, 156, 100, 239, 50] => {
            __private::__global::update_deactivated(program_id, accounts, ix_data)
        }
        [165, 158, 229, 97, 168, 220, 187, 225] => {
            __private::__global::deactivate_stake(program_id, accounts, ix_data)
        }
        [123, 69, 168, 195, 183, 213, 199, 214] => {
            __private::__global::emergency_unstake(program_id, accounts, ix_data)
        }
        [55, 241, 205, 221, 45, 114, 205, 163] => {
            __private::__global::partial_unstake(program_id, accounts, ix_data)
        }
        [216, 36, 141, 225, 243, 78, 125, 237] => {
            __private::__global::merge_stakes(program_id, accounts, ix_data)
        }
        _ => Err(anchor_lang::__private::ErrorCode::InstructionFallbackNotFound.into()),
    }
}
/// Create a private module to not clutter the program's namespace.
/// Defines an entrypoint for each individual instruction handler
/// wrapper.
mod __private {
    use super::*;
    /// __idl mod defines handlers for injected Anchor IDL instructions.
    pub mod __idl {
        use super::*;
        #[inline(never)]
        #[cfg(not(feature = "no-idl"))]
        pub fn __idl_dispatch(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            idl_ix_data: &[u8],
        ) -> ProgramResult {
            let mut accounts = accounts;
            let mut data: &[u8] = idl_ix_data;
            let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            match ix {
                anchor_lang::idl::IdlInstruction::Create { data_len } => {
                    let mut accounts = anchor_lang::idl::IdlCreateAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_create_account(program_id, &mut accounts, data_len)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::CreateBuffer => {
                    let mut accounts = anchor_lang::idl::IdlCreateBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_create_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::Write { data } => {
                    let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_write(program_id, &mut accounts, data)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                    let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_set_authority(program_id, &mut accounts, new_authority)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetBuffer => {
                    let mut accounts = anchor_lang::idl::IdlSetBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                    )?;
                    __idl_set_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
            }
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_account(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlCreateAccounts,
            data_len: u64,
        ) -> ProgramResult {
            if program_id != accounts.program.key {
                return Err(anchor_lang::__private::ErrorCode::IdlInstructionInvalidProgram.into());
            }
            let from = accounts.from.key;
            let (base, nonce) = Pubkey::find_program_address(&[], program_id);
            let seed = anchor_lang::idl::IdlAccount::seed();
            let owner = accounts.program.key;
            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
            let space = 8 + 32 + 4 + data_len as usize;
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);
            let seeds = &[&[nonce][..]];
            let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                from,
                &to,
                &base,
                seed,
                lamports,
                space as u64,
                owner,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    accounts.from.clone(),
                    accounts.to.clone(),
                    accounts.base.clone(),
                    accounts.system_program.clone(),
                ],
                &[seeds],
            )?;
            let mut idl_account = {
                let mut account_data = accounts.to.try_borrow_data()?;
                let mut account_data_slice: &[u8] = &account_data;
                anchor_lang::idl::IdlAccount::try_deserialize_unchecked(&mut account_data_slice)?
            };
            idl_account.authority = *accounts.from.key;
            let mut data = accounts.to.try_borrow_mut_data()?;
            let dst: &mut [u8] = &mut data;
            let mut cursor = std::io::Cursor::new(dst);
            idl_account.try_serialize(&mut cursor)?;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_buffer(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlCreateBuffer,
        ) -> ProgramResult {
            let mut buffer = &mut accounts.buffer;
            buffer.authority = *accounts.authority.key;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_write(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlAccounts,
            idl_data: Vec<u8>,
        ) -> ProgramResult {
            let mut idl = &mut accounts.idl;
            idl.data.extend(idl_data);
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_authority(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlAccounts,
            new_authority: Pubkey,
        ) -> ProgramResult {
            accounts.idl.authority = new_authority;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_buffer(
            program_id: &Pubkey,
            accounts: &mut anchor_lang::idl::IdlSetBuffer,
        ) -> ProgramResult {
            accounts.idl.data = accounts.buffer.data.clone();
            Ok(())
        }
    }
    /// __state mod defines wrapped handlers for state instructions.
    pub mod __state {
        use super::*;
    }
    /// __interface mod defines wrapped handlers for `#[interface]` trait
    /// implementations.
    pub mod __interface {
        use super::*;
    }
    /// __global mod defines wrapped handlers for global instructions.
    pub mod __global {
        use super::*;
        #[inline(never)]
        pub fn initialize(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Initialize::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Initialize { data } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                Initialize::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::initialize(
                Context::new(program_id, &mut accounts, remaining_accounts),
                data,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn change_authority(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ChangeAuthority::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ChangeAuthority { data } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                ChangeAuthority::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::change_authority(
                Context::new(program_id, &mut accounts, remaining_accounts),
                data,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn add_validator(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::AddValidator::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::AddValidator { score } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                AddValidator::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::add_validator(
                Context::new(program_id, &mut accounts, remaining_accounts),
                score,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn remove_validator(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::RemoveValidator::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::RemoveValidator {
                index,
                validator_vote,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                RemoveValidator::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::remove_validator(
                Context::new(program_id, &mut accounts, remaining_accounts),
                index,
                validator_vote,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn set_validator_score(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::SetValidatorScore::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::SetValidatorScore {
                index,
                validator_vote,
                score,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                SetValidatorScore::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::set_validator_score(
                Context::new(program_id, &mut accounts, remaining_accounts),
                index,
                validator_vote,
                score,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn config_validator_system(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ConfigValidatorSystem::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ConfigValidatorSystem { extra_runs } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                ConfigValidatorSystem::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::config_validator_system(
                Context::new(program_id, &mut accounts, remaining_accounts),
                extra_runs,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn deposit(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Deposit::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Deposit { lamports } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts = Deposit::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::deposit(
                Context::new(program_id, &mut accounts, remaining_accounts),
                lamports,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn deposit_stake_account(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::DepositStakeAccount::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::DepositStakeAccount { validator_index } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                DepositStakeAccount::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::deposit_stake_account(
                Context::new(program_id, &mut accounts, remaining_accounts),
                validator_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn liquid_unstake(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::LiquidUnstake::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::LiquidUnstake { msol_amount } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                LiquidUnstake::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::liquid_unstake(
                Context::new(program_id, &mut accounts, remaining_accounts),
                msol_amount,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn add_liquidity(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::AddLiquidity::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::AddLiquidity { lamports } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                AddLiquidity::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::add_liquidity(
                Context::new(program_id, &mut accounts, remaining_accounts),
                lamports,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn remove_liquidity(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::RemoveLiquidity::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::RemoveLiquidity { tokens } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                RemoveLiquidity::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::remove_liquidity(
                Context::new(program_id, &mut accounts, remaining_accounts),
                tokens,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn set_lp_params(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::SetLpParams::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::SetLpParams {
                min_fee,
                max_fee,
                liquidity_target,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                SetLpParams::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::set_lp_params(
                Context::new(program_id, &mut accounts, remaining_accounts),
                min_fee,
                max_fee,
                liquidity_target,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn config_marinade(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::ConfigMarinade::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::ConfigMarinade { params } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                ConfigMarinade::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::config_marinade(
                Context::new(program_id, &mut accounts, remaining_accounts),
                params,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn order_unstake(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::OrderUnstake::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::OrderUnstake { msol_amount } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                OrderUnstake::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::order_unstake(
                Context::new(program_id, &mut accounts, remaining_accounts),
                msol_amount,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn claim(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::Claim::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::Claim = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts = Claim::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::claim(Context::new(program_id, &mut accounts, remaining_accounts))?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn stake_reserve(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::StakeReserve::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::StakeReserve { validator_index } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                StakeReserve::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::stake_reserve(
                Context::new(program_id, &mut accounts, remaining_accounts),
                validator_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn update_active(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::UpdateActive::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::UpdateActive {
                stake_index,
                validator_index,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                UpdateActive::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::update_active(
                Context::new(program_id, &mut accounts, remaining_accounts),
                stake_index,
                validator_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn update_deactivated(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::UpdateDeactivated::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::UpdateDeactivated { stake_index } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                UpdateDeactivated::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::update_deactivated(
                Context::new(program_id, &mut accounts, remaining_accounts),
                stake_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn deactivate_stake(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::DeactivateStake::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::DeactivateStake {
                stake_index,
                validator_index,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                DeactivateStake::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::deactivate_stake(
                Context::new(program_id, &mut accounts, remaining_accounts),
                stake_index,
                validator_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn emergency_unstake(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::EmergencyUnstake::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::EmergencyUnstake {
                stake_index,
                validator_index,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                EmergencyUnstake::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::emergency_unstake(
                Context::new(program_id, &mut accounts, remaining_accounts),
                stake_index,
                validator_index,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn partial_unstake(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::PartialUnstake::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::PartialUnstake {
                stake_index,
                validator_index,
                desired_unstake_amount,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                PartialUnstake::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::partial_unstake(
                Context::new(program_id, &mut accounts, remaining_accounts),
                stake_index,
                validator_index,
                desired_unstake_amount,
            )?;
            accounts.exit(program_id)
        }
        #[inline(never)]
        pub fn merge_stakes(
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            ix_data: &[u8],
        ) -> ProgramResult {
            let ix = instruction::MergeStakes::deserialize(&mut &ix_data[..])
                .map_err(|_| anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::MergeStakes {
                destination_stake_index,
                source_stake_index,
                validator_index,
            } = ix;
            let mut remaining_accounts: &[AccountInfo] = accounts;
            let mut accounts =
                MergeStakes::try_accounts(program_id, &mut remaining_accounts, ix_data)?;
            marinade_finance::merge_stakes(
                Context::new(program_id, &mut accounts, remaining_accounts),
                destination_stake_index,
                source_stake_index,
                validator_index,
            )?;
            accounts.exit(program_id)
        }
    }
}
pub mod marinade_finance {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, data: InitializeData) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(data)?;
        Ok(())
    }
    pub fn change_authority(
        ctx: Context<ChangeAuthority>,
        data: ChangeAuthorityData,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(data)
    }
    pub fn add_validator(ctx: Context<AddValidator>, score: u32) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(score)
    }
    pub fn remove_validator(
        ctx: Context<RemoveValidator>,
        index: u32,
        validator_vote: Pubkey,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(index, validator_vote)
    }
    pub fn set_validator_score(
        ctx: Context<SetValidatorScore>,
        index: u32,
        validator_vote: Pubkey,
        score: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(index, validator_vote, score)
    }
    pub fn config_validator_system(
        ctx: Context<ConfigValidatorSystem>,
        extra_runs: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(extra_runs)
    }
    pub fn deposit(ctx: Context<Deposit>, lamports: u64) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(lamports)
    }
    pub fn deposit_stake_account(
        ctx: Context<DepositStakeAccount>,
        validator_index: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(validator_index)
    }
    pub fn liquid_unstake(ctx: Context<LiquidUnstake>, msol_amount: u64) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(msol_amount)
    }
    pub fn add_liquidity(ctx: Context<AddLiquidity>, lamports: u64) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(lamports)
    }
    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, tokens: u64) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(tokens)
    }
    pub fn set_lp_params(
        ctx: Context<SetLpParams>,
        min_fee: Fee,
        max_fee: Fee,
        liquidity_target: u64,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(min_fee, max_fee, liquidity_target)
    }
    pub fn config_marinade(
        ctx: Context<ConfigMarinade>,
        params: ConfigMarinadeParams,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(params)
    }
    pub fn order_unstake(ctx: Context<OrderUnstake>, msol_amount: u64) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(msol_amount)
    }
    pub fn claim(ctx: Context<Claim>) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process()
    }
    pub fn stake_reserve(ctx: Context<StakeReserve>, validator_index: u32) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(validator_index)
    }
    pub fn update_active(
        ctx: Context<UpdateActive>,
        stake_index: u32,
        validator_index: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(stake_index, validator_index)
    }
    pub fn update_deactivated(ctx: Context<UpdateDeactivated>, stake_index: u32) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(stake_index)
    }
    pub fn deactivate_stake(
        ctx: Context<DeactivateStake>,
        stake_index: u32,
        validator_index: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(stake_index, validator_index)
    }
    pub fn emergency_unstake(
        ctx: Context<EmergencyUnstake>,
        stake_index: u32,
        validator_index: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts.process(stake_index, validator_index)
    }
    pub fn partial_unstake(
        ctx: Context<PartialUnstake>,
        stake_index: u32,
        validator_index: u32,
        desired_unstake_amount: u64,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts
            .process(stake_index, validator_index, desired_unstake_amount)
    }
    pub fn merge_stakes(
        ctx: Context<MergeStakes>,
        destination_stake_index: u32,
        source_stake_index: u32,
        validator_index: u32,
    ) -> ProgramResult {
        check_context(&ctx)?;
        ctx.accounts
            .process(destination_stake_index, source_stake_index, validator_index)
    }
}
/// An Anchor generated module containing the program's set of
/// instructions, where each method handler in the `#[program]` mod is
/// associated with a struct defining the input arguments to the
/// method. These should be used directly, when one wants to serialize
/// Anchor instruction data, for example, when speciying
/// instructions on a client.
pub mod instruction {
    use super::*;
    /// Instruction struct definitions for `#[state]` methods.
    pub mod state {
        use super::*;
    }
    /// Instruction.
    pub struct Initialize {
        pub data: InitializeData,
    }
    impl borsh::ser::BorshSerialize for Initialize
    where
        InitializeData: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.data, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Initialize
    where
        InitializeData: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                data: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Initialize {
        fn data(&self) -> Vec<u8> {
            let mut d = [175, 175, 109, 31, 13, 152, 155, 237].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ChangeAuthority {
        pub data: ChangeAuthorityData,
    }
    impl borsh::ser::BorshSerialize for ChangeAuthority
    where
        ChangeAuthorityData: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.data, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ChangeAuthority
    where
        ChangeAuthorityData: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                data: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ChangeAuthority {
        fn data(&self) -> Vec<u8> {
            let mut d = [50, 106, 66, 104, 99, 118, 145, 88].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct AddValidator {
        pub score: u32,
    }
    impl borsh::ser::BorshSerialize for AddValidator
    where
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for AddValidator
    where
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                score: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for AddValidator {
        fn data(&self) -> Vec<u8> {
            let mut d = [250, 113, 53, 54, 141, 117, 215, 185].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct RemoveValidator {
        pub index: u32,
        pub validator_vote: Pubkey,
    }
    impl borsh::ser::BorshSerialize for RemoveValidator
    where
        u32: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_vote, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for RemoveValidator
    where
        u32: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_vote: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for RemoveValidator {
        fn data(&self) -> Vec<u8> {
            let mut d = [25, 96, 211, 155, 161, 14, 168, 188].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct SetValidatorScore {
        pub index: u32,
        pub validator_vote: Pubkey,
        pub score: u32,
    }
    impl borsh::ser::BorshSerialize for SetValidatorScore
    where
        u32: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_vote, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for SetValidatorScore
    where
        u32: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_vote: borsh::BorshDeserialize::deserialize(buf)?,
                score: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for SetValidatorScore {
        fn data(&self) -> Vec<u8> {
            let mut d = [101, 41, 206, 33, 216, 111, 25, 78].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ConfigValidatorSystem {
        pub extra_runs: u32,
    }
    impl borsh::ser::BorshSerialize for ConfigValidatorSystem
    where
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.extra_runs, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ConfigValidatorSystem
    where
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                extra_runs: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ConfigValidatorSystem {
        fn data(&self) -> Vec<u8> {
            let mut d = [27, 90, 97, 209, 17, 115, 7, 40].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct Deposit {
        pub lamports: u64,
    }
    impl borsh::ser::BorshSerialize for Deposit
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.lamports, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Deposit
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                lamports: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for Deposit {
        fn data(&self) -> Vec<u8> {
            let mut d = [242, 35, 198, 137, 82, 225, 242, 182].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct DepositStakeAccount {
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for DepositStakeAccount
    where
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for DepositStakeAccount
    where
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for DepositStakeAccount {
        fn data(&self) -> Vec<u8> {
            let mut d = [110, 130, 115, 41, 164, 102, 2, 59].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct LiquidUnstake {
        pub msol_amount: u64,
    }
    impl borsh::ser::BorshSerialize for LiquidUnstake
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.msol_amount, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for LiquidUnstake
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                msol_amount: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for LiquidUnstake {
        fn data(&self) -> Vec<u8> {
            let mut d = [30, 30, 119, 240, 191, 227, 12, 16].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct AddLiquidity {
        pub lamports: u64,
    }
    impl borsh::ser::BorshSerialize for AddLiquidity
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.lamports, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for AddLiquidity
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                lamports: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for AddLiquidity {
        fn data(&self) -> Vec<u8> {
            let mut d = [181, 157, 89, 67, 143, 182, 52, 72].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct RemoveLiquidity {
        pub tokens: u64,
    }
    impl borsh::ser::BorshSerialize for RemoveLiquidity
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.tokens, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for RemoveLiquidity
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                tokens: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for RemoveLiquidity {
        fn data(&self) -> Vec<u8> {
            let mut d = [80, 85, 209, 72, 24, 206, 177, 108].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct SetLpParams {
        pub min_fee: Fee,
        pub max_fee: Fee,
        pub liquidity_target: u64,
    }
    impl borsh::ser::BorshSerialize for SetLpParams
    where
        Fee: borsh::ser::BorshSerialize,
        Fee: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.min_fee, writer)?;
            borsh::BorshSerialize::serialize(&self.max_fee, writer)?;
            borsh::BorshSerialize::serialize(&self.liquidity_target, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for SetLpParams
    where
        Fee: borsh::BorshDeserialize,
        Fee: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                min_fee: borsh::BorshDeserialize::deserialize(buf)?,
                max_fee: borsh::BorshDeserialize::deserialize(buf)?,
                liquidity_target: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for SetLpParams {
        fn data(&self) -> Vec<u8> {
            let mut d = [227, 163, 242, 45, 79, 203, 106, 44].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct ConfigMarinade {
        pub params: ConfigMarinadeParams,
    }
    impl borsh::ser::BorshSerialize for ConfigMarinade
    where
        ConfigMarinadeParams: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.params, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for ConfigMarinade
    where
        ConfigMarinadeParams: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                params: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for ConfigMarinade {
        fn data(&self) -> Vec<u8> {
            let mut d = [67, 3, 34, 114, 190, 185, 17, 62].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct OrderUnstake {
        pub msol_amount: u64,
    }
    impl borsh::ser::BorshSerialize for OrderUnstake
    where
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.msol_amount, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for OrderUnstake
    where
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                msol_amount: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for OrderUnstake {
        fn data(&self) -> Vec<u8> {
            let mut d = [97, 167, 144, 107, 117, 190, 128, 36].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct Claim;
    impl borsh::ser::BorshSerialize for Claim {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for Claim {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {})
        }
    }
    impl anchor_lang::InstructionData for Claim {
        fn data(&self) -> Vec<u8> {
            let mut d = [62, 198, 214, 193, 213, 159, 108, 210].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct StakeReserve {
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for StakeReserve
    where
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for StakeReserve
    where
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for StakeReserve {
        fn data(&self) -> Vec<u8> {
            let mut d = [87, 217, 23, 179, 205, 25, 113, 129].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct UpdateActive {
        pub stake_index: u32,
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for UpdateActive
    where
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for UpdateActive
    where
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for UpdateActive {
        fn data(&self) -> Vec<u8> {
            let mut d = [4, 67, 81, 64, 136, 245, 93, 152].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct UpdateDeactivated {
        pub stake_index: u32,
    }
    impl borsh::ser::BorshSerialize for UpdateDeactivated
    where
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for UpdateDeactivated
    where
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for UpdateDeactivated {
        fn data(&self) -> Vec<u8> {
            let mut d = [16, 232, 131, 115, 156, 100, 239, 50].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct DeactivateStake {
        pub stake_index: u32,
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for DeactivateStake
    where
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for DeactivateStake
    where
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for DeactivateStake {
        fn data(&self) -> Vec<u8> {
            let mut d = [165, 158, 229, 97, 168, 220, 187, 225].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct EmergencyUnstake {
        pub stake_index: u32,
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for EmergencyUnstake
    where
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for EmergencyUnstake
    where
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for EmergencyUnstake {
        fn data(&self) -> Vec<u8> {
            let mut d = [123, 69, 168, 195, 183, 213, 199, 214].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct PartialUnstake {
        pub stake_index: u32,
        pub validator_index: u32,
        pub desired_unstake_amount: u64,
    }
    impl borsh::ser::BorshSerialize for PartialUnstake
    where
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            borsh::BorshSerialize::serialize(&self.desired_unstake_amount, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for PartialUnstake
    where
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
                desired_unstake_amount: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for PartialUnstake {
        fn data(&self) -> Vec<u8> {
            let mut d = [55, 241, 205, 221, 45, 114, 205, 163].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
    /// Instruction.
    pub struct MergeStakes {
        pub destination_stake_index: u32,
        pub source_stake_index: u32,
        pub validator_index: u32,
    }
    impl borsh::ser::BorshSerialize for MergeStakes
    where
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
        u32: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.destination_stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.source_stake_index, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_index, writer)?;
            Ok(())
        }
    }
    impl borsh::de::BorshDeserialize for MergeStakes
    where
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
        u32: borsh::BorshDeserialize,
    {
        fn deserialize(
            buf: &mut &[u8],
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                destination_stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                source_stake_index: borsh::BorshDeserialize::deserialize(buf)?,
                validator_index: borsh::BorshDeserialize::deserialize(buf)?,
            })
        }
    }
    impl anchor_lang::InstructionData for MergeStakes {
        fn data(&self) -> Vec<u8> {
            let mut d = [216, 36, 141, 225, 243, 78, 125, 237].to_vec();
            d.append(&mut self.try_to_vec().expect("Should always serialize"));
            d
        }
    }
}
/// An Anchor generated module, providing a set of structs
/// mirroring the structs deriving `Accounts`, where each field is
/// a `Pubkey`. This is useful for specifying accounts for a client.
pub mod accounts {
    pub use crate::__client_accounts_config_marinade::*;
    pub use crate::__client_accounts_stake_reserve::*;
    pub use crate::__client_accounts_update_active::*;
    pub use crate::__client_accounts_deactivate_stake::*;
    pub use crate::__client_accounts_change_authority::*;
    pub use crate::__client_accounts_set_lp_params::*;
    pub use crate::__client_accounts_deposit_stake_account::*;
    pub use crate::__client_accounts_emergency_unstake::*;
    pub use crate::__client_accounts_deposit::*;
    pub use crate::__client_accounts_liquid_unstake::*;
    pub use crate::__client_accounts_merge_stakes::*;
    pub use crate::__client_accounts_initialize::*;
    pub use crate::__client_accounts_claim::*;
    pub use crate::__client_accounts_set_validator_score::*;
    pub use crate::__client_accounts_add_validator::*;
    pub use crate::__client_accounts_order_unstake::*;
    pub use crate::__client_accounts_remove_validator::*;
    pub use crate::__client_accounts_config_validator_system::*;
    pub use crate::__client_accounts_add_liquidity::*;
    pub use crate::__client_accounts_remove_liquidity::*;
    pub use crate::__client_accounts_update_deactivated::*;
    pub use crate::__client_accounts_partial_unstake::*;
}
#[cfg(not(feature = "no-entrypoint"))]
pub fn test_entry(program_id: &Pubkey, accounts: &[AccountInfo], ix_data: &[u8]) -> ProgramResult {
    entry(program_id, accounts, ix_data)
}
pub struct Fee {
    pub basis_points: u32,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Fee {
    #[inline]
    fn clone(&self) -> Fee {
        {
            let _: ::core::clone::AssertParamIsClone<u32>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for Fee {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Fee {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Self {
                basis_points: ref __self_0_0,
            } => {
                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "Fee");
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "basis_points",
                    &&(*__self_0_0),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for Fee {
    #[inline]
    fn default() -> Fee {
        Fee {
            basis_points: ::core::default::Default::default(),
        }
    }
}
impl borsh::ser::BorshSerialize for Fee
where
    u32: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.basis_points, writer)?;
        Ok(())
    }
}
impl borsh::de::BorshDeserialize for Fee
where
    u32: borsh::BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            basis_points: borsh::BorshDeserialize::deserialize(buf)?,
        })
    }
}
impl ::core::marker::StructuralPartialEq for Fee {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Fee {
    #[inline]
    fn eq(&self, other: &Fee) -> bool {
        match *other {
            Self {
                basis_points: ref __self_1_0,
            } => match *self {
                Self {
                    basis_points: ref __self_0_0,
                } => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Fee) -> bool {
        match *other {
            Self {
                basis_points: ref __self_1_0,
            } => match *self {
                Self {
                    basis_points: ref __self_0_0,
                } => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl ::core::marker::StructuralEq for Fee {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Fee {
    #[inline]
    #[doc(hidden)]
    #[no_coverage]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<u32>;
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialOrd for Fee {
    #[inline]
    fn partial_cmp(&self, other: &Fee) -> ::core::option::Option<::core::cmp::Ordering> {
        match *other {
            Self {
                basis_points: ref __self_1_0,
            } => match *self {
                Self {
                    basis_points: ref __self_0_0,
                } => match ::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0), &(*__self_1_0)) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                    }
                    cmp => cmp,
                },
            },
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Ord for Fee {
    #[inline]
    fn cmp(&self, other: &Fee) -> ::core::cmp::Ordering {
        match *other {
            Self {
                basis_points: ref __self_1_0,
            } => match *self {
                Self {
                    basis_points: ref __self_0_0,
                } => match ::core::cmp::Ord::cmp(&(*__self_0_0), &(*__self_1_0)) {
                    ::core::cmp::Ordering::Equal => ::core::cmp::Ordering::Equal,
                    cmp => cmp,
                },
            },
        }
    }
}
impl Display for Fee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        {
            let result = f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", "%"],
                &[::core::fmt::ArgumentV1::new_display(
                    &(self.basis_points as f32 / 100.0),
                )],
            ));
            result
        }
    }
}
impl Fee {
    pub fn from_basis_points(basis_points: u32) -> Self {
        Self { basis_points }
    }
    /// generic check, capped Fee
    pub fn check_max(&self, max_basis_points: u32) -> Result<(), CommonError> {
        if self.basis_points > max_basis_points {
            Err(CommonError::FeeTooHigh)
        } else {
            Ok(())
        }
    }
    /// base check, Fee <= 100%
    pub fn check(&self) -> Result<(), CommonError> {
        self.check_max(10_000)
    }
    pub fn apply(&self, lamports: u64) -> u64 {
        (lamports as u128 * self.basis_points as u128 / 10_000_u128) as u64
    }
}
impl TryFrom<f64> for Fee {
    type Error = CommonError;
    fn try_from(n: f64) -> Result<Self, CommonError> {
        let basis_points_i = (n * 100.0).floor() as i64;
        let basis_points =
            u32::try_from(basis_points_i).map_err(|_| CommonError::CalculationFailure)?;
        let fee = Fee::from_basis_points(basis_points);
        fee.check()?;
        Ok(fee)
    }
}
impl FromStr for Fee {
    type Err = CommonError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        f64::try_into(s.parse().map_err(|_| CommonError::CalculationFailure)?)
    }
}
pub struct Initialize<'info> {
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub creator_authority: AccountInfo<'info>,
    # [account (zero , rent_exempt = enforce)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    # [account (mut , rent_exempt = enforce)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    # [account (mut , rent_exempt = enforce)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    pub msol_mint: CpiAccount<'info, Mint>,
    ///CHECK: stf anchor
    pub operational_sol_account: AccountInfo<'info>,
    pub liq_pool: LiqPoolInitialize<'info>,
    treasury_msol_account: CpiAccount<'info, TokenAccount>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for Initialize<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let creator_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let state = &accounts[0];
        *accounts = &accounts[1..];
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let operational_sol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool: LiqPoolInitialize<'info> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let treasury_msol_account: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if true {
            if !creator_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        let __anchor_rent = Rent::get()?;
        let state: anchor_lang::ProgramAccount<State> = {
            let mut __data: &[u8] = &state.try_borrow_data()?;
            let mut __disc_bytes = [0u8; 8];
            __disc_bytes.copy_from_slice(&__data[..8]);
            let __discriminator = u64::from_le_bytes(__disc_bytes);
            if __discriminator != 0 {
                return Err(anchor_lang::__private::ErrorCode::ConstraintZero.into());
            }
            anchor_lang::ProgramAccount::try_from_unchecked(program_id, &state)?
        };
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !__anchor_rent.is_exempt(
            state.to_account_info().lamports(),
            state.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
        }
        let __anchor_rent = Rent::get()?;
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !__anchor_rent.is_exempt(
            stake_list.to_account_info().lamports(),
            stake_list.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
        }
        let __anchor_rent = Rent::get()?;
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !__anchor_rent.is_exempt(
            validator_list.to_account_info().lamports(),
            validator_list.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
        }
        Ok(Initialize {
            creator_authority,
            state,
            reserve_pda,
            stake_list,
            validator_list,
            msol_mint,
            operational_sol_account,
            liq_pool,
            treasury_msol_account,
            clock,
            rent,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for Initialize<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.creator_authority.to_account_infos());
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.operational_sol_account.to_account_infos());
        account_infos.extend(self.liq_pool.to_account_infos());
        account_infos.extend(self.treasury_msol_account.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for Initialize<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.creator_authority.to_account_metas(Some(true)));
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.operational_sol_account.to_account_metas(None));
        account_metas.extend(self.liq_pool.to_account_metas(None));
        account_metas.extend(self.treasury_msol_account.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for Initialize<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_initialize {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub use __client_accounts_liq_pool_initialize::LiqPoolInitialize;
    pub struct Initialize {
        pub creator_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub operational_sol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool: __client_accounts_liq_pool_initialize::LiqPoolInitialize,
        pub treasury_msol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for Initialize
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        __client_accounts_liq_pool_initialize::LiqPoolInitialize: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.creator_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool, writer)?;
            borsh::BorshSerialize::serialize(&self.treasury_msol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for Initialize {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.creator_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.reserve_pda,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.msol_mint,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.operational_sol_account,
                    false,
                ),
            );
            account_metas.extend(self.liq_pool.to_account_metas(None));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.treasury_msol_account,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas
        }
    }
}
pub struct InitializeData {
    pub admin_authority: Pubkey,
    pub validator_manager_authority: Pubkey,
    pub min_stake: u64,
    pub reward_fee: Fee,
    pub liq_pool: LiqPoolInitializeData,
    pub additional_stake_record_space: u32,
    pub additional_validator_record_space: u32,
    pub slots_for_stake_delta: u64,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for InitializeData {
    #[inline]
    fn clone(&self) -> InitializeData {
        {
            let _: ::core::clone::AssertParamIsClone<Pubkey>;
            let _: ::core::clone::AssertParamIsClone<Pubkey>;
            let _: ::core::clone::AssertParamIsClone<u64>;
            let _: ::core::clone::AssertParamIsClone<Fee>;
            let _: ::core::clone::AssertParamIsClone<LiqPoolInitializeData>;
            let _: ::core::clone::AssertParamIsClone<u32>;
            let _: ::core::clone::AssertParamIsClone<u32>;
            let _: ::core::clone::AssertParamIsClone<u64>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for InitializeData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for InitializeData {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Self {
                admin_authority: ref __self_0_0,
                validator_manager_authority: ref __self_0_1,
                min_stake: ref __self_0_2,
                reward_fee: ref __self_0_3,
                liq_pool: ref __self_0_4,
                additional_stake_record_space: ref __self_0_5,
                additional_validator_record_space: ref __self_0_6,
                slots_for_stake_delta: ref __self_0_7,
            } => {
                let debug_trait_builder =
                    &mut ::core::fmt::Formatter::debug_struct(f, "InitializeData");
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "admin_authority",
                    &&(*__self_0_0),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "validator_manager_authority",
                    &&(*__self_0_1),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "min_stake",
                    &&(*__self_0_2),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "reward_fee",
                    &&(*__self_0_3),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "liq_pool",
                    &&(*__self_0_4),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "additional_stake_record_space",
                    &&(*__self_0_5),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "additional_validator_record_space",
                    &&(*__self_0_6),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "slots_for_stake_delta",
                    &&(*__self_0_7),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for InitializeData {
    #[inline]
    fn default() -> InitializeData {
        InitializeData {
            admin_authority: ::core::default::Default::default(),
            validator_manager_authority: ::core::default::Default::default(),
            min_stake: ::core::default::Default::default(),
            reward_fee: ::core::default::Default::default(),
            liq_pool: ::core::default::Default::default(),
            additional_stake_record_space: ::core::default::Default::default(),
            additional_validator_record_space: ::core::default::Default::default(),
            slots_for_stake_delta: ::core::default::Default::default(),
        }
    }
}
impl ::core::marker::StructuralPartialEq for InitializeData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for InitializeData {
    #[inline]
    fn eq(&self, other: &InitializeData) -> bool {
        match *other {
            Self {
                admin_authority: ref __self_1_0,
                validator_manager_authority: ref __self_1_1,
                min_stake: ref __self_1_2,
                reward_fee: ref __self_1_3,
                liq_pool: ref __self_1_4,
                additional_stake_record_space: ref __self_1_5,
                additional_validator_record_space: ref __self_1_6,
                slots_for_stake_delta: ref __self_1_7,
            } => match *self {
                Self {
                    admin_authority: ref __self_0_0,
                    validator_manager_authority: ref __self_0_1,
                    min_stake: ref __self_0_2,
                    reward_fee: ref __self_0_3,
                    liq_pool: ref __self_0_4,
                    additional_stake_record_space: ref __self_0_5,
                    additional_validator_record_space: ref __self_0_6,
                    slots_for_stake_delta: ref __self_0_7,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                        && (*__self_0_4) == (*__self_1_4)
                        && (*__self_0_5) == (*__self_1_5)
                        && (*__self_0_6) == (*__self_1_6)
                        && (*__self_0_7) == (*__self_1_7)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &InitializeData) -> bool {
        match *other {
            Self {
                admin_authority: ref __self_1_0,
                validator_manager_authority: ref __self_1_1,
                min_stake: ref __self_1_2,
                reward_fee: ref __self_1_3,
                liq_pool: ref __self_1_4,
                additional_stake_record_space: ref __self_1_5,
                additional_validator_record_space: ref __self_1_6,
                slots_for_stake_delta: ref __self_1_7,
            } => match *self {
                Self {
                    admin_authority: ref __self_0_0,
                    validator_manager_authority: ref __self_0_1,
                    min_stake: ref __self_0_2,
                    reward_fee: ref __self_0_3,
                    liq_pool: ref __self_0_4,
                    additional_stake_record_space: ref __self_0_5,
                    additional_validator_record_space: ref __self_0_6,
                    slots_for_stake_delta: ref __self_0_7,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                        || (*__self_0_4) != (*__self_1_4)
                        || (*__self_0_5) != (*__self_1_5)
                        || (*__self_0_6) != (*__self_1_6)
                        || (*__self_0_7) != (*__self_1_7)
                }
            },
        }
    }
}
impl borsh::ser::BorshSerialize for InitializeData
where
    Pubkey: borsh::ser::BorshSerialize,
    Pubkey: borsh::ser::BorshSerialize,
    u64: borsh::ser::BorshSerialize,
    Fee: borsh::ser::BorshSerialize,
    LiqPoolInitializeData: borsh::ser::BorshSerialize,
    u32: borsh::ser::BorshSerialize,
    u32: borsh::ser::BorshSerialize,
    u64: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.admin_authority, writer)?;
        borsh::BorshSerialize::serialize(&self.validator_manager_authority, writer)?;
        borsh::BorshSerialize::serialize(&self.min_stake, writer)?;
        borsh::BorshSerialize::serialize(&self.reward_fee, writer)?;
        borsh::BorshSerialize::serialize(&self.liq_pool, writer)?;
        borsh::BorshSerialize::serialize(&self.additional_stake_record_space, writer)?;
        borsh::BorshSerialize::serialize(&self.additional_validator_record_space, writer)?;
        borsh::BorshSerialize::serialize(&self.slots_for_stake_delta, writer)?;
        Ok(())
    }
}
impl borsh::de::BorshDeserialize for InitializeData
where
    Pubkey: borsh::BorshDeserialize,
    Pubkey: borsh::BorshDeserialize,
    u64: borsh::BorshDeserialize,
    Fee: borsh::BorshDeserialize,
    LiqPoolInitializeData: borsh::BorshDeserialize,
    u32: borsh::BorshDeserialize,
    u32: borsh::BorshDeserialize,
    u64: borsh::BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            admin_authority: borsh::BorshDeserialize::deserialize(buf)?,
            validator_manager_authority: borsh::BorshDeserialize::deserialize(buf)?,
            min_stake: borsh::BorshDeserialize::deserialize(buf)?,
            reward_fee: borsh::BorshDeserialize::deserialize(buf)?,
            liq_pool: borsh::BorshDeserialize::deserialize(buf)?,
            additional_stake_record_space: borsh::BorshDeserialize::deserialize(buf)?,
            additional_validator_record_space: borsh::BorshDeserialize::deserialize(buf)?,
            slots_for_stake_delta: borsh::BorshDeserialize::deserialize(buf)?,
        })
    }
}
pub struct LiqPoolInitialize<'info> {
    pub lp_mint: CpiAccount<'info, Mint>,
    ///CHECK: stf anchor
    pub sol_leg_pda: AccountInfo<'info>,
    pub msol_leg: CpiAccount<'info, TokenAccount>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for LiqPoolInitialize<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let lp_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let sol_leg_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_leg: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        Ok(LiqPoolInitialize {
            lp_mint,
            sol_leg_pda,
            msol_leg,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for LiqPoolInitialize<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.lp_mint.to_account_infos());
        account_infos.extend(self.sol_leg_pda.to_account_infos());
        account_infos.extend(self.msol_leg.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for LiqPoolInitialize<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.lp_mint.to_account_metas(None));
        account_metas.extend(self.sol_leg_pda.to_account_metas(None));
        account_metas.extend(self.msol_leg.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for LiqPoolInitialize<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_liq_pool_initialize {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct LiqPoolInitialize {
        pub lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub sol_leg_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_leg: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for LiqPoolInitialize
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.lp_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.sol_leg_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_leg, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for LiqPoolInitialize {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.lp_mint,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.sol_leg_pda,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.msol_leg,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct LiqPoolInitializeData {
    pub lp_liquidity_target: u64,
    pub lp_max_fee: Fee,
    pub lp_min_fee: Fee,
    pub lp_treasury_cut: Fee,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for LiqPoolInitializeData {
    #[inline]
    fn clone(&self) -> LiqPoolInitializeData {
        {
            let _: ::core::clone::AssertParamIsClone<u64>;
            let _: ::core::clone::AssertParamIsClone<Fee>;
            let _: ::core::clone::AssertParamIsClone<Fee>;
            let _: ::core::clone::AssertParamIsClone<Fee>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for LiqPoolInitializeData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for LiqPoolInitializeData {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Self {
                lp_liquidity_target: ref __self_0_0,
                lp_max_fee: ref __self_0_1,
                lp_min_fee: ref __self_0_2,
                lp_treasury_cut: ref __self_0_3,
            } => {
                let debug_trait_builder =
                    &mut ::core::fmt::Formatter::debug_struct(f, "LiqPoolInitializeData");
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "lp_liquidity_target",
                    &&(*__self_0_0),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "lp_max_fee",
                    &&(*__self_0_1),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "lp_min_fee",
                    &&(*__self_0_2),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "lp_treasury_cut",
                    &&(*__self_0_3),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for LiqPoolInitializeData {
    #[inline]
    fn default() -> LiqPoolInitializeData {
        LiqPoolInitializeData {
            lp_liquidity_target: ::core::default::Default::default(),
            lp_max_fee: ::core::default::Default::default(),
            lp_min_fee: ::core::default::Default::default(),
            lp_treasury_cut: ::core::default::Default::default(),
        }
    }
}
impl ::core::marker::StructuralPartialEq for LiqPoolInitializeData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for LiqPoolInitializeData {
    #[inline]
    fn eq(&self, other: &LiqPoolInitializeData) -> bool {
        match *other {
            Self {
                lp_liquidity_target: ref __self_1_0,
                lp_max_fee: ref __self_1_1,
                lp_min_fee: ref __self_1_2,
                lp_treasury_cut: ref __self_1_3,
            } => match *self {
                Self {
                    lp_liquidity_target: ref __self_0_0,
                    lp_max_fee: ref __self_0_1,
                    lp_min_fee: ref __self_0_2,
                    lp_treasury_cut: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &LiqPoolInitializeData) -> bool {
        match *other {
            Self {
                lp_liquidity_target: ref __self_1_0,
                lp_max_fee: ref __self_1_1,
                lp_min_fee: ref __self_1_2,
                lp_treasury_cut: ref __self_1_3,
            } => match *self {
                Self {
                    lp_liquidity_target: ref __self_0_0,
                    lp_max_fee: ref __self_0_1,
                    lp_min_fee: ref __self_0_2,
                    lp_treasury_cut: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
impl borsh::ser::BorshSerialize for LiqPoolInitializeData
where
    u64: borsh::ser::BorshSerialize,
    Fee: borsh::ser::BorshSerialize,
    Fee: borsh::ser::BorshSerialize,
    Fee: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.lp_liquidity_target, writer)?;
        borsh::BorshSerialize::serialize(&self.lp_max_fee, writer)?;
        borsh::BorshSerialize::serialize(&self.lp_min_fee, writer)?;
        borsh::BorshSerialize::serialize(&self.lp_treasury_cut, writer)?;
        Ok(())
    }
}
impl borsh::de::BorshDeserialize for LiqPoolInitializeData
where
    u64: borsh::BorshDeserialize,
    Fee: borsh::BorshDeserialize,
    Fee: borsh::BorshDeserialize,
    Fee: borsh::BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            lp_liquidity_target: borsh::BorshDeserialize::deserialize(buf)?,
            lp_max_fee: borsh::BorshDeserialize::deserialize(buf)?,
            lp_min_fee: borsh::BorshDeserialize::deserialize(buf)?,
            lp_treasury_cut: borsh::BorshDeserialize::deserialize(buf)?,
        })
    }
}
pub struct ChangeAuthority<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub admin_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for ChangeAuthority<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let admin_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !admin_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(ChangeAuthority {
            state,
            admin_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for ChangeAuthority<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.admin_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for ChangeAuthority<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.admin_authority.to_account_metas(Some(true)));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for ChangeAuthority<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_change_authority {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct ChangeAuthority {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub admin_authority: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for ChangeAuthority
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.admin_authority, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for ChangeAuthority {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.admin_authority,
                    true,
                ),
            );
            account_metas
        }
    }
}
pub struct ChangeAuthorityData {
    pub admin: Option<Pubkey>,
    pub validator_manager: Option<Pubkey>,
    pub operational_sol_account: Option<Pubkey>,
    pub treasury_msol_account: Option<Pubkey>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for ChangeAuthorityData {
    #[inline]
    fn clone(&self) -> ChangeAuthorityData {
        {
            let _: ::core::clone::AssertParamIsClone<Option<Pubkey>>;
            let _: ::core::clone::AssertParamIsClone<Option<Pubkey>>;
            let _: ::core::clone::AssertParamIsClone<Option<Pubkey>>;
            let _: ::core::clone::AssertParamIsClone<Option<Pubkey>>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for ChangeAuthorityData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for ChangeAuthorityData {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Self {
                admin: ref __self_0_0,
                validator_manager: ref __self_0_1,
                operational_sol_account: ref __self_0_2,
                treasury_msol_account: ref __self_0_3,
            } => {
                let debug_trait_builder =
                    &mut ::core::fmt::Formatter::debug_struct(f, "ChangeAuthorityData");
                let _ =
                    ::core::fmt::DebugStruct::field(debug_trait_builder, "admin", &&(*__self_0_0));
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "validator_manager",
                    &&(*__self_0_1),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "operational_sol_account",
                    &&(*__self_0_2),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "treasury_msol_account",
                    &&(*__self_0_3),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for ChangeAuthorityData {
    #[inline]
    fn default() -> ChangeAuthorityData {
        ChangeAuthorityData {
            admin: ::core::default::Default::default(),
            validator_manager: ::core::default::Default::default(),
            operational_sol_account: ::core::default::Default::default(),
            treasury_msol_account: ::core::default::Default::default(),
        }
    }
}
impl ::core::marker::StructuralPartialEq for ChangeAuthorityData {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for ChangeAuthorityData {
    #[inline]
    fn eq(&self, other: &ChangeAuthorityData) -> bool {
        match *other {
            Self {
                admin: ref __self_1_0,
                validator_manager: ref __self_1_1,
                operational_sol_account: ref __self_1_2,
                treasury_msol_account: ref __self_1_3,
            } => match *self {
                Self {
                    admin: ref __self_0_0,
                    validator_manager: ref __self_0_1,
                    operational_sol_account: ref __self_0_2,
                    treasury_msol_account: ref __self_0_3,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &ChangeAuthorityData) -> bool {
        match *other {
            Self {
                admin: ref __self_1_0,
                validator_manager: ref __self_1_1,
                operational_sol_account: ref __self_1_2,
                treasury_msol_account: ref __self_1_3,
            } => match *self {
                Self {
                    admin: ref __self_0_0,
                    validator_manager: ref __self_0_1,
                    operational_sol_account: ref __self_0_2,
                    treasury_msol_account: ref __self_0_3,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                }
            },
        }
    }
}
impl borsh::ser::BorshSerialize for ChangeAuthorityData
where
    Option<Pubkey>: borsh::ser::BorshSerialize,
    Option<Pubkey>: borsh::ser::BorshSerialize,
    Option<Pubkey>: borsh::ser::BorshSerialize,
    Option<Pubkey>: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.admin, writer)?;
        borsh::BorshSerialize::serialize(&self.validator_manager, writer)?;
        borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
        borsh::BorshSerialize::serialize(&self.treasury_msol_account, writer)?;
        Ok(())
    }
}
impl borsh::de::BorshDeserialize for ChangeAuthorityData
where
    Option<Pubkey>: borsh::BorshDeserialize,
    Option<Pubkey>: borsh::BorshDeserialize,
    Option<Pubkey>: borsh::BorshDeserialize,
    Option<Pubkey>: borsh::BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            admin: borsh::BorshDeserialize::deserialize(buf)?,
            validator_manager: borsh::BorshDeserialize::deserialize(buf)?,
            operational_sol_account: borsh::BorshDeserialize::deserialize(buf)?,
            treasury_msol_account: borsh::BorshDeserialize::deserialize(buf)?,
        })
    }
}
pub struct AddLiquidity<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    pub lp_mint: CpiAccount<'info, Mint>,
    ///CHECK: stf anchor
    pub lp_mint_authority: AccountInfo<'info>,
    pub liq_pool_msol_leg: CpiAccount<'info, TokenAccount>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: many
    ///CHECK: stf anchor
    pub liq_pool_sol_leg_pda: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub transfer_from: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub mint_to: CpiAccount<'info, TokenAccount>,
    ///CHECK: many
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: many
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for AddLiquidity<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let lp_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let lp_mint_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_sol_leg_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_from: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let mint_to: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !lp_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_sol_leg_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !transfer_from.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !transfer_from.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !mint_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(AddLiquidity {
            state,
            lp_mint,
            lp_mint_authority,
            liq_pool_msol_leg,
            liq_pool_sol_leg_pda,
            transfer_from,
            mint_to,
            system_program,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for AddLiquidity<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.lp_mint.to_account_infos());
        account_infos.extend(self.lp_mint_authority.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg.to_account_infos());
        account_infos.extend(self.liq_pool_sol_leg_pda.to_account_infos());
        account_infos.extend(self.transfer_from.to_account_infos());
        account_infos.extend(self.mint_to.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for AddLiquidity<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.lp_mint.to_account_metas(None));
        account_metas.extend(self.lp_mint_authority.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg.to_account_metas(None));
        account_metas.extend(self.liq_pool_sol_leg_pda.to_account_metas(None));
        account_metas.extend(self.transfer_from.to_account_metas(Some(true)));
        account_metas.extend(self.mint_to.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for AddLiquidity<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.lp_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_sol_leg_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_from, program_id)?;
        anchor_lang::AccountsExit::exit(&self.mint_to, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_add_liquidity {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct AddLiquidity {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub lp_mint_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_sol_leg_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_from: anchor_lang::solana_program::pubkey::Pubkey,
        pub mint_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for AddLiquidity
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_mint_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_sol_leg_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_from, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_to, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for AddLiquidity {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.lp_mint,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.lp_mint_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.liq_pool_msol_leg,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_sol_leg_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_from,
                true,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.mint_to,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    pub lp_mint: CpiAccount<'info, Mint>,
    #[account(mut)]
    ///CHECK: many
    pub burn_from: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub burn_from_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub transfer_sol_to: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub transfer_msol_to: CpiAccount<'info, TokenAccount>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub liq_pool_sol_leg_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub liq_pool_msol_leg: CpiAccount<'info, TokenAccount>,
    ///CHECK: stf anchor
    pub liq_pool_msol_leg_authority: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for RemoveLiquidity<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let lp_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let burn_from: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let burn_from_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_sol_to: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_msol_to: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_sol_leg_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !lp_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !burn_from.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !burn_from_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !transfer_sol_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !transfer_msol_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_sol_leg_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_msol_leg.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(RemoveLiquidity {
            state,
            lp_mint,
            burn_from,
            burn_from_authority,
            transfer_sol_to,
            transfer_msol_to,
            liq_pool_sol_leg_pda,
            liq_pool_msol_leg,
            liq_pool_msol_leg_authority,
            system_program,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for RemoveLiquidity<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.lp_mint.to_account_infos());
        account_infos.extend(self.burn_from.to_account_infos());
        account_infos.extend(self.burn_from_authority.to_account_infos());
        account_infos.extend(self.transfer_sol_to.to_account_infos());
        account_infos.extend(self.transfer_msol_to.to_account_infos());
        account_infos.extend(self.liq_pool_sol_leg_pda.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg_authority.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for RemoveLiquidity<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.lp_mint.to_account_metas(None));
        account_metas.extend(self.burn_from.to_account_metas(None));
        account_metas.extend(self.burn_from_authority.to_account_metas(Some(true)));
        account_metas.extend(self.transfer_sol_to.to_account_metas(None));
        account_metas.extend(self.transfer_msol_to.to_account_metas(None));
        account_metas.extend(self.liq_pool_sol_leg_pda.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg_authority.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for RemoveLiquidity<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.lp_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.burn_from, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_sol_to, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_msol_to, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_sol_leg_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_msol_leg, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_remove_liquidity {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct RemoveLiquidity {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub lp_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub burn_from: anchor_lang::solana_program::pubkey::Pubkey,
        pub burn_from_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_sol_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_msol_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_sol_leg_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for RemoveLiquidity
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.lp_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.burn_from, writer)?;
            borsh::BorshSerialize::serialize(&self.burn_from_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_sol_to, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_msol_to, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_sol_leg_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for RemoveLiquidity {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.lp_mint,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.burn_from,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.burn_from_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_sol_to,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_msol_to,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_sol_leg_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_msol_leg,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.liq_pool_msol_leg_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct Deposit<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    pub msol_mint: CpiAccount<'info, Mint>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub liq_pool_sol_leg_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub liq_pool_msol_leg: CpiAccount<'info, TokenAccount>,
    ///CHECK: stf anchor
    pub liq_pool_msol_leg_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub transfer_from: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub mint_to: CpiAccount<'info, TokenAccount>,
    ///CHECK: stf anchor
    pub msol_mint_authority: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for Deposit<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_sol_leg_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_from: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let mint_to: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !msol_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_sol_leg_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_msol_leg.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !reserve_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !transfer_from.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !transfer_from.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !mint_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(Deposit {
            state,
            msol_mint,
            liq_pool_sol_leg_pda,
            liq_pool_msol_leg,
            liq_pool_msol_leg_authority,
            reserve_pda,
            transfer_from,
            mint_to,
            msol_mint_authority,
            system_program,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for Deposit<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.liq_pool_sol_leg_pda.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg_authority.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.transfer_from.to_account_infos());
        account_infos.extend(self.mint_to.to_account_infos());
        account_infos.extend(self.msol_mint_authority.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for Deposit<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.liq_pool_sol_leg_pda.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg_authority.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.transfer_from.to_account_metas(Some(true)));
        account_metas.extend(self.mint_to.to_account_metas(None));
        account_metas.extend(self.msol_mint_authority.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for Deposit<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.msol_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_sol_leg_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_msol_leg, program_id)?;
        anchor_lang::AccountsExit::exit(&self.reserve_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_from, program_id)?;
        anchor_lang::AccountsExit::exit(&self.mint_to, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_deposit {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct Deposit {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_sol_leg_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_from: anchor_lang::solana_program::pubkey::Pubkey,
        pub mint_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for Deposit
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_sol_leg_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_from, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_to, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for Deposit {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.msol_mint,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_sol_leg_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_msol_leg,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.liq_pool_msol_leg_authority,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.reserve_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_from,
                true,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.mint_to,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.msol_mint_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct DepositStakeAccount<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub duplication_flag: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub rent_payer: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub msol_mint: CpiAccount<'info, Mint>,
    #[account(mut)]
    ///CHECK: many
    pub mint_to: CpiAccount<'info, TokenAccount>,
    ///CHECK: stf anchor
    pub msol_mint_authority: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for DepositStakeAccount<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let duplication_flag: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent_payer: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let mint_to: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !stake_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !duplication_flag.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !rent_payer.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !rent_payer.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !msol_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !mint_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(DepositStakeAccount {
            state,
            validator_list,
            stake_list,
            stake_account,
            stake_authority,
            duplication_flag,
            rent_payer,
            msol_mint,
            mint_to,
            msol_mint_authority,
            clock,
            rent,
            system_program,
            token_program,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for DepositStakeAccount<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_authority.to_account_infos());
        account_infos.extend(self.duplication_flag.to_account_infos());
        account_infos.extend(self.rent_payer.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.mint_to.to_account_infos());
        account_infos.extend(self.msol_mint_authority.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for DepositStakeAccount<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_authority.to_account_metas(Some(true)));
        account_metas.extend(self.duplication_flag.to_account_metas(None));
        account_metas.extend(self.rent_payer.to_account_metas(Some(true)));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.mint_to.to_account_metas(None));
        account_metas.extend(self.msol_mint_authority.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for DepositStakeAccount<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.duplication_flag, program_id)?;
        anchor_lang::AccountsExit::exit(&self.rent_payer, program_id)?;
        anchor_lang::AccountsExit::exit(&self.msol_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.mint_to, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_deposit_stake_account {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct DepositStakeAccount {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub duplication_flag: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent_payer: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub mint_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for DepositStakeAccount
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.duplication_flag, writer)?;
            borsh::BorshSerialize::serialize(&self.rent_payer, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_to, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for DepositStakeAccount {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.duplication_flag,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.rent_payer,
                true,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.msol_mint,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.mint_to,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.msol_mint_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct LiquidUnstake<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    pub msol_mint: CpiAccount<'info, Mint>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub liq_pool_sol_leg_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub liq_pool_msol_leg: CpiAccount<'info, TokenAccount>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub treasury_msol_account: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub get_msol_from: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub get_msol_from_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub transfer_sol_to: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for LiquidUnstake<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_sol_leg_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let liq_pool_msol_leg: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let treasury_msol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let get_msol_from: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let get_msol_from_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_sol_to: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !msol_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_sol_leg_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !liq_pool_msol_leg.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !treasury_msol_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !get_msol_from.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !get_msol_from_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !transfer_sol_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(LiquidUnstake {
            state,
            msol_mint,
            liq_pool_sol_leg_pda,
            liq_pool_msol_leg,
            treasury_msol_account,
            get_msol_from,
            get_msol_from_authority,
            transfer_sol_to,
            system_program,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for LiquidUnstake<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.liq_pool_sol_leg_pda.to_account_infos());
        account_infos.extend(self.liq_pool_msol_leg.to_account_infos());
        account_infos.extend(self.treasury_msol_account.to_account_infos());
        account_infos.extend(self.get_msol_from.to_account_infos());
        account_infos.extend(self.get_msol_from_authority.to_account_infos());
        account_infos.extend(self.transfer_sol_to.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for LiquidUnstake<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.liq_pool_sol_leg_pda.to_account_metas(None));
        account_metas.extend(self.liq_pool_msol_leg.to_account_metas(None));
        account_metas.extend(self.treasury_msol_account.to_account_metas(None));
        account_metas.extend(self.get_msol_from.to_account_metas(None));
        account_metas.extend(self.get_msol_from_authority.to_account_metas(Some(true)));
        account_metas.extend(self.transfer_sol_to.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for LiquidUnstake<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.msol_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_sol_leg_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.liq_pool_msol_leg, program_id)?;
        anchor_lang::AccountsExit::exit(&self.treasury_msol_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.get_msol_from, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_sol_to, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_liquid_unstake {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct LiquidUnstake {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_sol_leg_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub liq_pool_msol_leg: anchor_lang::solana_program::pubkey::Pubkey,
        pub treasury_msol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub get_msol_from: anchor_lang::solana_program::pubkey::Pubkey,
        pub get_msol_from_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_sol_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for LiquidUnstake
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_sol_leg_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.liq_pool_msol_leg, writer)?;
            borsh::BorshSerialize::serialize(&self.treasury_msol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.get_msol_from, writer)?;
            borsh::BorshSerialize::serialize(&self.get_msol_from_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_sol_to, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for LiquidUnstake {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.msol_mint,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_sol_leg_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.liq_pool_msol_leg,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.treasury_msol_account,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.get_msol_from,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.get_msol_from_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_sol_to,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct AddValidator<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub manager_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub validator_vote: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub duplication_flag: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub rent_payer: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for AddValidator<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_vote: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let duplication_flag: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent_payer: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !duplication_flag.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !rent_payer.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !rent_payer.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(AddValidator {
            state,
            manager_authority,
            validator_list,
            validator_vote,
            duplication_flag,
            rent_payer,
            clock,
            rent,
            system_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for AddValidator<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.manager_authority.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.validator_vote.to_account_infos());
        account_infos.extend(self.duplication_flag.to_account_infos());
        account_infos.extend(self.rent_payer.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for AddValidator<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.manager_authority.to_account_metas(Some(true)));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.validator_vote.to_account_metas(None));
        account_metas.extend(self.duplication_flag.to_account_metas(None));
        account_metas.extend(self.rent_payer.to_account_metas(Some(true)));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for AddValidator<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.duplication_flag, program_id)?;
        anchor_lang::AccountsExit::exit(&self.rent_payer, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_add_validator {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct AddValidator {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_vote: anchor_lang::solana_program::pubkey::Pubkey,
        pub duplication_flag: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent_payer: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for AddValidator
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_vote, writer)?;
            borsh::BorshSerialize::serialize(&self.duplication_flag, writer)?;
            borsh::BorshSerialize::serialize(&self.rent_payer, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for AddValidator {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.manager_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.validator_vote,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.duplication_flag,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.rent_payer,
                true,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct RemoveValidator<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub manager_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub duplication_flag: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub operational_sol_account: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for RemoveValidator<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let duplication_flag: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let operational_sol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !duplication_flag.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !operational_sol_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(RemoveValidator {
            state,
            manager_authority,
            validator_list,
            duplication_flag,
            operational_sol_account,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for RemoveValidator<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.manager_authority.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.duplication_flag.to_account_infos());
        account_infos.extend(self.operational_sol_account.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for RemoveValidator<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.manager_authority.to_account_metas(Some(true)));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.duplication_flag.to_account_metas(None));
        account_metas.extend(self.operational_sol_account.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for RemoveValidator<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.duplication_flag, program_id)?;
        anchor_lang::AccountsExit::exit(&self.operational_sol_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_remove_validator {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct RemoveValidator {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub duplication_flag: anchor_lang::solana_program::pubkey::Pubkey,
        pub operational_sol_account: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for RemoveValidator
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.duplication_flag, writer)?;
            borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for RemoveValidator {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.manager_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.duplication_flag,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.operational_sol_account,
                false,
            ));
            account_metas
        }
    }
}
pub struct SetValidatorScore<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub manager_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for SetValidatorScore<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(SetValidatorScore {
            state,
            manager_authority,
            validator_list,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for SetValidatorScore<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.manager_authority.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for SetValidatorScore<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.manager_authority.to_account_metas(Some(true)));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for SetValidatorScore<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_set_validator_score {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct SetValidatorScore {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for SetValidatorScore
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for SetValidatorScore {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.manager_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas
        }
    }
}
pub struct ConfigValidatorSystem<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub manager_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for ConfigValidatorSystem<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(ConfigValidatorSystem {
            state,
            manager_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for ConfigValidatorSystem<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.manager_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for ConfigValidatorSystem<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.manager_authority.to_account_metas(Some(true)));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for ConfigValidatorSystem<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_config_validator_system {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct ConfigValidatorSystem {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for ConfigValidatorSystem
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.manager_authority, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for ConfigValidatorSystem {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.manager_authority,
                    true,
                ),
            );
            account_metas
        }
    }
}
pub struct OrderUnstake<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    pub msol_mint: CpiAccount<'info, Mint>,
    #[account(mut)]
    ///CHECK: many
    pub burn_msol_from: CpiAccount<'info, TokenAccount>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub burn_msol_authority: AccountInfo<'info>,
    # [account (zero , rent_exempt = enforce)]
    ///CHECK: many
    pub new_ticket_account: ProgramAccount<'info, TicketAccountData>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for OrderUnstake<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let burn_msol_from: CpiAccount<TokenAccount> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let burn_msol_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let new_ticket_account = &accounts[0];
        *accounts = &accounts[1..];
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !msol_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !burn_msol_from.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !burn_msol_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        let __anchor_rent = Rent::get()?;
        let new_ticket_account: anchor_lang::ProgramAccount<TicketAccountData> = {
            let mut __data: &[u8] = &new_ticket_account.try_borrow_data()?;
            let mut __disc_bytes = [0u8; 8];
            __disc_bytes.copy_from_slice(&__data[..8]);
            let __discriminator = u64::from_le_bytes(__disc_bytes);
            if __discriminator != 0 {
                return Err(anchor_lang::__private::ErrorCode::ConstraintZero.into());
            }
            anchor_lang::ProgramAccount::try_from_unchecked(program_id, &new_ticket_account)?
        };
        if !new_ticket_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !__anchor_rent.is_exempt(
            new_ticket_account.to_account_info().lamports(),
            new_ticket_account.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
        }
        Ok(OrderUnstake {
            state,
            msol_mint,
            burn_msol_from,
            burn_msol_authority,
            new_ticket_account,
            clock,
            rent,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for OrderUnstake<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.burn_msol_from.to_account_infos());
        account_infos.extend(self.burn_msol_authority.to_account_infos());
        account_infos.extend(self.new_ticket_account.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for OrderUnstake<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.burn_msol_from.to_account_metas(None));
        account_metas.extend(self.burn_msol_authority.to_account_metas(Some(true)));
        account_metas.extend(self.new_ticket_account.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for OrderUnstake<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.msol_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.burn_msol_from, program_id)?;
        anchor_lang::AccountsExit::exit(&self.new_ticket_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_order_unstake {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct OrderUnstake {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub burn_msol_from: anchor_lang::solana_program::pubkey::Pubkey,
        pub burn_msol_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub new_ticket_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for OrderUnstake
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.burn_msol_from, writer)?;
            borsh::BorshSerialize::serialize(&self.burn_msol_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.new_ticket_account, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for OrderUnstake {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.msol_mint,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.burn_msol_from,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.burn_msol_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.new_ticket_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct Claim<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub ticket_account: ProgramAccount<'info, TicketAccountData>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub transfer_sol_to: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for Claim<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let ticket_account: ProgramAccount<TicketAccountData> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let transfer_sol_to: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !reserve_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !ticket_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !transfer_sol_to.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(Claim {
            state,
            reserve_pda,
            ticket_account,
            transfer_sol_to,
            clock,
            system_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for Claim<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.ticket_account.to_account_infos());
        account_infos.extend(self.transfer_sol_to.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for Claim<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.ticket_account.to_account_metas(None));
        account_metas.extend(self.transfer_sol_to.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for Claim<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.reserve_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.ticket_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.transfer_sol_to, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_claim {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct Claim {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub ticket_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub transfer_sol_to: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for Claim
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.ticket_account, writer)?;
            borsh::BorshSerialize::serialize(&self.transfer_sol_to, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for Claim {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.reserve_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.ticket_account,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.transfer_sol_to,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct StakeReserve<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_vote: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_deposit_authority: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub epoch_schedule: Sysvar<'info, EpochSchedule>,
    pub rent: Sysvar<'info, Rent>,
    ///CHECK: stf anchor
    pub stake_history: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_config: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for StakeReserve<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_vote: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_deposit_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let epoch_schedule: Sysvar<EpochSchedule> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_history: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_config: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !validator_vote.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !reserve_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(StakeReserve {
            state,
            validator_list,
            stake_list,
            validator_vote,
            reserve_pda,
            stake_account,
            stake_deposit_authority,
            clock,
            epoch_schedule,
            rent,
            stake_history,
            stake_config,
            system_program,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for StakeReserve<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.validator_vote.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_deposit_authority.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.epoch_schedule.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.stake_history.to_account_infos());
        account_infos.extend(self.stake_config.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for StakeReserve<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.validator_vote.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_deposit_authority.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.epoch_schedule.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.stake_history.to_account_metas(None));
        account_metas.extend(self.stake_config.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for StakeReserve<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_vote, program_id)?;
        anchor_lang::AccountsExit::exit(&self.reserve_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_stake_reserve {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct StakeReserve {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_vote: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_deposit_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub epoch_schedule: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_history: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_config: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for StakeReserve
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_vote, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.epoch_schedule, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_history, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_config, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for StakeReserve {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_vote,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.reserve_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_deposit_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.epoch_schedule,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_history,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_config,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct UpdateCommon<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_withdraw_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub msol_mint: CpiAccount<'info, Mint>,
    ///CHECK: stf anchor
    pub msol_mint_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub treasury_msol_account: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    ///CHECK: stf anchor
    pub stake_history: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub token_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for UpdateCommon<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_withdraw_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint: CpiAccount<Mint> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let msol_mint_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let treasury_msol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_history: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let token_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !reserve_pda.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !msol_mint.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !treasury_msol_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(UpdateCommon {
            state,
            stake_list,
            stake_account,
            stake_withdraw_authority,
            reserve_pda,
            msol_mint,
            msol_mint_authority,
            treasury_msol_account,
            clock,
            stake_history,
            stake_program,
            token_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateCommon<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_withdraw_authority.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.msol_mint.to_account_infos());
        account_infos.extend(self.msol_mint_authority.to_account_infos());
        account_infos.extend(self.treasury_msol_account.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.stake_history.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos.extend(self.token_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for UpdateCommon<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_withdraw_authority.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.msol_mint.to_account_metas(None));
        account_metas.extend(self.msol_mint_authority.to_account_metas(None));
        account_metas.extend(self.treasury_msol_account.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.stake_history.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas.extend(self.token_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for UpdateCommon<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.reserve_pda, program_id)?;
        anchor_lang::AccountsExit::exit(&self.msol_mint, program_id)?;
        anchor_lang::AccountsExit::exit(&self.treasury_msol_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_update_common {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct UpdateCommon {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_withdraw_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint: anchor_lang::solana_program::pubkey::Pubkey,
        pub msol_mint_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub treasury_msol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_history: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub token_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for UpdateCommon
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_withdraw_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint, writer)?;
            borsh::BorshSerialize::serialize(&self.msol_mint_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.treasury_msol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_history, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            borsh::BorshSerialize::serialize(&self.token_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for UpdateCommon {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_withdraw_authority,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.reserve_pda,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.msol_mint,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.msol_mint_authority,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.treasury_msol_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_history,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.token_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct UpdateActive<'info> {
    pub common: UpdateCommon<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for UpdateActive<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let common: UpdateCommon<'info> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(UpdateActive {
            common,
            validator_list,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateActive<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.common.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for UpdateActive<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.common.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for UpdateActive<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.common, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_update_active {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub use __client_accounts_update_common::UpdateCommon;
    pub struct UpdateActive {
        pub common: __client_accounts_update_common::UpdateCommon,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for UpdateActive
    where
        __client_accounts_update_common::UpdateCommon: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.common, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for UpdateActive {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.common.to_account_metas(None));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas
        }
    }
}
impl<'info> Deref for UpdateActive<'info> {
    type Target = UpdateCommon<'info>;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}
impl<'info> DerefMut for UpdateActive<'info> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}
pub struct UpdateDeactivated<'info> {
    pub common: UpdateCommon<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub operational_sol_account: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for UpdateDeactivated<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let common: UpdateCommon<'info> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let operational_sol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !operational_sol_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(UpdateDeactivated {
            common,
            operational_sol_account,
            system_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateDeactivated<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.common.to_account_infos());
        account_infos.extend(self.operational_sol_account.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for UpdateDeactivated<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.common.to_account_metas(None));
        account_metas.extend(self.operational_sol_account.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for UpdateDeactivated<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.common, program_id)?;
        anchor_lang::AccountsExit::exit(&self.operational_sol_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_update_deactivated {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub use __client_accounts_update_common::UpdateCommon;
    pub struct UpdateDeactivated {
        pub common: __client_accounts_update_common::UpdateCommon,
        pub operational_sol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for UpdateDeactivated
    where
        __client_accounts_update_common::UpdateCommon: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.common, writer)?;
            borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for UpdateDeactivated {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.common.to_account_metas(None));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.operational_sol_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
impl<'info> Deref for UpdateDeactivated<'info> {
    type Target = UpdateCommon<'info>;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}
impl<'info> DerefMut for UpdateDeactivated<'info> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}
pub struct SetLpParams<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub admin_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for SetLpParams<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let admin_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !admin_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(SetLpParams {
            state,
            admin_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for SetLpParams<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.admin_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for SetLpParams<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.admin_authority.to_account_metas(Some(true)));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for SetLpParams<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_set_lp_params {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct SetLpParams {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub admin_authority: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for SetLpParams
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.admin_authority, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for SetLpParams {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.admin_authority,
                    true,
                ),
            );
            account_metas
        }
    }
}
pub struct ConfigMarinadeParams {
    pub rewards_fee: Option<Fee>,
    pub slots_for_stake_delta: Option<u64>,
    pub min_stake: Option<u64>,
    pub min_deposit: Option<u64>,
    pub min_withdraw: Option<u64>,
    pub staking_sol_cap: Option<u64>,
    pub liquidity_sol_cap: Option<u64>,
    pub auto_add_validator_enabled: Option<bool>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for ConfigMarinadeParams {
    #[inline]
    fn clone(&self) -> ConfigMarinadeParams {
        {
            let _: ::core::clone::AssertParamIsClone<Option<Fee>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<u64>>;
            let _: ::core::clone::AssertParamIsClone<Option<bool>>;
            *self
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::marker::Copy for ConfigMarinadeParams {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for ConfigMarinadeParams {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Self {
                rewards_fee: ref __self_0_0,
                slots_for_stake_delta: ref __self_0_1,
                min_stake: ref __self_0_2,
                min_deposit: ref __self_0_3,
                min_withdraw: ref __self_0_4,
                staking_sol_cap: ref __self_0_5,
                liquidity_sol_cap: ref __self_0_6,
                auto_add_validator_enabled: ref __self_0_7,
            } => {
                let debug_trait_builder =
                    &mut ::core::fmt::Formatter::debug_struct(f, "ConfigMarinadeParams");
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "rewards_fee",
                    &&(*__self_0_0),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "slots_for_stake_delta",
                    &&(*__self_0_1),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "min_stake",
                    &&(*__self_0_2),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "min_deposit",
                    &&(*__self_0_3),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "min_withdraw",
                    &&(*__self_0_4),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "staking_sol_cap",
                    &&(*__self_0_5),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "liquidity_sol_cap",
                    &&(*__self_0_6),
                );
                let _ = ::core::fmt::DebugStruct::field(
                    debug_trait_builder,
                    "auto_add_validator_enabled",
                    &&(*__self_0_7),
                );
                ::core::fmt::DebugStruct::finish(debug_trait_builder)
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::default::Default for ConfigMarinadeParams {
    #[inline]
    fn default() -> ConfigMarinadeParams {
        ConfigMarinadeParams {
            rewards_fee: ::core::default::Default::default(),
            slots_for_stake_delta: ::core::default::Default::default(),
            min_stake: ::core::default::Default::default(),
            min_deposit: ::core::default::Default::default(),
            min_withdraw: ::core::default::Default::default(),
            staking_sol_cap: ::core::default::Default::default(),
            liquidity_sol_cap: ::core::default::Default::default(),
            auto_add_validator_enabled: ::core::default::Default::default(),
        }
    }
}
impl ::core::marker::StructuralPartialEq for ConfigMarinadeParams {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for ConfigMarinadeParams {
    #[inline]
    fn eq(&self, other: &ConfigMarinadeParams) -> bool {
        match *other {
            Self {
                rewards_fee: ref __self_1_0,
                slots_for_stake_delta: ref __self_1_1,
                min_stake: ref __self_1_2,
                min_deposit: ref __self_1_3,
                min_withdraw: ref __self_1_4,
                staking_sol_cap: ref __self_1_5,
                liquidity_sol_cap: ref __self_1_6,
                auto_add_validator_enabled: ref __self_1_7,
            } => match *self {
                Self {
                    rewards_fee: ref __self_0_0,
                    slots_for_stake_delta: ref __self_0_1,
                    min_stake: ref __self_0_2,
                    min_deposit: ref __self_0_3,
                    min_withdraw: ref __self_0_4,
                    staking_sol_cap: ref __self_0_5,
                    liquidity_sol_cap: ref __self_0_6,
                    auto_add_validator_enabled: ref __self_0_7,
                } => {
                    (*__self_0_0) == (*__self_1_0)
                        && (*__self_0_1) == (*__self_1_1)
                        && (*__self_0_2) == (*__self_1_2)
                        && (*__self_0_3) == (*__self_1_3)
                        && (*__self_0_4) == (*__self_1_4)
                        && (*__self_0_5) == (*__self_1_5)
                        && (*__self_0_6) == (*__self_1_6)
                        && (*__self_0_7) == (*__self_1_7)
                }
            },
        }
    }
    #[inline]
    fn ne(&self, other: &ConfigMarinadeParams) -> bool {
        match *other {
            Self {
                rewards_fee: ref __self_1_0,
                slots_for_stake_delta: ref __self_1_1,
                min_stake: ref __self_1_2,
                min_deposit: ref __self_1_3,
                min_withdraw: ref __self_1_4,
                staking_sol_cap: ref __self_1_5,
                liquidity_sol_cap: ref __self_1_6,
                auto_add_validator_enabled: ref __self_1_7,
            } => match *self {
                Self {
                    rewards_fee: ref __self_0_0,
                    slots_for_stake_delta: ref __self_0_1,
                    min_stake: ref __self_0_2,
                    min_deposit: ref __self_0_3,
                    min_withdraw: ref __self_0_4,
                    staking_sol_cap: ref __self_0_5,
                    liquidity_sol_cap: ref __self_0_6,
                    auto_add_validator_enabled: ref __self_0_7,
                } => {
                    (*__self_0_0) != (*__self_1_0)
                        || (*__self_0_1) != (*__self_1_1)
                        || (*__self_0_2) != (*__self_1_2)
                        || (*__self_0_3) != (*__self_1_3)
                        || (*__self_0_4) != (*__self_1_4)
                        || (*__self_0_5) != (*__self_1_5)
                        || (*__self_0_6) != (*__self_1_6)
                        || (*__self_0_7) != (*__self_1_7)
                }
            },
        }
    }
}
impl borsh::ser::BorshSerialize for ConfigMarinadeParams
where
    Option<Fee>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<u64>: borsh::ser::BorshSerialize,
    Option<bool>: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.rewards_fee, writer)?;
        borsh::BorshSerialize::serialize(&self.slots_for_stake_delta, writer)?;
        borsh::BorshSerialize::serialize(&self.min_stake, writer)?;
        borsh::BorshSerialize::serialize(&self.min_deposit, writer)?;
        borsh::BorshSerialize::serialize(&self.min_withdraw, writer)?;
        borsh::BorshSerialize::serialize(&self.staking_sol_cap, writer)?;
        borsh::BorshSerialize::serialize(&self.liquidity_sol_cap, writer)?;
        borsh::BorshSerialize::serialize(&self.auto_add_validator_enabled, writer)?;
        Ok(())
    }
}
impl borsh::de::BorshDeserialize for ConfigMarinadeParams
where
    Option<Fee>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<u64>: borsh::BorshDeserialize,
    Option<bool>: borsh::BorshDeserialize,
{
    fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            rewards_fee: borsh::BorshDeserialize::deserialize(buf)?,
            slots_for_stake_delta: borsh::BorshDeserialize::deserialize(buf)?,
            min_stake: borsh::BorshDeserialize::deserialize(buf)?,
            min_deposit: borsh::BorshDeserialize::deserialize(buf)?,
            min_withdraw: borsh::BorshDeserialize::deserialize(buf)?,
            staking_sol_cap: borsh::BorshDeserialize::deserialize(buf)?,
            liquidity_sol_cap: borsh::BorshDeserialize::deserialize(buf)?,
            auto_add_validator_enabled: borsh::BorshDeserialize::deserialize(buf)?,
        })
    }
}
pub struct ConfigMarinade<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub admin_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for ConfigMarinade<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let admin_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !admin_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(ConfigMarinade {
            state,
            admin_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for ConfigMarinade<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.admin_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for ConfigMarinade<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.admin_authority.to_account_metas(Some(true)));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for ConfigMarinade<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_config_marinade {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct ConfigMarinade {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub admin_authority: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for ConfigMarinade
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.admin_authority, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for ConfigMarinade {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.admin_authority,
                    true,
                ),
            );
            account_metas
        }
    }
}
pub struct DeactivateStake<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_deposit_authority: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub split_stake_account: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub split_stake_rent_payer: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    pub epoch_schedule: Sysvar<'info, EpochSchedule>,
    ///CHECK: stf anchor
    pub stake_history: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for DeactivateStake<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_deposit_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let split_stake_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let split_stake_rent_payer: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let epoch_schedule: Sysvar<EpochSchedule> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_history: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !split_stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !split_stake_account.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !split_stake_rent_payer.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !split_stake_rent_payer.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(DeactivateStake {
            state,
            reserve_pda,
            validator_list,
            stake_list,
            stake_account,
            stake_deposit_authority,
            split_stake_account,
            split_stake_rent_payer,
            clock,
            rent,
            epoch_schedule,
            stake_history,
            system_program,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for DeactivateStake<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_deposit_authority.to_account_infos());
        account_infos.extend(self.split_stake_account.to_account_infos());
        account_infos.extend(self.split_stake_rent_payer.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.epoch_schedule.to_account_infos());
        account_infos.extend(self.stake_history.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for DeactivateStake<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_deposit_authority.to_account_metas(None));
        account_metas.extend(self.split_stake_account.to_account_metas(Some(true)));
        account_metas.extend(self.split_stake_rent_payer.to_account_metas(Some(true)));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.epoch_schedule.to_account_metas(None));
        account_metas.extend(self.stake_history.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for DeactivateStake<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.split_stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.split_stake_rent_payer, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_deactivate_stake {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct DeactivateStake {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_deposit_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub split_stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub split_stake_rent_payer: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub epoch_schedule: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_history: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for DeactivateStake
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.split_stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.split_stake_rent_payer, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.epoch_schedule, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_history, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for DeactivateStake {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.reserve_pda,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_deposit_authority,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.split_stake_account,
                true,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.split_stake_rent_payer,
                true,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.epoch_schedule,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_history,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct EmergencyUnstake<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_manager_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_deposit_authority: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for EmergencyUnstake<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_deposit_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !validator_manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(EmergencyUnstake {
            state,
            validator_manager_authority,
            validator_list,
            stake_list,
            stake_account,
            stake_deposit_authority,
            clock,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for EmergencyUnstake<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.validator_manager_authority.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_deposit_authority.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for EmergencyUnstake<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(
            self.validator_manager_authority
                .to_account_metas(Some(true)),
        );
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_deposit_authority.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for EmergencyUnstake<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_emergency_unstake {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct EmergencyUnstake {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_deposit_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for EmergencyUnstake
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for EmergencyUnstake {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.validator_manager_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_deposit_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct PartialUnstake<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_manager_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub stake_account: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_deposit_authority: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub reserve_pda: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub split_stake_account: AccountInfo<'info>,
    #[account(mut, signer)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub split_stake_rent_payer: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    ///CHECK: stf anchor
    pub stake_history: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub system_program: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for PartialUnstake<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_manager_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_account: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_deposit_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let reserve_pda: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let split_stake_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let split_stake_rent_payer: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let rent: Sysvar<Rent> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_history: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let system_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !validator_manager_authority.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !split_stake_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !split_stake_account.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        if !split_stake_rent_payer.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if true {
            if !split_stake_rent_payer.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
        Ok(PartialUnstake {
            state,
            validator_manager_authority,
            validator_list,
            stake_list,
            stake_account,
            stake_deposit_authority,
            reserve_pda,
            split_stake_account,
            split_stake_rent_payer,
            clock,
            rent,
            stake_history,
            system_program,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for PartialUnstake<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.validator_manager_authority.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.stake_account.to_account_infos());
        account_infos.extend(self.stake_deposit_authority.to_account_infos());
        account_infos.extend(self.reserve_pda.to_account_infos());
        account_infos.extend(self.split_stake_account.to_account_infos());
        account_infos.extend(self.split_stake_rent_payer.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.rent.to_account_infos());
        account_infos.extend(self.stake_history.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for PartialUnstake<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(
            self.validator_manager_authority
                .to_account_metas(Some(true)),
        );
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.stake_account.to_account_metas(None));
        account_metas.extend(self.stake_deposit_authority.to_account_metas(None));
        account_metas.extend(self.reserve_pda.to_account_metas(None));
        account_metas.extend(self.split_stake_account.to_account_metas(Some(true)));
        account_metas.extend(self.split_stake_rent_payer.to_account_metas(Some(true)));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.rent.to_account_metas(None));
        account_metas.extend(self.stake_history.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for PartialUnstake<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.split_stake_account, program_id)?;
        anchor_lang::AccountsExit::exit(&self.split_stake_rent_payer, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_partial_unstake {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct PartialUnstake {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_manager_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_deposit_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub reserve_pda: anchor_lang::solana_program::pubkey::Pubkey,
        pub split_stake_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub split_stake_rent_payer: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub rent: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_history: anchor_lang::solana_program::pubkey::Pubkey,
        pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for PartialUnstake
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_manager_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.reserve_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.split_stake_account, writer)?;
            borsh::BorshSerialize::serialize(&self.split_stake_rent_payer, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.rent, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_history, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for PartialUnstake {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.validator_manager_authority,
                    true,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_deposit_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.reserve_pda,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.split_stake_account,
                true,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.split_stake_rent_payer,
                true,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.rent, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_history,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.system_program,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
pub struct MergeStakes<'info> {
    #[account(mut)]
    ///CHECK: many
    pub state: ProgramAccount<'info, State>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub stake_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub validator_list: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    pub destination_stake: CpiAccount<'info, StakeWrapper>,
    #[account(mut)]
    ///CHECK: many
    pub source_stake: CpiAccount<'info, StakeWrapper>,
    ///CHECK: stf anchor
    pub stake_deposit_authority: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_withdraw_authority: AccountInfo<'info>,
    #[account(mut)]
    ///CHECK: many
    ///CHECK: stf anchor
    pub operational_sol_account: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
    ///CHECK: stf anchor
    pub stake_history: AccountInfo<'info>,
    ///CHECK: stf anchor
    pub stake_program: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info> for MergeStakes<'info>
where
    'info: 'info,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
        ix_data: &[u8],
    ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
        let state: ProgramAccount<State> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let validator_list: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let destination_stake: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let source_stake: CpiAccount<StakeWrapper> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_deposit_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_withdraw_authority: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let operational_sol_account: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let clock: Sysvar<Clock> =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_history: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        let stake_program: AccountInfo =
            anchor_lang::Accounts::try_accounts(program_id, accounts, ix_data)?;
        if !state.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !stake_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !validator_list.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !destination_stake.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !source_stake.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        if !operational_sol_account.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
        Ok(MergeStakes {
            state,
            stake_list,
            validator_list,
            destination_stake,
            source_stake,
            stake_deposit_authority,
            stake_withdraw_authority,
            operational_sol_account,
            clock,
            stake_history,
            stake_program,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for MergeStakes<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.state.to_account_infos());
        account_infos.extend(self.stake_list.to_account_infos());
        account_infos.extend(self.validator_list.to_account_infos());
        account_infos.extend(self.destination_stake.to_account_infos());
        account_infos.extend(self.source_stake.to_account_infos());
        account_infos.extend(self.stake_deposit_authority.to_account_infos());
        account_infos.extend(self.stake_withdraw_authority.to_account_infos());
        account_infos.extend(self.operational_sol_account.to_account_infos());
        account_infos.extend(self.clock.to_account_infos());
        account_infos.extend(self.stake_history.to_account_infos());
        account_infos.extend(self.stake_program.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for MergeStakes<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.state.to_account_metas(None));
        account_metas.extend(self.stake_list.to_account_metas(None));
        account_metas.extend(self.validator_list.to_account_metas(None));
        account_metas.extend(self.destination_stake.to_account_metas(None));
        account_metas.extend(self.source_stake.to_account_metas(None));
        account_metas.extend(self.stake_deposit_authority.to_account_metas(None));
        account_metas.extend(self.stake_withdraw_authority.to_account_metas(None));
        account_metas.extend(self.operational_sol_account.to_account_metas(None));
        account_metas.extend(self.clock.to_account_metas(None));
        account_metas.extend(self.stake_history.to_account_metas(None));
        account_metas.extend(self.stake_program.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for MergeStakes<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
        anchor_lang::AccountsExit::exit(&self.state, program_id)?;
        anchor_lang::AccountsExit::exit(&self.stake_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.validator_list, program_id)?;
        anchor_lang::AccountsExit::exit(&self.destination_stake, program_id)?;
        anchor_lang::AccountsExit::exit(&self.source_stake, program_id)?;
        anchor_lang::AccountsExit::exit(&self.operational_sol_account, program_id)?;
        Ok(())
    }
}
/// An internal, Anchor generated module. This is used (as an
/// implementation detail), to generate a struct for a given
/// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
/// instead of an `AccountInfo`. This is useful for clients that want
/// to generate a list of accounts, without explicitly knowing the
/// order all the fields should be in.
///
/// To access the struct in this module, one should use the sibling
/// `accounts` module (also generated), which re-exports this.
pub(crate) mod __client_accounts_merge_stakes {
    use super::*;
    use anchor_lang::prelude::borsh;
    pub struct MergeStakes {
        pub state: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub validator_list: anchor_lang::solana_program::pubkey::Pubkey,
        pub destination_stake: anchor_lang::solana_program::pubkey::Pubkey,
        pub source_stake: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_deposit_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_withdraw_authority: anchor_lang::solana_program::pubkey::Pubkey,
        pub operational_sol_account: anchor_lang::solana_program::pubkey::Pubkey,
        pub clock: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_history: anchor_lang::solana_program::pubkey::Pubkey,
        pub stake_program: anchor_lang::solana_program::pubkey::Pubkey,
    }
    impl borsh::ser::BorshSerialize for MergeStakes
    where
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
        anchor_lang::solana_program::pubkey::Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.state, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_list, writer)?;
            borsh::BorshSerialize::serialize(&self.validator_list, writer)?;
            borsh::BorshSerialize::serialize(&self.destination_stake, writer)?;
            borsh::BorshSerialize::serialize(&self.source_stake, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_deposit_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_withdraw_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.operational_sol_account, writer)?;
            borsh::BorshSerialize::serialize(&self.clock, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_history, writer)?;
            borsh::BorshSerialize::serialize(&self.stake_program, writer)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for MergeStakes {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.state, false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.stake_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.validator_list,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.destination_stake,
                false,
            ));
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.source_stake,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_deposit_authority,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_withdraw_authority,
                    false,
                ),
            );
            account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                self.operational_sol_account,
                false,
            ));
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.clock, false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_history,
                    false,
                ),
            );
            account_metas.push(
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    self.stake_program,
                    false,
                ),
            );
            account_metas
        }
    }
}
