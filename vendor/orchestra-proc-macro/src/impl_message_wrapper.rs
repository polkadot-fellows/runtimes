// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashSet;

use itertools::Itertools;
use quote::quote;
use syn::{spanned::Spanned, Result};

use super::*;

/// Generates the wrapper type enum.
pub(crate) fn impl_message_wrapper_enum(info: &OrchestraInfo) -> Result<proc_macro2::TokenStream> {
	let consumes = info.any_message();
	let consumes_variant = info.variant_names();

	let outgoing = &info.outgoing_ty;

	let message_wrapper = &info.message_wrapper;

	let (outgoing_from_impl, outgoing_decl) = if let Some(outgoing) = outgoing {
		let outgoing_variant = outgoing.get_ident().ok_or_else(|| {
			syn::Error::new(
				outgoing.span(),
				"Missing identifier to use as enum variant for outgoing.",
			)
		})?;
		(
			quote! {
				impl ::std::convert::From< #outgoing > for #message_wrapper {
					fn from(message: #outgoing) -> Self {
						#message_wrapper :: #outgoing_variant ( message )
					}
				}
			},
			quote! {
				#outgoing_variant ( #outgoing ) ,
			},
		)
	} else {
		(TokenStream::new(), TokenStream::new())
	};

	let mut ts = quote! {
		/// Generated message type wrapper over all possible messages
		/// used by any subsystem.
		#[allow(missing_docs)]
		#[derive(Debug)]
		pub enum #message_wrapper {
			#(
				#consumes_variant ( #consumes ),
			)*
			#outgoing_decl
			// dummy message type
			Empty,
		}

		impl ::std::convert::From< () > for #message_wrapper {
			fn from(_: ()) -> Self {
				#message_wrapper :: Empty
			}
		}

		#(
			impl ::std::convert::From< #consumes > for #message_wrapper {
				fn from(message: #consumes) -> Self {
					#message_wrapper :: #consumes_variant ( message )
				}
			}
		)*

		#outgoing_from_impl
	};

	// TODO it's not perfect, if the same type is used with different paths
	// the detection will fail
	let outgoing = HashSet::<&Path>::from_iter(
		info.subsystems().iter().map(|ssf| ssf.messages_to_send.iter()).flatten(),
	);
	let incoming = HashSet::<&Path>::from_iter(
		info.subsystems().iter().filter_map(|ssf| ssf.message_to_consume.as_ref()),
	);

	// Try to maintain the ordering according to the span start in the declaration.
	fn cmp<'p, 'q>(a: &'p &&Path, b: &'q &&Path) -> std::cmp::Ordering {
		a.span()
			.start()
			.partial_cmp(&b.span().start())
			.unwrap_or(std::cmp::Ordering::Equal)
	}

	// sent but not received
	if cfg!(feature = "deny_unconsumed_messages") {
		for sbnr in outgoing.difference(&incoming).sorted_by(cmp) {
			ts.extend(
				syn::Error::new(
					sbnr.span(),
					format!(
						"Message `{}` is sent but never received",
						sbnr.get_ident()
							.expect("Message is a path that must end in an identifier. qed")
					),
				)
				.to_compile_error(),
			);
		}
	}

	// received but not sent

	if cfg!(feature = "deny_unsent_messages") {
		for rbns in incoming.difference(&outgoing).sorted_by(cmp) {
			ts.extend(
				syn::Error::new(
					rbns.span(),
					format!(
						"Message `{}` is received but never sent",
						rbns.get_ident()
							.expect("Message is a path that must end in an identifier. qed")
					),
				)
				.to_compile_error(),
			);
		}
	}
	Ok(ts)
}
