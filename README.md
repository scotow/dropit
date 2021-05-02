# Dropit

🗃 Dropit is a temporary file hosting and sharing solution. 🗃

## Features

- Upload files from the terminal (by using `curl` for example)
- Short and long alias generation, short to copy/past it and long to easily share it verbally
- Configurable expiration based on file size
- Quota determiner based on user IP
- Json or plain text response (helpful for scripting)
- Upload files from a minimalist web interface:
    - Drag & drop of files
    - QRCode generation
    - Upload progress bar
    - Readable size, duration and expiration
    - Customizable color
  
## Configuration

### Options

```
USAGE:
    dropit [FLAGS] [OPTIONS] --global-size-sum <global-size-sum> --ip-file-count <ip-file-count> --ip-size-sum <ip-size-sum> --threshold <thresholds>...

FLAGS:
    -R, --behind-reverse-proxy    
    -h, --help                    Prints help information
    -V, --version                 Prints version information

OPTIONS:
    -a, --address <address>                     [default: 127.0.0.1]
    -C, --color <color>                         [default: #15b154]
    -S, --global-size-sum <global-size-sum>    
    -c, --ip-file-count <ip-file-count>        
    -s, --ip-size-sum <ip-size-sum>            
    -p, --port <port>                           [default: 8080]
    -t, --threshold <thresholds>...            
    -u, --uploads-dir <uploads-dir>             [default: uploads]
```

Here is an example of a Dropit instance:

```
dropit \
  --ip-size-sum 512MB \
  --ip-file-count 64 \
  --global-size-sum 10GB \
  --threshold 64MB:24h \
  --threshold 256MB:6h \
  --behind-reverse-proxy
```

- Allowing at most 64 simultaneous files from the same IP
- Allowing a total of 512 MB of file content from the same IP
- Allowing a total of 10 GB of file content from anybody
- Setting the duration of files smaller than 64 MB to 24h
- Setting the duration of files smaller than 256 MB to 6h
- Forbidding files larger than 256M
- Using the X-Forwarded-For header to determine user IP address
- Listening on default address and port (127.0.0.1:8080)
- Creating (if needed) a directory named "uploads" (default) and storing uploaded files in it

### Reverse-proxy

If you host Dropit behind a reverse-proxy, make sure to use the `--behind-reverse-proxy` option and to forward the client IP, protocol and original host by setting the `X-Forwarded-For`, `X-Forwarded-Proto` and `X-Forwarded-Host` headers.    
    
## Foreseeable features

- Archive download (zip/tar)
- Revoke API / button
- Alias regeneration
- File refresh

## Libraries

- `hyper` as a HTTP backend and `routerify` to help with the routing. Evaluating alternatives like `warp` in the future
- `Sqlite` and `sqlx` as a metadata storage
- `tokio` as an async runtime
- `structopt` for options parsing and usage generation
