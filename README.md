# Dropit

Dropit is a temporary file hosting and sharing solution.

### Features

- Upload files from the terminal (by using `curl` for example)
- Short and long alias generation, short to copy/past it and long to easily share it verbally
- Configurable expiration based on file size
- Quota determiner based on user IP
- Upload files from a minimalist web interface:
    - Drag & drop of files
    - QRCode generation
    - Upload progress bar
    - Readable size, duration and expiration
  
### Configuration

#### Options

```
USAGE:
    dropit [OPTIONS] --ip-file-count <ip-file-count> --ip-size-sum <ip-size-sum> --threshold <thresholds>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --address <address>                 [default: 127.0.0.1]
    -I, --ip-file-count <ip-file-count>    
    -i, --ip-size-sum <ip-size-sum>        
    -p, --port <port>                       [default: 8080]
    -t, --threshold <thresholds>...        
    -u, --uploads-dir <uploads-dir>         [default: uploads]
```

Here is an example of a Dropit instance:
- Allowing at most 64 simultaneous files from the same IP
- Allowing a total of 512M of file content from the same IP
- Setting the duration of files smaller than 64M to 24h
- Setting the duration of files smaller than 256M to 6h
- Forbidding files larger than 256M
- Listening on default address and port (127.0.0.1:8080)
- Create (if needed) a directory named "uploads" (default) and store uploaded files in it

```
dropit \
  --ip-size-sum 512000000 \
  --ip-file-count 64 \
  --threshold 64000000:86400 \
  --threshold 256000000:21600
```

#### Reverse-proxy

If you host Dropit behind a reverse-proxy, make sure to forward the original host, client IP and protocol using the  
    
### Foreseeable features

- Different upload output formats:
    - `plain/text` to help in the terminal and for scripting
    - QRCode in the terminal
- Archive download (zip/tar)
- Revoke API/button
- Alias regeneration
- File refresh
- Trusted reverse proxies

### Libraries

- `hyper` as a HTTP backend and `routerify` to help with the routing. Evaluating alternatives like `warp` in the future;
- `Sqlite` and `sqlx` as a metadata storage;
- `tokio` as an async runtime.
