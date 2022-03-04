# dali

![build](https://github.com/olxgroup-oss/dali/workflows/build/badge.svg)

dali (named after the great painter Salvador Dali) is a service which performs image transformations. It is used by [OLX](https://olxgroup.com) to process and serve images for users in around 40 countries.
The application supports:

* Retrieving source images from an HTTP URL
* Encoding images to PNG, JPEG, WEBP or HEIC
* Resizing an image
* Rotating an image
* Apply a watermark image to an image

## Configuration

All configuration should be provided through either a json config file or environment variables. The application will search first by a file named `config/default.json`, then if the environment variable `RUN_MODE` is set it will search for `config/RUN_MODE`, override the values and lastly it will override with environment variables.

| Name | Type | Description | Required | Possible Values | Notes |
|------|------|-------------|----------|-----------------|-------|
| `log_level` | Enum(trace, debug, info, warn, error) | Logging level for the application | N | <ul><li>`error`</li><li>`warn`</li><li>`info`</li><li>`debug`</li><li>`trace`</li></ul> | Default value is `info`. |
| `app_port` | integer | Port which the web server listens to for requests  | Y | - | |
| `health_port` | integer | Port which the web server listens to for the health requests  | Y | - | |
| `server_client_timeout` | integer | Defines a timeout for reading client request header. If a client does not transmit the entire set headers within this time, the request is terminated with the 408 (Request Time-out) error. | N | - | - |
| `client_shutdown_timeout` | integer | Defines a timeout for shutdown connection. If a shutdown procedure does not complete within this time, the request is dropped. | N | - | - |
| `server_keep_alive` | integer | Server keep alive value | N | - | If not specified, it will use the OS's configuration |
| `http_client_con_timeout` | integer | Set the timeout for connecting to a URL | N | - | Default is no timeout |
| `http_client_read_timeout` | integer | Set the timeout for the response | N | - | Default is no timeout |
| `http_client_write_timeout` | integer | Set the timeout for the request | N | - | Default is no timeout |
| `http_client_max_size_of_payload` | integer | Max size of response payload (for `awc` feature only) | N | - | Default is 256Kb |
| `max_threads` | integer | Max number of threads the application will spawn for serving requests and processing images | N | - | If not specified it will take the number of physical CPUs from the machine |
| `vips_threads` | integer | Max number of threads for image processing that will be used | N | - | if not specified it will take `max_threads/2` with a minimum of 1 |
| `app_threads` | integer | Max number of threads for serving requests that will be used | N | - | if not specified it will take `max_threads/2` with a minimum of 1 |
| `metrics_threads` | integer | Max number of threads for serving the `health` and `metrics` endpoints | N | - | Defaults to 1 |

The application will compute the number of threads by the following formula: `pod_number_of_cpus * cpu_usage_percentage / 100`. This number will be divided by 2 and half will be assigned to the HTTP connection listener and half will be assigned to `libvips` (the image library). An extra worker will be created to listen to the `health` endpoint (this was done to be sure the application won't block the `health` endpoint even when overloaded).

## Running locally

### Requirements

* Libvips
* A HTTP server for images
* Docker
* Rust

This application relies on C libvips library. That means it has to be previously installed into the system before compiling and/or running.

For installation follow this [instructions](https://libvips.github.io/libvips/install.html). (Required minimum version 8.10.1)

Using `rustup` is the recommended way to install `rust`. It is a tool that manages and updates rust versions (like `nvm` for node for example). To install it, simply run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`. Then run `rustup update`.

To build and run the application, run the following command:

``` cargo run ```

## Testing

There are 3 kinds of tests: unit, integration and benchmark tests.

### Unit Tests

At the moment, the only logic the application has is regarding resizing and watermark positioning. There are some unit tests around these use cases in the package `image_processor`. All other functions are deeply tied to the `libvips` library which complicates testing since it is not easily mockable.
To run them, simply execute: `cargo test --bin dali`. The parameter `--bin` is needed because otherwise it would run also integration tests.

### Integration Tests

This tests run over a running application.
To run the tests, the script will start the containers through `docker`, copy some sample files to the `http` container and run the tests over the application, checking the array of bytes from the responses against expected result images stored in the `tests/resources/results` directory. To run the whole flow, simply run: `make test`.

### Benchmark Tests

This is an experimental feature from rust, so in order to use, the nightly rust toolchain has to be enabled. To do that run:

```
rustup toolchain install nightly
rustup default nightly
```

To rollback to stable, run `rustup default stable`.

To run the benchmark, simply call `cargo bench` (application must be running at localhost on port 8080).

It outputs the average time per iteration and the deviation between max and min. Example output from a 4 core 2.3GHz MacBook (application running inside docker limited to 4 cores and with 4 workers):

```test bench_highhes ... bench:  71,112,344 ns/iter (+/- 7,798,699)```

## API

The application supports the following endpoints.

### `/health`

Signifies the application is healthy by returning a HTTP Status OK - 200 return code.

### `/metrics`

Prometheus formatted metrics. Currently exposes request count and duration per endpoint

### `/`

Fetches and processes an image file. The only mandatory parameter is the `image_address`.

#### General query parameters

| Parameter | Description |
|-----------------|-------------|
| `image_address` | The address for the Image. Should be a HTTP, HTTPS or HTTP valid URI. |
| `format` | desired image format. Possible values are `Jpeg`, `Png`, `Heic` and `Webp`. Defaults to Jpeg |
| `quality` | desired quality for the image. For Jpeg, it goes from 0 to 100 (defaults to 75) |
| `size[width]` | desired width for the image. Images won't get upscaled or have their aspect ratio changed by variations on parameters for width and height. |
| `size[height]` | desired height for the image. Images won't get upscaled or have their aspect ratio changed by variations on parameters for width and height. |
| `rotation` | optional rotation of the image. Possible values are `R90`, `R180` and `R270` |

#### Watermarking query parameters

Watermarks is an array parameter and therefore, must be indexed when informed (0 indexed).

| Parameter | Description |
|-----------------|-------------|
| `watermarks[0][image_address]` | watermark file. File has to be smaller than original file. Should be a HTTP, HTTPS or HTTP valid URI. |
| `watermarks[0][alpha]` | opacity from the watermark over the original image. it is a floating point number from 0 to 1. |
| `watermarks[0][position][x][origin]` | identifier to position the watermark based on a point or centered (X axis). Possible values: Left (default), Right, Center. |
| `watermarks[0][position][y][origin]` | identifier to position the watermark based on a point or centered (Y axis). Possible values: Top (default), Bottom, Center. |
| `watermarks[0][position][x][pos]` | position of the watermark in the X axis. Value in pixels. |
| `watermarks[0][position][y][pos]` | position of the watermark in the Y axis. Value in pixels. |
| `watermarks[0][size]` | optional size of the watermark. It should be a value between 1 and 100 representing a percentage from the original image. |

## License

(c) Copyright 2019-2020 [OLX](https://olxgroup.com). Released under [Apache 2 License](LICENSE)
