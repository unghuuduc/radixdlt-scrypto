use sbor::{describe::Type, *};

use crate::crypto::*;
use crate::engine::{api::*, call_engine, types::VaultId};
use crate::math::*;
use crate::misc::*;
use crate::resource::*;
use crate::resource_def;
use crate::rust::borrow::ToOwned;
use crate::rust::fmt;
use crate::rust::str::FromStr;
use crate::rust::string::String;
use crate::rust::vec::Vec;
use crate::types::*;

/// Represents a persistent resource container on ledger state.
pub struct Vault(pub VaultId);

impl Vault {
    /// Creates an empty vault to permanently hold resource of the given definition.
    pub fn new(resource_def_id: ResourceDefId) -> Self {
        let input = CreateEmptyVaultInput {
            resource_def_id: resource_def_id,
        };
        let output: CreateEmptyVaultOutput = call_engine(CREATE_EMPTY_VAULT, input);

        Self(output.vault_id)
    }

    /// Creates an empty vault and fills it with an initial bucket of resources.
    pub fn with_bucket(bucket: Bucket) -> Self {
        let mut vault = Vault::new(bucket.resource_def_id());
        vault.put(bucket);
        vault
    }

    /// Puts a bucket of resources into this vault.
    pub fn put(&mut self, bucket: Bucket) {
        let input = PutIntoVaultInput {
            vault_id: self.0,
            bucket_id: bucket.0,
        };
        let _: PutIntoVaultOutput = call_engine(PUT_INTO_VAULT, input);
    }

    /// Takes some amount of resource from this vault into a bucket.
    pub fn take<A: Into<Decimal>>(&mut self, amount: A) -> Bucket {
        let input = TakeFromVaultInput {
            vault_id: self.0,
            amount: amount.into(),
            auth: None,
        };
        let output: TakeFromVaultOutput = call_engine(TAKE_FROM_VAULT, input);

        Bucket(output.bucket_id)
    }

    /// Takes some amount of resource from this vault into a bucket.
    ///
    /// This variant of `take` accepts an additional auth parameter to support resources
    /// with or without `RESTRICTED_TRANSFER` flag on.
    pub fn take_with_auth<A: Into<Decimal>>(&mut self, amount: A, auth: Proof) -> Bucket {
        let input = TakeFromVaultInput {
            vault_id: self.0,
            amount: amount.into(),
            auth: Some(auth.0),
        };
        let output: TakeFromVaultOutput = call_engine(TAKE_FROM_VAULT, input);

        Bucket(output.bucket_id)
    }

    /// Takes all resource stored in this vault.
    pub fn take_all(&mut self) -> Bucket {
        self.take(self.amount())
    }

    /// Takes all resource stored in this vault.
    ///
    /// This variant of `take_all` accepts an additional auth parameter to support resources
    /// with or without `RESTRICTED_TRANSFER` flag on.
    pub fn take_all_with_auth(&mut self, auth: Proof) -> Bucket {
        self.take_with_auth(self.amount(), auth)
    }

    /// Takes a non-fungible from this vault, by id.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible vault or the specified non-fungible is not found.
    pub fn take_non_fungible(&self, non_fungible_id: &NonFungibleId) -> Bucket {
        let input = TakeNonFungibleFromVaultInput {
            vault_id: self.0,
            non_fungible_id: non_fungible_id.clone(),
            auth: None,
        };
        let output: TakeNonFungibleFromVaultOutput =
            call_engine(TAKE_NON_FUNGIBLE_FROM_VAULT, input);

        Bucket(output.bucket_id)
    }

    /// Takes a non-fungible from this vault, by id.
    ///
    /// This variant of `take_non_fungible` accepts an additional auth parameter to support resources
    /// with or without `RESTRICTED_TRANSFER` flag on.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible vault or the specified non-fungible is not found.
    pub fn take_non_fungible_with_auth(
        &self,
        non_fungible_id: &NonFungibleId,
        auth: Proof,
    ) -> Bucket {
        let input = TakeNonFungibleFromVaultInput {
            vault_id: self.0,
            non_fungible_id: non_fungible_id.clone(),
            auth: Some(auth.0),
        };
        let output: TakeNonFungibleFromVaultOutput =
            call_engine(TAKE_NON_FUNGIBLE_FROM_VAULT, input);

        Bucket(output.bucket_id)
    }

    /// This is a convenience method for using the contained resource for authorization.
    ///
    /// It conducts the following actions in one shot:
    /// 1. Takes `1` resource from this vault into a bucket;
    /// 2. Creates a `Proof`.
    /// 3. Applies the specified function `f` with the created proof;
    /// 4. Puts the `1` resource back into this vault.
    ///
    pub fn authorize<F: FnOnce(Proof) -> O, O>(&mut self, f: F) -> O {
        let bucket = self.take(1);
        let output = f(bucket.present());
        self.put(bucket);
        output
    }

