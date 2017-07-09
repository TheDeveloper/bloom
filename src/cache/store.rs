// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::cmp;

use bmemcached::MemcachedClient;

use ::APP_CONF;

pub struct CacheStoreBuilder;

pub struct CacheStore {
    client: Option<MemcachedClient>
}

type CacheResult = Result<Option<String>, &'static str>;

impl CacheStoreBuilder {
    pub fn new() -> CacheStore {
        info!("binding to store backend at {}", APP_CONF.memcached.inet);

        let tcp_addr = format!("{}:{}", APP_CONF.memcached.inet.ip(),
                            APP_CONF.memcached.inet.port());

        match MemcachedClient::new(
            vec![tcp_addr], APP_CONF.memcached.pool_size) {
            Ok(client_raw) => {
                info!("bound to store backend");

                CacheStore {
                    client: Some(client_raw)
                }
            }
            Err(err) => panic!("could not connect to memcached")
        }
    }
}

impl CacheStore {
    pub fn get(&self, key: &str) -> CacheResult {
        match self.client {
            Some(ref client) => {
                match client.get(key) {
                    Ok(String) => Ok(Some(String)),
                    _ => Err("failed")
                }
            }
            _ => {
                Err("disconnected")
            }
        }
    }

    pub fn set(&self, key: &str, value: &str, ttl: u32) -> CacheResult {
        match self.client {
            Some(ref client) => {
                // Cap TTL to 'max_key_expiration'
                let ttl_cap = cmp::min(ttl,
                                APP_CONF.memcached.max_key_expiration);

                // Ensure value is not larger than 'max_key_size'
                if value.len() > APP_CONF.memcached.max_key_size {
                    return Err("too large")
                }

                match client.set(key, value, ttl_cap) {
                    Ok(_) => Ok(None),
                    _ => Err("failed")
                }
            }
            _ => {
                Err("disconnected")
            }
        }
    }

    pub fn purge(&self, key: &str) -> CacheResult {
        match self.client {
            Some(ref client) => {
                match client.delete(key) {
                    Ok(_) => Ok(None),
                    _ => Err("failed")
                }
            }
            _ => {
                Err("disconnected")
            }
        }
    }
}
