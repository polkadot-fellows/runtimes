// Copyright (C) 2020 Pierre Krieger
// SPDX-License-Identifier: Apache-2.0
//
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

//! Jaeger client.
//!
//! # Overview
//!
//! In order to use this crate, you must be familiar with the concept of a *span*.
//!
//! A *span* covers a certain period of time, typically from the start of an operation to the end.
//! In other words, you generally start a span at the beginning of a function or block, and end
//! it at the end of the function/block.
//!
//! The purpose of this crate is to let you easily record spans and send them to a Jaeger server,
//! which will aggerate them and let you visualize them.
//!
//! Each span belongs to a *trace*. A trace is identified by a 128 bits identifier. Jaeger lets
//! you easily visualize all the spans belonging to the same trace, even if they come from
//! different clients.
//!
//! As an example, imagine an HTTP frontend server receiving an HTTP request. It can generate a
//! new trace id for this request, then pass this identifier around to other external processes
//! that process parts of this request. These external processes, being all connected to the same
//! Jaeger server, can report spans corresponding to this request.
//!
//! The easiest way to start a Jaeger server for quick experimentation is through Docker:
//!
//! ```notrust
//! docker run -d --name jaeger \
//!   -e COLLECTOR_ZIPKIN_HTTP_PORT=9411 \
//!   -p 5775:5775/udp \
//!   -p 6831:6831/udp \
//!   -p 6832:6832/udp \
//!   -p 5778:5778 \
//!   -p 16686:16686 \
//!   -p 14268:14268 \
//!   -p 14250:14250 \
//!   -p 9411:9411 \
//!   jaegertracing/all-in-one:1.20
//! ```
//!
//! See also [the official documentation](https://www.jaegertracing.io/docs/1.20/getting-started/).
//!
//! # Usage: initialization
//!
//! First and foremost, call [`init`] in order to allocate all the necessary objects.
//!
//! This returns a combination of a [`TracesIn`] and [`TracesOut`]. Think of them as a sender and
//! receiver. The [`TracesIn`] is used in order to send completed spans to the [`TracesOut`].
//!
//! Sending the traces to the server isn't covered by this library. The [`TracesOut`] must be
//! polled using [`TracesOut::next`], and the data sent through UDP to the Jaeger server.
//!
//! ```
//! # async fn foo() {
//! let (traces_in, mut traces_out) = mick_jaeger::init(mick_jaeger::Config {
//!     service_name: "demo".to_string(),
//! });
//!
//! let udp_socket = async_std::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
//! udp_socket.connect("127.0.0.1:6831").await.unwrap();
//!
//! async_std::task::spawn(async move {
//!     loop {
//!         let buf = traces_out.next().await;
//!         udp_socket.send(&buf).await.unwrap();
//!     }
//! });
//! # }
//! ```
//!
//! If [`TracesOut::next`] isn't called often enough, in other words if the background task is too
//! slow, the spans sent on the [`TracesIn`] will be automatically and silently discarded. This
//! isn't expected to happen under normal circumstances.
//!
//! # Usage: spans
//!
//! Use the [`TracesIn::span`] method to create spans.
//!
//! The basic way to use this library is to use [`TracesIn::span`]. This creates a [`Span`] object
//! that, when destroyed, will send a report destined to the [`TracesOut`].
//!
//! > **Note**: As long as a [`Span`] is alive, it will not be visible on the Jaeger server. You
//! >           are encouraged to create short-lived spans and long-lived trace IDs.
//!
//! ```
//! # use std::num::NonZeroU128;
//! # let mut traces_in: std::sync::Arc<mick_jaeger::TracesIn> = return;
//! let _span = traces_in.span(NonZeroU128::new(43).unwrap(), "something");
//!
//! // do something
//!
//! // The span is reported when it is destroyed at the end of the scope.
//! ```
//!
//! > **Note**: Do not name your spans `_`, otherwise they will be destroyed immediately!
//!
//! It is possible, and encouraged, to add tags to spans.
//!
//! ```
//! # use std::num::NonZeroU128;
//! # let mut traces_in: std::sync::Arc<mick_jaeger::TracesIn> = return;
//! let mut _span = traces_in.span(NonZeroU128::new(43).unwrap(), "something");
//! _span.add_string_tag("key", "value");
//! ```
//!
//! Spans can have children:
//!
//! ```
//! # use std::num::NonZeroU128;
//! fn my_function(traces_in: &std::sync::Arc<mick_jaeger::TracesIn>) {
//!     let mut _span = traces_in.span(NonZeroU128::new(43).unwrap(), "foo");
//!
//!     // do something
//!
//!     {
//!         let mut _span = _span.child("bar");
//!         // something expensive
//!     }
//! }
//! ```
//!
//! If an event happens at a precise point in time rather than over time, logs can also be added.
//!
//! ```
//! # use std::num::NonZeroU128;
//! # let mut traces_in: std::sync::Arc<mick_jaeger::TracesIn> = return;
//! let mut _span = traces_in.span(NonZeroU128::new(43).unwrap(), "something");
//! _span.log().with_string("key", "value");
//! ```
//!
//! # Differences with other crates
//!
//! While there exists other crates that let you interface with *Jaeger*, they are all
//! overcomplicated according to the author of `mick_jaeger`. Some are lossy abstractions: by
//! trying to be easy to use, they hide important details (such as the trace ID), which causes
//! more confusion than it helps.
//!
//! `mick_jaeger` tries to be simple. The fact that it doesn't handle sending to the server
//! removes a lot of opinionated decisions concerning networking libraries and threading.
//!
//! `mick_jaeger` could theoretically be `no_std`-compatible (after a few tweaks), but can't
//! because at the time of writing there is no no-std-compatible library for the *thrift*
//! protocol.
//!

