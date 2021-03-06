Bloom
=====

[![Build Status](https://travis-ci.org/valeriansaliou/bloom.svg?branch=master)](https://travis-ci.org/valeriansaliou/bloom)

**Bloom is a REST API caching middleware, acting as a reverse proxy between your load balancers and your REST API workers.**

It is completely agnostic of your API implementation, and requires minimal changes to your existing API code to work.

Bloom relies on `redis`, [configured as a LRU cache](https://github.com/valeriansaliou/bloom/blob/master/examples/config/redis.conf) to store cached data. It is built in Rust and focuses on performance and low resource usage.

**Important: Bloom works great if your API implements REST conventions. Your API needs to use HTTP read methods, namely `GET`, `HEAD` solely as read methods (do not use HTTP GET parameters as a way to update data).**

:newspaper: The Bloom project was initially announced in [a post on my personal journal](https://journal.valeriansaliou.name/announcing-bloom-a-rest-api-caching-middleware/).

![Bloom](https://valeriansaliou.github.io/bloom/images/bloom.jpg)

## Who uses it?

<table>
<tr>
<td align="center"><a href="https://crisp.chat/"><img src="https://valeriansaliou.github.io/bloom/images/crisp.png" height="64" /></a></td>
</tr>
<tr>
<td align="center">Crisp</td>
</tr>
</table>

_👋 You use Bloom and you want to be listed there? [Contact me](https://valeriansaliou.name/)._

## Features

* **The same Bloom server can be used for different API workers at once**, using HTTP header `Bloom-Request-Shard` (eg. Main API uses shard `0`, Search API uses shard `1`)
* **Cache stored on buckets**, specified in your REST API responses using HTTP header `Bloom-Response-Bucket`.
* **Cache clustered by authentication token**, no cache leak across users is possible, using the standard `Authorization` HTTP header.
* **Cache can be expired directly from your REST API workers**, via a control channel.
* **Configurable per-request caching strategy**, using `Bloom-Request-*` HTTP headers in the requests your Load Balancers forward to Bloom.
  * Specify caching shard for an API system with `Bloom-Request-Shard` (default shard is `0`, maximum value is `255`).
* **Configurable per-response caching strategy**, using `Bloom-Response-*` HTTP headers in your API responses to Bloom.
  * Disable all cache for an API route with `Bloom-Response-Ignore`.
  * Specify caching bucket for an API route with `Bloom-Response-Bucket`.
  * Specify caching TTL in seconds for an API route with `Bloom-Response-TTL` (other than default TTL).
* **Serve `304 Not Modified` to non-modified route contents**, lowering bandwidth usage and speeding up requests to your users.

## The Bloom Approach

Bloom can be hot-plugged to sit between your existing Load Balancers (eg. NGINX), and your API workers (eg. NodeJS). It has been initially built to reduce the workload and drastically reduce CPU usage in case of API traffic spike, or DOS / DDoS attacks.

A simpler caching approach could have been to enable caching at the Load Balancer level for HTTP read methods (`GET`, `HEAD`, `OPTIONS`). Although simple as a solution, it would not work with a REST API. REST API serve dynamic content by nature, that rely heavily on Authorization headers. Also, any cache needs to be purged at some point, if the content in cache becomes stale due to data updates in some database.

NGINX Lua scripts could do that job just fine, you say! Well, I firmly believe Load Balancers should be simple, and be based on configuration only, without scripting. As Load Balancers are the entry point to all your HTTP / WebSocket services, you'd want to avoid frequent deployments and custom code there, and handoff that caching complexity to a dedicated middleware component.

## How does it work?

Bloom is installed on the same box as each of your API workers. As seen from your Load Balancers, there is a Bloom instance per API worker. This way, your Load Balancing setup (eg. Round-Robin with health checks) is not broken. Each Bloom instance can be set to be visible from its own LAN IP your Load Balancers can point to, and then those Bloom instances can point to your API worker listeners on the local loopback.

Bloom acts as a Reverse Proxy of its own, and caches read HTTP methods (`GET`, `HEAD`, `OPTIONS`), while directly proxying HTTP write methods (`POST`, `PATCH`, `PUT` and others). All Bloom instances share the same cache storage on a common `redis` instance available on the LAN.

Bloom is built in Rust for memory safety, code elegance and especially performance. Bloom can be compiled to native code for your server architecture.

Bloom has minimal static configuration, and relies on HTTP response headers served by your API workers to configure caching on a per-response basis. Those HTTP headers are intercepted by Bloom and not served to your Load Balancer responses. Those headers are formatted as `Bloom-Response-*`. Upon serving response to your Load Balancers, Bloom sets a cache status header, namely `Bloom-Status` which can be seen publicly in HTTP responses (either with value `HIT`, `MISS` or `DIRECT` — it helps debug your cache configuration).

![Bloom Schema](https://valeriansaliou.github.io/bloom/docs/models/schema.png)

## How to use it?

### Installation

Bloom is built in Rust. To install it, either download a version from the [Bloom releases](https://github.com/valeriansaliou/bloom/releases) page, or pull the source code from `master`.

Then, build it using `cargo`:

```bash
cargo build --release
```

You can find the built binaries in the `./target/release` directory.

**Notice: [Debian & Ubuntu packages](#how-to-install-it-on-debian--ubuntu) are also available. You may use those instead of building Bloom yourself.**

### Configuration

Use the sample [config.cfg](https://github.com/valeriansaliou/bloom/blob/master/config.cfg) configuration file and adjust it to your own environment.

Make sure to properly configure the `[proxy]` section so that Bloom points to your API worker host and port.

### Run Bloom

Bloom can be run as such:

`./bloom -c /path/to/config.cfg`

**Important: make sure to spin up a Bloom instance for each API worker running on your infrastructure. Bloom does not manage the Load Balancing logic itself, so you should have a Bloom instance per API worker instance and still rely on eg. NGINX for Load Balancing.**

### Configure Load Balancers

Once Bloom is running and points to your API, you can configure your Load Balancers to point to Bloom IP and port (instead of your API IP and port as previously).

Bloom requires the `Bloom-Request-Shard` HTTP header to be set by your Load Balancer upon proxying a client request to Bloom. This header tells Bloom which cache shard to use for storing data (this way, you can have a single Bloom instance for different API sub-systems listening on the same server).

On NGINX, you may add the following rule to your existing proxy ruleset:

```
# Your existing ruleset goes here
proxy_pass http://(...)

# Adds the 'Bloom-Request-Shard' header for Bloom
proxy_set_header Bloom-Request-Shard 0;
```

**Notice: a shard number is an integer from 0 to 255 (8-bit unsigned number).**

## How to install it on Debian & Ubuntu?

Bloom provides [pre-built packages](https://packagecloud.io/valeriansaliou/bloom) for Debian-based systems (Debian, Ubuntu, etc.).

**1️⃣ Add the Bloom APT repository:**

```bash
curl -s https://packagecloud.io/install/repositories/valeriansaliou/bloom/script.deb.sh | sudo bash
```

**2️⃣ Install the Bloom package:**

```bash
sudo apt-get install bloom
```

**3️⃣ Edit the pre-filled Bloom configuration file:**

```bash
sudo nano /etc/bloom.cfg
```

**4️⃣ Restart Bloom:**

```
sudo service bloom restart
```

## How fast is it?

Bloom is built in Rust, which can be compiled to native code for your architecture. Rust, unlike eg. Golang, doesn't carry a GC (Garbage Collector), which is usually a bad thing for high-throughput / high-load production systems (as a GC halts all program instruction execution for an amount of time that depends on how many references are kept in memory).

Note that some compromises have been made relative to how Bloom manages memory. Heap-allocated objects are heavily used for the sake of simplicify. ie. responses from your API workers are fully buffered in memory before they are served to the client; which has the benefit of draining data from your API workers as fast as your loopback / LAN goes, even if the requester client has a very slow bandwidth.

## How does it deal with authenticated routes?

Authenticated routes are usually used by REST API to return data that's private to the requester user. Bloom being a cache system, it is critical that no cache leak from an authenticated route occur. Bloom solves the issue easily by isolating cache in namespaces for requests that send an HTTP `Authorization` header. This is the default, secure behavior.

If a route is being requested without HTTP `Authorization` header (ie. the request is anonymous / public), whatever the HTTP response code, that response will be cached by Bloom.

As your HTTP `Authorization` header contains sensitive authentication data (ie. username and password), Bloom stores those values hashed in `redis` (using a cryptographic hash function). That way, a `redis` database leak on your side will not allow an attacker to recover authentication key pairs.

## Can cache be programatically expired?

Yes. As your existing API workers perform the database updates on their end, they are already well aware of when data - _that might be cached by Bloom_ - gets stale. Therefore, Bloom provides an efficient way to tell it to expire cache for a given bucket. This system is called **Bloom Control**.

Bloom can be configured to listen on a TCP socket to expose a cache control interface. The default TCP port is 811. Bloom implements a basic Command-ACK protocol.

This way, your API worker (or any other worker in your infrastructure) can either tell Bloom to:

* **Expire cache for a given bucket.** Note that as a given bucket may contain variations of cache for different HTTP `Authorization` headers, bucket cache for all authentication tokens is purged at the same time when you purge cache for a bucket.
* **Expire cache for a given HTTP `Authorization` header.** Useful if an user logs-out and revokes their authentication token.

**➡️ Available commands:**

* `FLUSHB <namespace>`: flush cache for given bucket namespace
* `FLUSHA <authorization>`: flush cache for given authorization
* `SHARD <shard>`: select shard to use for connection
* `PING`: ping server
* `QUIT`: stop connection

**⬇️ Control flow example:**

```bash
telnet bloom.local 811
Trying ::1...
Connected to bloom.local.
Escape character is '^]'.
CONNECTED <bloom v1.0.0>
HASHREQ hxHw4AXWSS
HASHRES 753a5309
STARTED
SHARD 1
OK
FLUSHB 2eb6c00c
OK
FLUSHA b44c6f8e
OK
PING
PONG
QUIT
ENDED quit
Connection closed by foreign host.
```

**Notice: before any command can be issued, Bloom requires the client to validate its hasher function against the Bloom internal hasher (done with the `HASHREQ` and `HASHRES` exchange). FarmHash is used to hash keys, using the FarmHash.fingerprint32(), which computed results may vary between architectures. This way, most weird Bloom Control issues are prevented in advance.**

**📦 Bloom Control Libraries:**

* **NodeJS**: **[node-bloom-control](https://www.npmjs.com/package/bloom-control)**

👉 Cannot find the library for your programming language? Build your own and be referenced here! ([contact me](https://valeriansaliou.name/))
