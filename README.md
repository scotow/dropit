[![Release](https://img.shields.io/github/v/tag/scotow/dropit?label=version)](https://github.com/scotow/dropit/tags)
[![Build Status](https://img.shields.io/github/actions/workflow/status/scotow/dropit/ci.yml)](https://github.com/scotow/dropit/actions)


![Logo](banner.png)

## Features

- Upload files from the terminal (by using `curl` or the [shell script](https://github.com/scotow/dropit/blob/master/upload.sh))
- Short and long aliases generation, short to copy/past and long to easily share it verbally
- Configurable expiration based on file size
- Quota based on users' IP addresses or usernames
- Revocable files
- Expiration refresh
- Alias regeneration
- Archive download
- Downloads limit
- JSON or plain text response (helpful for scripting)
- Authenticate upload and/or download using Basic HTTP Auth or LDAP (direct bind or dn search)
- Upload files from a minimalist web interface:
  - Drag & drop
  - QRCode generation
  - Upload progress bar
  - Readable size, duration and expiration
  - Cache uploads links
  - Customizable color
  
## Configuration

### Options

```
Usage: dropit [OPTIONS] --threshold <THRESHOLDS> --origin-size-sum <ORIGIN_SIZE_SUM> --origin-file-count <ORIGIN_FILE_COUNT> --global-size-sum <GLOBAL_SIZE_SUM> <--ip-origin|--username-origin>

Options:
  -v, --verbose...                                                     Increase logs verbosity (Error (default), Warn, Info, Debug, Trace)
  -u, --uploads-dir <UPLOADS_DIR>                                      Upload files directory path (relative) [default: uploads]
  -U, --no-uploads-dir-creation                                        Disable upload files directory automatic creation (if missing)
  -d, --database <DATABASE>                                            Metadata database path (relative) [default: dropit.db]
  -D, --no-database-creation                                           Disable metadata database automatic creation (if missing)
  -a, --address <ADDRESS>                                              HTTP listening address [default: 127.0.0.1]
  -p, --port <PORT>                                                    HTTP listening port [default: 8080]
  -R, --behind-reverse-proxy                                           Use X-Forwarded-For, X-Forwarded-Proto and X-Forwarded-Host to determine uploads' origin
  -t, --threshold <THRESHOLDS>                                         Relations between files' sizes and their durations. Must be ordered by increasing size and decreasing duration
  -o, --ip-origin                                                      Use usernames as uploaders' identities
  -O, --username-origin                                                Use IP addresses as uploaders' identities
  -s, --origin-size-sum <ORIGIN_SIZE_SUM>                              Cumulative size limit from the same uploader
  -c, --origin-file-count <ORIGIN_FILE_COUNT>                          Number of files limit from the same uploader
  -S, --global-size-sum <GLOBAL_SIZE_SUM>                              Cumulative size limit from all users
      --auth-upload                                                    Protect upload endpoint with authentication
      --auth-download                                                  Protect download endpoint with authentication
  -C, --credential <CREDENTIALS>                                       Static list of credentials
      --ldap-address <LDAP_ADDRESS>                                    URI of the LDAP used to authenticate users
      --ldap-dn-pattern <LDAP_DN_PATTERN>                              LDAP DN pattern used when using single bind process
      --ldap-search-base-dn <LDAP_SEARCH_BASE_DN>                      LDAP base DN used during username searches
      --ldap-search-attribute-pattern <LDAP_SEARCH_ATTRIBUTE_PATTERN>  LDAP attribute(s) pattern used to match usernames during searches [default: (uid=%u)]
      --ldap-search-dn <LDAP_SEARCH_DN>                                LDAP DN used to bind during username searches
      --ldap-search-password <LDAP_SEARCH_PASSWORD>                    LDAP password used to bind during username searches
  -T, --theme <THEME>                                                  CSS color used in the web UI [default: #15b154]
  -h, --help                                                           Print help information
  -V, --version                                                        Print version information                                                        Print version information
```

Here is an example of a Dropit instance:

```
dropit \
  --ip-origin \
  --origin-size-sum 512MB \
  --origin-file-count 64 \
  --global-size-sum 10GB \
  --threshold 64MB:24h \
  --threshold 256MB:6h \
  --credential admin:password \
  --auth-upload \
  --behind-reverse-proxy
```

- Using uploader IP address to limit / calculate upload quota 
- Allowing at most 64 simultaneous files from the same IP
- Allowing a total of 512MB of file content from the same IP
- Allowing a total of 10GB of file content from anybody
- Setting the duration of files smaller than 64MB to 24h
- Setting the duration of files smaller than 256MB to 6h
- Forbidding files larger than 256MB
- Protecting upload endpoint with a basic auth and using admin/password as credentials
- Using the X-Forwarded-For header to determine user IP address
- Listening on default address and port (127.0.0.1:8080)
- Creating (if needed) a directory named "uploads" (default) and storing uploaded files in it
- Creating (if needed) the SQLite database "dropit.db" (default)

### Reverse-proxy

If you host Dropit behind a reverse-proxy, make sure to use the `--behind-reverse-proxy` option and to forward the client IP, protocol and original host by setting the `X-Forwarded-For`, `X-Forwarded-Proto` and `X-Forwarded-Host` headers.    

### Docker

If you prefer to run Dropit as a Docker container, you can either build the image yourself using the Dockerfile available in this repo, or you can use the [image](https://github.com/scotow/dropit/packages/737180) built by the GitHub action.

```
docker run -p 8080:8080 docker.pkg.github.com/scotow/dropit/dropit:latest [FLAGS] [OPTIONS]
```

Please read [Binding to all interfaces](#binding-to-all-interfaces) if you can't reach the process from outside the image.

### Binding to all interfaces

By default, Dropit will only listen on the loopback interface, aka. 127.0.0.1. If you **don't** want to host Dropit behind a reverse proxy or if you are using the Docker image, you should specify the `0.0.0.0` address by using the `-a | --address` option.

## Libraries

- `hyper` as an HTTP backend and `axum` for routing
- `SQLite` and `sqlx` as a metadata storage
- `tokio` as an async runtime
- `clap` for options parsing and usage generation