use futures::{channel::mpsc, prelude::*, stream::FusedStream as _};
use protocol::agent::TAgentSyncClient as _;
use std::{
    convert::TryFrom as _,
    mem,
    num::{NonZeroU128, NonZeroU64},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use thrift::transport::TIoChannel as _;

mod glue;
mod protocol;

/// Configuration to pass to [`init`].
pub struct Config {
    /// Name of the service. Reported to the Jaeger server.
    pub service_name: String,
}

pub fn init(config: Config) -> (Arc<TracesIn>, TracesOut) {
    let (tx, rx) = mpsc::channel(256);
    let (buffer, write) = glue::TBufferChannel::with_capacity(512).split().unwrap();
    let client = protocol::agent::AgentSyncClient::new(
        thrift::protocol::TCompactInputProtocol::new(glue::TNoopChannel),
        thrift::protocol::TCompactOutputProtocol::new(write),
    );
    let traces_out = TracesOut {
        rx: rx.ready_chunks(64),
        process: protocol::jaeger::Process {
            service_name: config.service_name,
            tags: Some(vec![]),
        },
        buffer,
        client,
    };
    let traces_in = TracesIn {
        sender: Mutex::new(tx),
    };
    (Arc::new(traces_in), traces_out)
}

pub struct TracesIn {
    sender: Mutex<mpsc::Sender<protocol::jaeger::Span>>,
}

impl TracesIn {
    /// Builds a new [`Span`].
    ///
    /// Must be passed a `trace_id` that is used to group spans together. Its meaning is
    /// arbitrary.
    pub fn span(
        self: &Arc<Self>,
        trace_id: NonZeroU128,
        operation_name: impl Into<String>,
    ) -> Span {
        self.span_with_id_and_parent(
            trace_id,
            NonZeroU64::new(rand::random()).unwrap(),
            None,
            operation_name,
        )
    }

    /// Builds a new [`Span`], using a specific span ID.
    ///
    /// Use this method when it is required to know the ID of a span, for example when building
    /// links between spans across different services.
    pub fn span_with_id(
        self: &Arc<Self>,
        trace_id: NonZeroU128,
        span_id: NonZeroU64,
        operation_name: impl Into<String>,
    ) -> Span {
        self.span_with_id_and_parent(trace_id, span_id, None, operation_name)
    }

    /// Builds a new [`Span`], whose parent uses a specific span ID.
    ///
    /// A `parent_id` equal to 0 means "no parent".
    pub fn span_with_parent(
        self: &Arc<Self>,
        trace_id: NonZeroU128,
        parent_id: Option<NonZeroU64>,
        operation_name: impl Into<String>,
    ) -> Span {
        self.span_with_id_and_parent(
            trace_id,
            NonZeroU64::new(rand::random()).unwrap(),
            parent_id,
            operation_name,
        )
    }

    /// Builds a new [`Span`], with a specific ID whose parent uses a specific span ID.
    ///
    /// A `parent_id` equal to 0 means "no parent".
    pub fn span_with_id_and_parent(
        self: &Arc<Self>,
        trace_id: NonZeroU128,
        span_id: NonZeroU64,
        parent_id: Option<NonZeroU64>,
        operation_name: impl Into<String>,
    ) -> Span {
        Span {
            traces_in: self.clone(),
            trace_id,
            span_id,
            parent_span_id: parent_id.map(|id| id.get()).unwrap_or(0),
            operation_name: operation_name.into(),
            references: Vec::new(),
            start_time: SystemTime::now(),
            tags: base_tags(),
            logs: Vec::new(),
        }
    }
}

pub struct Span {
    traces_in: Arc<TracesIn>,
    trace_id: NonZeroU128,
    span_id: NonZeroU64,
    /// [`Span::span_id`] of the parent, or `0` if no parent.
    parent_span_id: u64,
    operation_name: String,
    references: Vec<protocol::jaeger::SpanRef>,
    start_time: SystemTime,
    tags: Vec<protocol::jaeger::Tag>,
    logs: Vec<protocol::jaeger::Log>,
}

impl Span {
    /// Creates a new [`Span`], child of this one.
    ///
    /// > **Note**: There is no need to keep the parent [`Span`] alive while the children is
    /// >           alive. The protocol allows for parents that don't completely overlap their
    /// >           children.
    pub fn child(&self, operation_name: impl Into<String>) -> Span {
        self.child_with_id(NonZeroU64::new(rand::random()).unwrap(), operation_name)
    }

    /// Creates a new [`Span`], child of this one, with a specific ID.
    pub fn child_with_id(&self, span_id: NonZeroU64, operation_name: impl Into<String>) -> Span {
        Span {
            traces_in: self.traces_in.clone(),
            trace_id: self.trace_id,
            span_id,
            parent_span_id: self.span_id.get(),
            operation_name: operation_name.into(),
            references: Vec::new(),
            start_time: SystemTime::now(),
            tags: base_tags(),
            logs: Vec::new(),
        }
    }

    /// Returns the trace ID originally passed when building this span.
    pub fn trace_id(&self) -> NonZeroU128 {
        self.trace_id
    }

    /// Returns the span ID originally passed when building this span.
    pub fn span_id(&self) -> NonZeroU64 {
        self.span_id
    }

    /// Add a log entry to this span.
    pub fn log(&mut self) -> Log {
        let timestamp = i64::try_from(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::new(0, 0))
                .as_micros(),
        )
        .unwrap_or(i64::max_value());

        Log {
            span: self,
            timestamp,
            fields: Vec::new(),
        }
    }

    /// Adds a "followfrom" relation ship towards another span of (potentially) another trace.
    pub fn add_follows_from(&mut self, other: &Span) {
        self.add_follows_from_raw(other.trace_id(), other.span_id())
    }

    /// Adds a "followfrom" relation ship towards another span of (potentially) another trace.
    pub fn add_follows_from_raw(&mut self, trace_id: NonZeroU128, span_id: NonZeroU64) {
        self.references.push(protocol::jaeger::SpanRef {
            ref_type: protocol::jaeger::SpanRefType::FollowsFrom,
            trace_id_low: i64::from_be_bytes(
                <[u8; 8]>::try_from(&trace_id.get().to_be_bytes()[8..]).unwrap(),
            ),
            trace_id_high: i64::from_be_bytes(
                <[u8; 8]>::try_from(&trace_id.get().to_be_bytes()[..8]).unwrap(),
            ),
            span_id: i64::from_ne_bytes(span_id.get().to_ne_bytes()),
        });
    }

    /// Add a new key-value tag to this span.
    pub fn add_string_tag(&mut self, key: &str, value: &str) {
        // TODO: check for duplicates?
        self.tags.push(string_tag(key, value));
    }

    /// Add a new key-value tag to this span.
    pub fn add_int_tag(&mut self, key: &str, value: i64) {
        // TODO: check for duplicates?
        self.tags.push(int_tag(key, value));
    }

    /// Modifies the start time of this span.
    ///
    /// > **Note**: This method can be useful in order to generate a span with a `trace_id` that
    /// >           is only know after the span should have started. To do so, call
    /// >           [`StartTime::now`] when the span should start, create the span once you know
    /// >           the ̀`trace_id`, then call this method.
    pub fn override_start_time(&mut self, start_time: StartTime) {
        self.start_time = start_time.0;
    }

    /// Modifies the start time of this span.
    ///
    /// > **Note**: This method can be useful in order to generate a span with a `trace_id` that
    /// >           is only know after the span should have started. To do so, call
    /// >           [`StartTime::now`] when the span should start, create the span once you know
    /// >           the ̀`trace_id`, then call this method.
    pub fn with_start_time_override(mut self, start_time: StartTime) -> Self {
        self.override_start_time(start_time);
        self
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let end_time = SystemTime::now();

        // Try to send the span, but don't try too hard. If the channel is full, drop the tracing
        // information.
        let _ = self
            .traces_in
            .sender
            .lock()
            .unwrap()
            .try_send(protocol::jaeger::Span {
                trace_id_low: i64::from_be_bytes(
                    <[u8; 8]>::try_from(&self.trace_id.get().to_be_bytes()[8..]).unwrap(),
                ),
                trace_id_high: i64::from_be_bytes(
                    <[u8; 8]>::try_from(&self.trace_id.get().to_be_bytes()[..8]).unwrap(),
                ),
                span_id: i64::from_ne_bytes(self.span_id.get().to_ne_bytes()),
                parent_span_id: i64::from_ne_bytes(self.parent_span_id.to_ne_bytes()),
                operation_name: mem::take(&mut self.operation_name),
                references: if self.references.is_empty() {
                    None
                } else {
                    Some(mem::take(&mut self.references))
                },
                flags: 0,
                start_time: i64::try_from(
                    self.start_time
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_else(|_| Duration::new(0, 0))
                        .as_micros(),
                )
                .unwrap_or(i64::max_value()),
                duration: i64::try_from(
                    end_time
                        .duration_since(self.start_time)
                        .unwrap_or_else(|_| Duration::new(0, 0))
                        .as_micros(),
                )
                .unwrap_or(i64::max_value()),
                tags: Some(mem::take(&mut self.tags)),
                logs: if self.logs.is_empty() {
                    None
                } else {
                    Some(mem::take(&mut self.logs))
                },
            });
    }
}

