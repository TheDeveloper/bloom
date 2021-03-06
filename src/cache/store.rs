// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::cmp;
use std::time::Duration;
use r2d2::Pool;
use r2d2::config::Config;
use r2d2_redis::{RedisConnectionManager, Error};
use redis::{self, Connection, Commands};
use futures::future;
use futures::future::FutureResult;

use APP_CONF;

pub struct CacheStoreBuilder;

pub struct CacheStore {
    pool: Pool<RedisConnectionManager>,
}

#[derive(Debug)]
pub enum CacheStoreError {
    Disconnected,
    Failed,
    TooLarge,
}

#[derive(Debug)]
pub enum CachePurgeVariant {
    Bucket,
    Auth,
}

type CacheResult = FutureResult<Option<String>, CacheStoreError>;

impl CacheStoreBuilder {
    pub fn new() -> CacheStore {
        info!("binding to store backend at {}", APP_CONF.redis.inet);

        let tcp_addr_raw =
            format!(
            "redis://{}:{}/{}",
            APP_CONF.redis.inet.ip(),
            APP_CONF.redis.inet.port(),
            APP_CONF.redis.database,
        );

        match RedisConnectionManager::new(tcp_addr_raw.as_ref()) {
            Ok(manager) => {
                let config = Config::<Connection, Error>::builder()
                    .test_on_check_out(false)
                    .pool_size(APP_CONF.redis.pool_size)
                    .idle_timeout(Some(
                        Duration::from_secs(APP_CONF.redis.idle_timeout_seconds),
                    ))
                    .connection_timeout(Duration::from_secs(
                        APP_CONF.redis.connection_timeout_seconds,
                    ))
                    .build();

                match Pool::new(config, manager) {
                    Ok(pool) => {
                        info!("bound to store backend");

                        CacheStore { pool: pool }
                    }
                    Err(_) => panic!("could not spawn redis pool"),
                }
            }
            Err(_) => panic!("could not create redis connection manager"),
        }
    }
}

impl CacheStore {
    pub fn get(&self, key: &str) -> CacheResult {
        get_cache_store_client!(self, client {
            match (*client).get(key) {
                Ok(string) => Ok(Some(string)),
                _ => Err(CacheStoreError::Failed),
            }
        })
    }

    pub fn set(
        &self,
        key: &str,
        value: &str,
        ttl: usize,
        key_bucket: Option<String>,
    ) -> CacheResult {
        get_cache_store_client!(self, client {
            // Cap TTL to 'max_key_expiration'
            let ttl_cap = cmp::min(ttl, APP_CONF.redis.max_key_expiration);

            // Ensure value is not larger than 'max_key_size'
            if value.len() > APP_CONF.redis.max_key_size {
                Err(CacheStoreError::TooLarge)
            } else {
                match key_bucket {
                    Some(key_bucket_value) => {
                        // Bucket (MULTI operation for main data + bucket marker)
                        gen_cache_store_empty_result!(
                            redis::pipe()
                                .atomic()
                                .cmd("SETEX").arg(key).arg(ttl_cap).arg(value).ignore()
                                .cmd("SETEX").arg(key_bucket_value).arg(ttl_cap).arg("").ignore()
                                .query::<()>(&*client)
                        )
                    },
                    None => {
                        gen_cache_store_empty_result!(
                            (*client).set_ex::<_, _, ()>(key, value, ttl_cap)
                        )
                    },
                }
            }
        })
    }

    pub fn purge_pattern(&self, variant: &CachePurgeVariant, key_pattern: &str) -> CacheResult {
        get_cache_store_client!(self, client {
            // Invoke keyspace cleanup script for key pattern
            gen_cache_store_empty_result!(
                redis::Script::new(variant.get_script())
                    .arg(key_pattern)
                    .invoke::<()>(&*client)
            )
        })
    }
}

impl CachePurgeVariant {
    fn get_script(&self) -> &'static str {
        match *self {
            CachePurgeVariant::Bucket => {
                r#"
                    local re = '^(.+):b:.+$'
                    local targets = {}

                    for _, bucket_key in pairs(redis.call('KEYS', ARGV[1])) do
                        local base_key = bucket_key:match(re)

                        if base_key then
                            table.insert(targets, base_key)
                        end

                        table.insert(targets, bucket_key)
                    end

                    if next(targets) then
                        redis.call('DEL', unpack(targets))
                    end
                "#
            }
            CachePurgeVariant::Auth => {
                r#"
                    local targets = redis.call('KEYS', ARGV[1])

                    if next(targets) then
                        redis.call('DEL', unpack(targets))
                    end
                "#
            }
        }
    }
}
