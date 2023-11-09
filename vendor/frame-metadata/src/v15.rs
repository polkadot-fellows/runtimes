// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(feature = "decode")]
use codec::Decode;
#[cfg(feature = "serde_full")]
use serde::Serialize;

use super::{RuntimeMetadataPrefixed, META_RESERVED};
use codec::Encode;
use scale_info::{
	form::{Form, MetaForm, PortableForm},
	prelude::{collections::BTreeMap, vec::Vec},
	IntoPortable, MetaType, PortableRegistry, Registry,
};

pub use super::v14::{
	PalletCallMetadata, PalletConstantMetadata, PalletErrorMetadata, PalletEventMetadata,
	PalletStorageMetadata, StorageEntryMetadata, StorageEntryModifier, StorageEntryType,
	StorageHasher,
};

/// Latest runtime metadata
pub type RuntimeMetadataLastVersion = RuntimeMetadataV15;

impl From<RuntimeMetadataLastVersion> for super::RuntimeMetadataPrefixed {
	fn from(metadata: RuntimeMetadataLastVersion) -> RuntimeMetadataPrefixed {
		RuntimeMetadataPrefixed(META_RESERVED, super::RuntimeMetadata::V15(metadata))
	}
}

/// The metadata of a runtime.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub struct RuntimeMetadataV15 {
	/// Type registry containing all types used in the metadata.
	pub types: PortableRegistry,
	/// Metadata of all the pallets.
	pub pallets: Vec<PalletMetadata<PortableForm>>,
	/// Metadata of the extrinsic.
	pub extrinsic: ExtrinsicMetadata<PortableForm>,
	/// The type of the `Runtime`.
	pub ty: <PortableForm as Form>::Type,
	/// Metadata of the Runtime API.
	pub apis: Vec<RuntimeApiMetadata<PortableForm>>,
	/// The outer enums types as found in the runtime.
	pub outer_enums: OuterEnums<PortableForm>,
	/// Allows users to add custom types to the metadata.
	pub custom: CustomMetadata<PortableForm>,
}

impl RuntimeMetadataV15 {
	/// Create a new instance of [`RuntimeMetadataV15`].
	pub fn new(
		pallets: Vec<PalletMetadata>,
		extrinsic: ExtrinsicMetadata,
		runtime_type: MetaType,
		apis: Vec<RuntimeApiMetadata>,
		outer_enums: OuterEnums,
		custom: CustomMetadata,
	) -> Self {
		let mut registry = Registry::new();
		let pallets = registry.map_into_portable(pallets);
		let extrinsic = extrinsic.into_portable(&mut registry);
		let ty = registry.register_type(&runtime_type);
		let apis = registry.map_into_portable(apis);
		let outer_enums = outer_enums.into_portable(&mut registry);
		let custom = custom.into_portable(&mut registry);

		Self {
			types: registry.into(),
			pallets,
			extrinsic,
			ty,
			apis,
			outer_enums,
			custom,
		}
	}
}

/// Metadata of a runtime trait.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct RuntimeApiMetadata<T: Form = MetaForm> {
	/// Trait name.
	pub name: T::String,
	/// Trait methods.
	pub methods: Vec<RuntimeApiMethodMetadata<T>>,
	/// Trait documentation.
	pub docs: Vec<T::String>,
}

impl IntoPortable for RuntimeApiMetadata {
	type Output = RuntimeApiMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		RuntimeApiMetadata {
			name: self.name.into_portable(registry),
			methods: registry.map_into_portable(self.methods),
			docs: registry.map_into_portable(self.docs),
		}
	}
}

/// Metadata of a runtime method.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct RuntimeApiMethodMetadata<T: Form = MetaForm> {
	/// Method name.
	pub name: T::String,
	/// Method parameters.
	pub inputs: Vec<RuntimeApiMethodParamMetadata<T>>,
	/// Method output.
	pub output: T::Type,
	/// Method documentation.
	pub docs: Vec<T::String>,
}

impl IntoPortable for RuntimeApiMethodMetadata {
	type Output = RuntimeApiMethodMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		RuntimeApiMethodMetadata {
			name: self.name.into_portable(registry),
			inputs: registry.map_into_portable(self.inputs),
			output: registry.register_type(&self.output),
			docs: registry.map_into_portable(self.docs),
		}
	}
}

/// Metadata of a runtime method parameter.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct RuntimeApiMethodParamMetadata<T: Form = MetaForm> {
	/// Parameter name.
	pub name: T::String,
	/// Parameter type.
	pub ty: T::Type,
}

impl IntoPortable for RuntimeApiMethodParamMetadata {
	type Output = RuntimeApiMethodParamMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		RuntimeApiMethodParamMetadata {
			name: self.name.into_portable(registry),
			ty: registry.register_type(&self.ty),
		}
	}
}

/// Metadata of the extrinsic used by the runtime.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct ExtrinsicMetadata<T: Form = MetaForm> {
	/// Extrinsic version.
	pub version: u8,
	/// The type of the address that signes the extrinsic
	pub address_ty: T::Type,
	/// The type of the outermost Call enum.
	pub call_ty: T::Type,
	/// The type of the extrinsic's signature.
	pub signature_ty: T::Type,
	/// The type of the outermost Extra enum.
	pub extra_ty: T::Type,
	/// The signed extensions in the order they appear in the extrinsic.
	pub signed_extensions: Vec<SignedExtensionMetadata<T>>,
}