pub struct StartTime(SystemTime);

impl StartTime {
    pub fn now() -> Self {
        StartTime(SystemTime::now())
    }
}

pub struct Log<'a> {
    span: &'a mut Span,
    timestamp: i64,
    fields: Vec<protocol::jaeger::Tag>,
}

impl<'a> Log<'a> {
    /// Add a new key-value tag to this log.
    pub fn with_string(mut self, key: &str, value: &str) -> Self {
        self.fields.push(string_tag(key, value));
        self
    }

    /// Add a new key-value tag to this log.
    pub fn with_int(mut self, key: &str, value: i64) -> Self {
        self.fields.push(int_tag(key, value));
        self
    }

    // TODO: other methods
}

impl<'a> Drop for Log<'a> {
    fn drop(&mut self) {
        self.span.logs.push(protocol::jaeger::Log {
            timestamp: self.timestamp,
            fields: mem::replace(&mut self.fields, Vec::new()),
        });
    }
}

fn int_tag(key: &str, value: i64) -> protocol::jaeger::Tag {
    protocol::jaeger::Tag {
        key: key.to_string(),
        v_type: protocol::jaeger::TagType::Long,
        v_long: Some(value),
        v_str: None,
        v_double: None,
        v_bool: None,
        v_binary: None,
    }
}

