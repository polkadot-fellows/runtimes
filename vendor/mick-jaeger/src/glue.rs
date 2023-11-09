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

// Copied from https://github.com/open-telemetry/opentelemetry-rust/tree/master/opentelemetry-jaeger/src/transport

use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) struct TNoopChannel;

impl io::Read for TNoopChannel {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TBufferChannel {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl TBufferChannel {
    pub fn with_capacity(capacity: usize) -> Self {
        TBufferChannel {
            inner: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
        }
    }

    pub fn take_bytes(&mut self) -> Vec<u8> {
        self.inner
            .lock()
            .map(|mut write| write.split_off(0))
            .unwrap_or_default()
    }
}

impl io::Read for TBufferChannel {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        unreachable!("jaeger protocol never reads")
    }
}

impl io::Write for TBufferChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(mut inner) = self.inner.lock() {
            inner.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl thrift::transport::TIoChannel for TBufferChannel {
    fn split(
        self,
    ) -> thrift::Result<(
        thrift::transport::ReadHalf<Self>,
        thrift::transport::WriteHalf<Self>,
    )>
    where
        Self: Sized,
    {
        Ok((
            thrift::transport::ReadHalf::new(self.clone()),
            thrift::transport::WriteHalf::new(self),
        ))
    }
}
