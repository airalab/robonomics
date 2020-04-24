///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! Stream based pipes.

use futures::{future, Stream, Future, StreamExt};
use std::pin::Pin;

/// Common stream type for pipes.
pub type PipeStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

/// Result of transition function.
pub type PipeFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Pipe joins two streams.
pub trait Pipe<'a, A: 'a, B: 'a>: Sized + Send + 'a {
    /// Asynchronous transition function with state.
    fn exec(&mut self, input: A) -> PipeFuture<'a, B>;

    /// Launch stream processing.
    /// Note: it could be launch only once.
    fn pipe(mut self, input: PipeStream<'a, A>) -> PipeStream<'a, B> {
        input.then(move |v| self.exec(v)).boxed()
    }
}

pub trait Consumer<'a, T: 'a>: Pipe<'a, T, ()> {
    fn consume(self, input: PipeStream<'a, T>) -> PipeFuture<'a, ()> {
        Box::pin(self.pipe(input).for_each(|_| future::ready(())))
    }
}