impl IntoPortable for ExtrinsicMetadata {
	type Output = ExtrinsicMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		ExtrinsicMetadata {
			version: self.version,
			address_ty: registry.register_type(&self.address_ty),
			call_ty: registry.register_type(&self.call_ty),
			signature_ty: registry.register_type(&self.signature_ty),
			extra_ty: registry.register_type(&self.extra_ty),
			signed_extensions: registry.map_into_portable(self.signed_extensions),
		}
	}
}

/// Metadata of an extrinsic's signed extension.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct SignedExtensionMetadata<T: Form = MetaForm> {
	/// The unique signed extension identifier, which may be different from the type name.
	pub identifier: T::String,
	/// The type of the signed extension, with the data to be included in the extrinsic.
	pub ty: T::Type,
	/// The type of the additional signed data, with the data to be included in the signed payload
	pub additional_signed: T::Type,
}

impl IntoPortable for SignedExtensionMetadata {
	type Output = SignedExtensionMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		SignedExtensionMetadata {
			identifier: self.identifier.into_portable(registry),
			ty: registry.register_type(&self.ty),
			additional_signed: registry.register_type(&self.additional_signed),
		}
	}
}

/// All metadata about an runtime pallet.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct PalletMetadata<T: Form = MetaForm> {
	/// Pallet name.
	pub name: T::String,
	/// Pallet storage metadata.
	pub storage: Option<PalletStorageMetadata<T>>,
	/// Pallet calls metadata.
	pub calls: Option<PalletCallMetadata<T>>,
	/// Pallet event metadata.
	pub event: Option<PalletEventMetadata<T>>,
	/// Pallet constants metadata.
	pub constants: Vec<PalletConstantMetadata<T>>,
	/// Pallet error metadata.
	pub error: Option<PalletErrorMetadata<T>>,
	/// Define the index of the pallet, this index will be used for the encoding of pallet event,
	/// call and origin variants.
	pub index: u8,
	/// Pallet documentation.
	pub docs: Vec<T::String>,
}

impl IntoPortable for PalletMetadata {
	type Output = PalletMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PalletMetadata {
			name: self.name.into_portable(registry),
			storage: self.storage.map(|storage| storage.into_portable(registry)),
			calls: self.calls.map(|calls| calls.into_portable(registry)),
			event: self.event.map(|event| event.into_portable(registry)),
			constants: registry.map_into_portable(self.constants),
			error: self.error.map(|error| error.into_portable(registry)),
			index: self.index,
			docs: registry.map_into_portable(self.docs),
		}
	}
}

/// Metadata for custom types.
///
/// This map associates a string key to a `CustomValueMetadata`.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct CustomMetadata<T: Form = MetaForm> {
	/// The custom map.
	pub map: BTreeMap<T::String, CustomValueMetadata<T>>,
}

impl IntoPortable for CustomMetadata {
	type Output = CustomMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		let map = self
			.map
			.into_iter()
			.map(|(key, value)| (key.into_portable(registry), value.into_portable(registry)))
			.collect();

		CustomMetadata { map }
	}
}

/// The associated value of a custom metadata type.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct CustomValueMetadata<T: Form = MetaForm> {
	/// The custom type.
	pub ty: T::Type,
	/// The custom value of this type.
	pub value: Vec<u8>,
}

impl IntoPortable for CustomValueMetadata {
	type Output = CustomValueMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		CustomValueMetadata {
			ty: registry.register_type(&self.ty),
			value: self.value,
		}
	}
}

/// The type of the outer enums.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct OuterEnums<T: Form = MetaForm> {
	/// The type of the outer `RuntimeCall` enum.
	pub call_enum_ty: T::Type,
	/// The type of the outer `RuntimeEvent` enum.
	pub event_enum_ty: T::Type,
	/// The module error type of the
	/// [`DispatchError::Module`](https://docs.rs/sp-runtime/24.0.0/sp_runtime/enum.DispatchError.html#variant.Module) variant.
	///
	/// The `Module` variant will be 5 scale encoded bytes which are normally decoded into
	/// an `{ index: u8, error: [u8; 4] }` struct. This type ID points to an enum type which instead
	/// interprets the first `index` byte as a pallet variant, and the remaining `error` bytes as the
	/// appropriate `pallet::Error` type. It is an equally valid way to decode the error bytes, and
	/// can be more informative.
	///
	/// # Note
	///
	/// - This type cannot be used directly to decode `sp_runtime::DispatchError` from the
	///   chain. It provides just the information needed to decode `sp_runtime::DispatchError::Module`.
	/// - Decoding the 5 error bytes into this type will not always lead to all of the bytes being consumed;
	///   many error types do not require all of the bytes to represent them fully.
	pub error_enum_ty: T::Type,
}

impl IntoPortable for OuterEnums {
	type Output = OuterEnums<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		OuterEnums {
			call_enum_ty: registry.register_type(&self.call_enum_ty),
			event_enum_ty: registry.register_type(&self.event_enum_ty),
			error_enum_ty: registry.register_type(&self.error_enum_ty),
		}
	}
}
