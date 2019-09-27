///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life>
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
//! This module provides macro for build message players and join it in one future

#![feature(async_await)]
#![feature(async_closure)]
#![feature(core_intrinsics)]
#![feature(trace_macros)]
#![feature(proc_macro_quote)]

extern crate proc_macro;

use quote::quote;
use proc_macro::{TokenStream};

#[proc_macro]
pub fn players_builder(input: TokenStream) -> TokenStream {

    let mut messages = Vec::new();
    let mut next_item = String::new();
    for item in input {
        match item.to_string().as_str() {
            "," => {
                messages.push(next_item);
                next_item = String::new();
            }
            s => next_item += s,
        }
    }

    if next_item != "" {
        messages.push(next_item);
    }

    let message_refs = messages.iter().map(String::as_str).collect::<Vec<&str>>();

    let mut message_pairs = Vec::<(&str, &str)>::new();
    for message in message_refs {
        message_pairs.push(string_into_pair(message));
    };

    let players_by_type = message_pairs.iter().map( |pair| {
        let (p, n) = *pair;
        let ps = p.clone().to_string();
        let ns = n.clone().to_string();

        let type_name = format!("players_{}_{}", ps.clone(), ns.clone()).to_owned();
        let ident: syn::Ident = syn::parse_str(type_name.as_str()).unwrap();
        quote! {
            let mut #ident = vec![];
        }
    });
    let q_players_by_type = quote!{
        #( #players_by_type )*
    };


    let map_futures_vec = message_pairs.iter().map( |pair| {
        let (p, n) = *pair;
        let ps = p.clone().to_string();
        let ns = n.clone().to_string();

        let type_name = format!("players_{}_{}", ps.clone(), ns.clone()).to_owned();
        let ident: syn::Ident = syn::parse_str(type_name.as_str()).unwrap();

        let joined_type_name = format!("j_players_{}_{}", ps.clone(), ns.clone()).to_owned();
        let j_ident: syn::Ident = syn::parse_str(joined_type_name.as_str()).unwrap();

        quote! {
            let #j_ident = future::join_all(#ident).map(|_| ());
        }
    });
    let q_map_futures_vec = quote!{
        #( #map_futures_vec )*
    };

    let match_arms = message_pairs.iter().map( |pair| {

        let (p, n) = *pair;
        let ps = p.clone().to_string();
        let ns = n.clone().to_string();

        let sl = format!("{}/{}", ps.clone(), ns.clone()).to_string();
        let pnql = quote! {
            #sl
        };

        let type_name = format!("players_{}_{}", ps.clone(), ns.clone()).to_string();
        let ident: syn::Ident = syn::parse_str(type_name.as_str()).unwrap();

        let sr = format!("RosbagPlayer<{}::{}>", ps.clone(), ns.clone()).to_string();
        let ptype: syn::Type = syn::parse_str(sr.as_str()).unwrap();

        quote! {
          #pnql => #ident.push(<#ptype>::new(storage_topic_name, topics_ids.clone(), bag.clone(), start_msg_timestamp.clone()).play())
        }
    });

    let q_match_arms = quote!{
        #( #match_arms, )*
    };

    let vecs_joining_initial_quote = quote!{
        robonomics_player_codegen_nop()
    };
    let join_all_players_vecs_quote = message_pairs.iter().fold(
        vecs_joining_initial_quote, |gf, pair| {
            let (p, n) = *pair;
            let ps = p.clone().to_string();
            let ns = n.clone().to_string();

            let joined_type_name = format!("j_players_{}_{}", ps.clone(), ns.clone()).to_owned();
            let j_ident: syn::Ident = syn::parse_str(joined_type_name.as_str()).unwrap();
            quote!{
                future::join( #gf, #j_ident).map(|_| ())
            }
        }
    );

    let global = quote! {
        fn players_builder(bag: Arc<RosBag>) -> impl Future<Output=()> {
            //needed as initial impl Future<Output=()> for player's Future joining
            async fn robonomics_player_codegen_nop() {
            }

            let records = bag.records();

            type StorageTopics = HashMap<(String, String), Vec<u32>>;
            type PlayerConnections<'a> = HashMap<u32, Connection<'a>>;

            let mut player_connections = PlayerConnections::new();
            let mut storage_topics = StorageTopics::new();

            let mut start_msg_timestamp: u64 = 0;

            records.for_each(|record|
                match record {
                    Ok(Record::Connection(conn)) => {
                        let conn01 = conn.clone();
                        player_connections.insert(conn01.id.clone(), conn01.clone());

                        match storage_topics.get_mut(&(conn01.storage_topic.to_string(), conn01.tp.to_string())) {
                            Some(ids) => ids.push(conn01.id.clone()),
                            None => ({
                                let ids= vec![conn01.id.clone()];
                                storage_topics.insert((conn01.storage_topic.to_string(), conn01.tp.to_string()), ids);
                            }),
                        }
                    },

                    Ok(Record::MessageData(msg_data)) => {
                        if start_msg_timestamp == 0 || msg_data.time < start_msg_timestamp {
                            start_msg_timestamp = msg_data.time;
                        }
                    },
                    _ => ()
                }
            );
            #q_players_by_type
            for ((storage_topic_name, topic_type), topics_ids) in storage_topics.iter() {
                match topic_type.as_str() {
                    #q_match_arms
                    _ => log::warn!("PLAYERS GENERATION unsupported topic type {}", topic_type.as_str()),
                }
            }
            #q_map_futures_vec
            #join_all_players_vecs_quote
        }
    };

    TokenStream::from(global)
}

fn string_into_pair(input: &str) -> (&str, &str) {
    let mut parts = input.splitn(2, '/');
    let package = match parts.next() {
        Some(v) => v,
        None => ""
    };
    let name = match parts.next() {
        Some(v) => v,
        None => ""
    };
    (package, name)
}
