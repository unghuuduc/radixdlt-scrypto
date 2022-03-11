mod bucket;
mod non_fungible;
mod non_fungible_data;
mod non_fungible_address;
mod non_fungible_key;
mod proof;
mod resource_builder;
mod resource_def;
mod resource_type;
mod supply;
mod system;
mod vault;

/// Resource flags.
pub mod resource_flags;
/// Resource permissions.
pub mod resource_permissions;

pub use bucket::{Bucket, ParseBucketError};
pub use non_fungible::NonFungible;
pub use non_fungible_data::NonFungibleData;
pub use non_fungible_address::NonFungibleAddress;
pub use non_fungible_key::{NonFungibleKey, ParseNonFungibleKeyError};
pub use proof::{ParseProofError, Proof};
pub use resource_builder::{ResourceBuilder, DIVISIBILITY_MAXIMUM, DIVISIBILITY_NONE};
pub use resource_def::{ParseResourceDefIdError, ResourceDef, ResourceDefId};
pub use resource_flags::*;
pub use resource_permissions::*;
pub use resource_type::ResourceType;
pub use supply::Supply;
pub use system::{init_resource_system, resource_system, ResourceSystem};
pub use vault::{ParseVaultError, Vault};
