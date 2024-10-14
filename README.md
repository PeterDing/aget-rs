<h1 align="center">Aget-rs - Fast Asynchronous Downloader with Rust 🦀</h1>

[![CI](https://github.com/PeterDing/aget-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/PeterDing/aget-rs/actions/workflows/ci.yml)

`aget-rs` is a fast asynchronous downloader written in Rust.  
It requests a resource with a number of concurrent asynchronous request in a single thread.

Especially, the concurrent amount can be any positive number as your wish.

`aget-rs` supports to download a **HTTP/S** link, a **M3U8** video link, a **Torrent** and a **magnet** link.

## Installation

You can download the last release from https://github.com/PeterDing/aget-rs/releases

## Benchmark

We conside that there is a file to download. This file has 10MB.
The server which hosts the file has been set a limit rate to 100KB/s, but no connecting count limit.

It will be easy to calculate the total costing time when we use 1, 10, 100 connections to request the file.

In the benchmark test, we use `nginx` to simulate the environment where a limit rate is 100KB/s for downloading.

Following is the results of using `curl` and `aget-rs`. (For more details, you can find at [here](ci/benchmark.bash))

- One connection using `curl`

  ```
  time curl http://localhost:9010/abc
    % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                   Dload  Upload   Total   Spent    Left  Speed
  100 10.0M  100 10.0M    0     0   100k      0  0:01:42  0:01:42 --:--:--  103k
  real	1m42.147s
  user	0m0.021s
  sys	0m0.035s
  ```

  **time cost: 102s**

- 10 connections using `aget-rs`

  ```
  time ag http://localhost:9010/abc -s 10 -k 1m
      File: abc
    Length: 10.0M (10485760)
  10.0M/10.0M 100.00% NaNG/s eta: 0s        [==================================>]
  real	0m10.016s
  user	0m0.040s
  sys	0m0.020s
  ```

  **time cost: 10s, 10 times faster than curl**

- 100 connections using `aget-rs`

  ```
  time ag http://localhost:9010/abc -s 100 -k 103k
      File: abc
    Length: 10.0M (10485760)
  10.0M/10.0M 100.00% NaNG/s eta: 0s        [==================================>]
  real	0m2.016s
  user	0m0.087s
  sys	0m0.029s
  ```

  **time cost: 2s, 50 times faster than curl**

## Usage

- Request a resource with default configuration

  The default concurrent amount is `10` and chunk length is `1m`.

  ```shell
  ag http://cdimage.ubuntu.com/ubuntu/releases/18.10/release/ubuntu-18.10-server-amd64.iso
  ```

- Set concurrent amount and chunk length

  Use `-s` or `--concurrency` to set the number of concurrent request.  
   Use `-k` or `--chunk-size` to set the chunk length of each request.  
   `--chunk-size` takes a literal size description, example `1k` for one Kilobyte,  
   `2m` for two Megabyte, `1g` for Gigabyte.

  ```shell
  ag "url of resource" -s 20 -k 1m
  ```

- Set a path for output

  Use `-o` or `--out` to set the path.  
   If the argument is not gave, we take the last part of the url' path as the path.

  ```shell
  ag "url of resource" -o /path/to/file
  ```

  When download a torrent or magnet link, the path is the output directory.

- Set request headers

  Use `-H` to set headers.

  ```shell
  ag "url of resource" -H "Cookie: key=value" -H "Accept: */*"
  ```

- Set request method and data

  Use `-X` or `--method` to set method for http, example, `GET`, `POST`.  
   The default method is `GET`.  
   With a data, using `-d` or `--data`, example, `a=b`

  ```shell
  ag "url of resource" -d "a=b"
  ```

- Download a torrent or magnet link

  **Warning**: The `/path/to/outdir` directory below command must NOT exist. It will be created automatically.

  ```shell
  ag "magnet:..." -o /path/to/outdir
  ag "/path/to/torrent" -o /path/to/outdir
  ag "http://example.com/some.torrent" -o /path/to/outdir
  ```

  Use `--bt-file-regex` to only download files matching it in the torrent.

  ```shell
  ag "magnet:..." -o /path/to/outdir --bt-file-regex ".*\.mp4"
  ```

  Use `--seed` to seed the torrent after downloaded.

  ```shell
  ag "magnet:..." -o /path/to/outdir --seed
  ```

  Use `--bt-trackers` to specify trackers with comma as delimiter.

  ```shell
  ag "magnet:..." -o /path/to/outdir --bt-trackers "udp://tracker.opentrackr.org:1337/announce,udp://opentracker.io:6969/announce"
  ```

## Options

```
Usage: ag [OPTIONS] <URL>

Arguments:
  <URL>

Options:
  -m, --method <METHOD>                Request method, e.g. GET, POST [default: GET]
  -H, --header <HEADER>                Request headers, e.g. -H "User-Agent: aget"
  -d, --data <DATA>                    Request with POST method with the data, e.g. -d "a=b"
      --insecure                       Skip to verify the server's TLS certificate
  -s, --concurrency <CONCURRENCY>      The number of concurrency request [default: 10]
  -k, --chunk-size <CHUNK_SIZE>        The number ofinterval length of each concurrent request [default: '50m']
  -t, --timeout <TIMEOUT>              Timeout(seconds) of request [default: 60]
      --dns-timeout <DNS_TIMEOUT>      DNS Timeout(seconds) of request [default: 10]
      --retries <RETRIES>              The maximum times of retring [default: 5]
      --retry-wait <RETRY_WAIT>        The seconds between retries [default: 0]
      --proxy <PROXY>                  [protocol://]host[:port] Use this proxy
      --type <TYPE>                    Task type, auto/http/m3u8/bt [default: auto]
      --bt-file-regex <BT_FILE_REGEX>  A regex to only download files matching it in the torrent
      --seed                           Seed the torrent
      --bt-trackers <BT_TRACKERS>      Trackers for the torrent, e.g. --bt-trackers "udp://tracker.opentrackr.org:1337/announce
                                       ,udp://opentracker.io:6969/announce"
      --debug                          Debug output. Print all trackback for debugging
      --quiet                          Quiet mode. Don't show progress bar and task information. But still show the error information
  -o, --out <OUT>                      The path of output for the request e.g. -o "/path/to/file"
  -h, --help                           Print help
  -V, --version                        Print version
```

## Configuration

Aget can be configured by a configuration file. The file locates at `~/.config/aget/config`.
Following options can be set. Aget uses these options as the defaults for each command.

```
headers = [["key", "value"], ...]
concurrency = ...
chunk_size = "..."
timeout = ...
dns_timeout = ...
retries = ...
retry_wait = ...
```

If the file does not exist, aget will use the default configuration.

```toml
headers = [["user-agent", "aget/version"]]
concurrency = 10
chunk_size = "50m"
timeout = 60
dns_timeout = 10
retries = 5
retry_wait = 0
```
