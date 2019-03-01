# Aget-rs - Fast Asynchronous Downloader with Rust 🦀

`aget-rs` is a fast asynchronous downloader wrote with Rust.  
It requests a resource with a number of concurrent asynchronous request in a single thread.

Especially, the concurrent amount can be any positive number as your wish.

## Installation

You can download the last release from https://github.com/PeterDing/aget-rs/releases

## Usage

1. Request a resource with default configuration
  The default concurrent amount is `10` and chunk length is `500k`.
    ```shell
    ag http://cdimage.ubuntu.com/ubuntu/releases/18.10/release/ubuntu-18.10-server-amd64.iso
    ```

2. Set concurrent amount and chunk length
  Use `-s` or `--concurrent` to set the number of concurrent request.
  Use `-k` or `--chunk-size` to set the chunk length of each request.
  `--chunk-size` takes a literal size description, example `1k` for one Kilobyte,
  `2m` for two Megabyte, `1g` for Gigabyte.
    ```shell
    ag "url of resource" -s 20 -k 1m
    ```

3. Set a path for output
  Use `-o` or `--out` to set the path.
  If the argument is not gave, we take the last part of the url' path as the path.
    ```shell
    ag "url of resource" -o /path/to/file
    ```

4. Set request headers
  Use `-H` to set headers.
    ```shell
    ag "url of resource" -H "Cookie: key=value" -H "Accept: */*"
    ```

5. Set request method and data
  Use `-X` or `--method` to set method for http, example, `GET`, `POST`.
  The default method is `GET`.
  With a data, using `-d` or `--data`, example, `a=b`
    ```shell
    ag "url of resource" -d "a=b"
    ```

## Options

```
USAGE:
    ag <URL>

OPTIONS:
    -X, --method <method>                Request method,
                                         e.g. GET, POST.
                                         [default: GET]
    -H, --header <header>...             Request headers,
                                         e.g. -H "User-Agent: aget".
    -d, --data <data>                    Request with POST method with the data,
                                         e.g. -d "a=b".
    -o, --out <out>                      The path of output for the request,
                                         e.g. -o "/path/to/file".
    -s, --concurrent <concurrent>        The number of concurrent request,
                                         e.g. -s 10
                                         [default: 10]
    -k, --chunk-length <chunk-length>    The interval length of each concurrent request,
                                         e.g. -k 100k
                                         [default: 1m]

    -h, --help                           Print this help message.
    -V, --version                        Show version information.

ARGS:
    <URL>    URL to request.
```
