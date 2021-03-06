// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::str::SplitWhitespace;
use futures::Future;

use super::handle::ControlShard;
use APP_CACHE_STORE;
use cache::route::CacheRoute;
use cache::store::CachePurgeVariant;

#[derive(PartialEq)]
pub enum ControlCommandResponse {
    Void,
    Nil,
    Ok,
    Pong,
    Ended,
    Err,
}

pub struct ControlCommand;

pub const COMMAND_SIZE: usize = 6;

type ControlResult = Result<ControlCommandResponse, Option<()>>;

impl ControlCommandResponse {
    pub fn to_str(&self) -> &'static str {
        match *self {
            ControlCommandResponse::Void => "",
            ControlCommandResponse::Nil => "NIL",
            ControlCommandResponse::Ok => "OK",
            ControlCommandResponse::Pong => "PONG",
            ControlCommandResponse::Ended => "ENDED quit",
            ControlCommandResponse::Err => "ERR",
        }
    }
}

impl ControlCommand {
    pub fn dispatch_flush_bucket(
        shard: &ControlShard,
        mut parts: SplitWhitespace,
    ) -> ControlResult {
        let bucket = parts.next().unwrap_or("");

        if bucket.is_empty() == false {
            let pattern = CacheRoute::gen_key_bucket_with_ns(
                &CacheRoute::gen_ns_from_hash(*shard, "*", "*"),
                bucket,
            );

            return Self::proceed_flush(CachePurgeVariant::Bucket, pattern.as_ref());
        }

        Err(None)
    }

    pub fn dispatch_flush_auth(shard: &ControlShard, mut parts: SplitWhitespace) -> ControlResult {
        let auth = parts.next().unwrap_or("");

        if auth.is_empty() == false {
            let pattern = CacheRoute::gen_ns_from_hash(*shard, auth, "*");

            return Self::proceed_flush(CachePurgeVariant::Auth, pattern.as_ref());
        }

        Err(None)
    }

    pub fn dispatch_ping() -> ControlResult {
        Ok(ControlCommandResponse::Pong)
    }

    pub fn dispatch_shard(shard: &mut ControlShard, mut parts: SplitWhitespace) -> ControlResult {
        match parts.next().unwrap_or("").parse::<u8>() {
            Ok(shard_to) => {
                *shard = shard_to;

                Ok(ControlCommandResponse::Ok)
            }
            _ => Err(None),
        }
    }

    pub fn dispatch_quit() -> ControlResult {
        Ok(ControlCommandResponse::Ended)
    }

    fn proceed_flush(variant: CachePurgeVariant, pattern: &str) -> ControlResult {
        debug!("attempting to flush {:?} for pattern: {}", variant, pattern);

        match APP_CACHE_STORE.purge_pattern(&variant, pattern).wait() {
            Ok(_) => {
                info!("flushed {:?} for pattern: {}", variant, pattern);

                Ok(ControlCommandResponse::Ok)
            }
            Err(err) => {
                warn!(
                    "could not flush {:?} for pattern: {} because: {:?}",
                    variant,
                    pattern,
                    err
                );

                Err(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_matches_command_response_string() {
        assert_eq!(ControlCommandResponse::Nil.to_str(), "NIL");
        assert_eq!(ControlCommandResponse::Ok.to_str(), "OK");
        assert_eq!(ControlCommandResponse::Pong.to_str(), "PONG");
        assert_eq!(ControlCommandResponse::Ended.to_str(), "ENDED quit");
        assert_eq!(ControlCommandResponse::Err.to_str(), "ERR");
    }
}