    /// This is a convenience method for using the contained resource for authorization.
    ///
    /// It conducts the following actions in one shot:
    /// 1. Takes `1` resource from this vault into a bucket;
    /// 2. Creates a `Proof`.
    /// 3. Applies the specified function `f` with the created proof;
    /// 4. Puts the `1` resource back into this vault.
    ///
    /// This variant of `authorize` accepts an additional auth parameter to support resources
    /// with or without `RESTRICTED_TRANSFER` flag on.
    ///
    pub fn authorize_with_auth<F: FnOnce(Proof) -> O, O>(&mut self, f: F, auth: Proof) -> O {
        let bucket = self.take_with_auth(1, auth);
        let output = f(bucket.present());
        self.put(bucket);
        output
    }

    /// Returns the amount of resources within this vault.
    pub fn amount(&self) -> Decimal {
        let input = GetVaultAmountInput { vault_id: self.0 };
        let output: GetVaultAmountOutput = call_engine(GET_VAULT_AMOUNT, input);

        output.amount
    }

    /// Returns the resource definition of resources within this vault.
    pub fn resource_def_id(&self) -> ResourceDefId {
        let input = GetVaultResourceDefIdInput { vault_id: self.0 };
        let output: GetVaultResourceDefIdOutput = call_engine(GET_VAULT_RESOURCE_DEF_ID, input);

        output.resource_def_id
    }

    /// Checks if this vault is empty.
    pub fn is_empty(&self) -> bool {
        self.amount() == 0.into()
    }

    /// Returns all the non-fungible units contained.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible vault.
    pub fn get_non_fungibles<T: NonFungibleData>(&self) -> Vec<NonFungible<T>> {
        let input = GetNonFungibleIdsInVaultInput { vault_id: self.0 };
        let output: GetNonFungibleIdsInVaultOutput =
            call_engine(GET_NON_FUNGIBLE_IDS_IN_VAULT, input);
        let resource_def_id = self.resource_def_id();
        output
            .non_fungible_ids
            .iter()
            .map(|id| NonFungible::from(NonFungibleAddress::new(resource_def_id, id.clone())))
            .collect()
    }

    /// Get all non-fungible IDs in this vault.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible vault.
    pub fn get_non_fungible_ids(&self) -> Vec<NonFungibleId> {
        let input = GetNonFungibleIdsInVaultInput { vault_id: self.0 };
        let output: GetNonFungibleIdsInVaultOutput =
            call_engine(GET_NON_FUNGIBLE_IDS_IN_VAULT, input);

        output.non_fungible_ids
    }

    /// Returns the id of a singleton non-fungible.
    ///
    /// # Panic
    /// If this vault is empty or contains more than one non-fungibles.
    pub fn get_non_fungible_id(&self) -> NonFungibleId {
        let non_fungible_ids = self.get_non_fungible_ids();
        assert!(
            non_fungible_ids.len() == 1,
            "Expect 1 non-fungible, but found {}",
            non_fungible_ids.len()
        );
        non_fungible_ids[0].clone()
    }

    /// Returns the data of a non-fungible unit, both the immutable and mutable parts.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible bucket.
    pub fn get_non_fungible_data<T: NonFungibleData>(&self, id: &NonFungibleId) -> T {
        resource_def!(self.resource_def_id()).get_non_fungible_data(id)
    }

    /// Updates the mutable part of the data of a non-fungible unit.
    ///
    /// # Panics
    /// Panics if this is not a non-fungible vault or the specified non-fungible is not found.
    pub fn update_non_fungible_data<T: NonFungibleData>(
        &self,
        id: &NonFungibleId,
        new_data: T,
        auth: Proof,
    ) {
        resource_def!(self.resource_def_id()).update_non_fungible_data(id, new_data, auth)
    }
}

//========
// error
//========

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseVaultError {
    InvalidHex(String),
    InvalidLength(usize),
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseVaultError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseVaultError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//========
// binary
//========

impl TryFrom<&[u8]> for Vault {
    type Error = ParseVaultError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        match slice.len() {
            36 => Ok(Self((
                Hash(copy_u8_array(&slice[0..32])),
                u32::from_le_bytes(copy_u8_array(&slice[32..])),
            ))),
            _ => Err(ParseVaultError::InvalidLength(slice.len())),
        }
    }
}

impl Vault {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = self.0 .0.to_vec();
        v.extend(self.0 .1.to_le_bytes());
        v
    }
}

custom_type!(Vault, CustomType::Vault, Vec::new());

//======
// text
//======

impl FromStr for Vault {
    type Err = ParseVaultError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ParseVaultError::InvalidHex(s.to_owned()))?;
        Self::try_from(bytes.as_slice())
    }
}

impl fmt::Display for Vault {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", hex::encode(self.to_vec()))
    }
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}