fn string_tag(key: &str, value: &str) -> protocol::jaeger::Tag {
    protocol::jaeger::Tag {
        key: key.to_string(),
        v_type: protocol::jaeger::TagType::String,
        v_str: Some(value.to_string()),
        v_long: None,
        v_double: None,
        v_bool: None,
        v_binary: None,
    }
}

fn base_tags() -> Vec<protocol::jaeger::Tag> {
    vec![
        string_tag("otel.library.name", env!("CARGO_PKG_NAME")),
        string_tag("otel.library.version", env!("CARGO_PKG_VERSION")),
    ]
}

/// Receiving side for spans.
///
/// This object must be processed in order to send traces to the UDP server.
pub struct TracesOut {
    rx: stream::ReadyChunks<mpsc::Receiver<protocol::jaeger::Span>>,
    process: protocol::jaeger::Process,
    buffer: thrift::transport::ReadHalf<glue::TBufferChannel>,
    client: protocol::agent::AgentSyncClient<
        thrift::protocol::TCompactInputProtocol<glue::TNoopChannel>,
        thrift::protocol::TCompactOutputProtocol<
            thrift::transport::WriteHalf<glue::TBufferChannel>,
        >,
    >,
}

impl TracesOut {
    /// Returns the next packet of data to send on the UDP socket.
    pub async fn next(&mut self) -> Vec<u8> {
        if self.rx.is_terminated() {
            loop {
                futures::pending!()
            }
        }

        let spans = self.rx.select_next_some().await;

        self.client
            .emit_batch(protocol::jaeger::Batch {
                spans,
                process: self.process.clone(),
            })
            .unwrap();
        self.buffer.take_bytes()
    }

    /// Add a new key-value tag to the process.
    pub fn add_string_tag(&mut self, key: &str, value: &str) {
        // TODO: check for duplicates?
        self.process
            .tags
            .as_mut()
            .unwrap()
            .push(string_tag(key, value));
    }

    /// Add a new key-value tag to the process.
    pub fn add_int_tag(&mut self, key: &str, value: i64) {
        // TODO: check for duplicates?
        self.process
            .tags
            .as_mut()
            .unwrap()
            .push(int_tag(key, value));
    }
}
