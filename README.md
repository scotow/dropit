# Dropit

Dropit is a temporary file hosting and sharing solution.

### Features

- Upload files from the terminal (by using `curl` for example);
- Short and long alias generation, short to copy/past it and long to easily share it verbally;
- Configurable expiration based on file size;
- Quota determiner based on user IP;
- Upload files from a minimalist web interface:
    - Drag & drop of files;
    - QRCode generation;
    - Upload progress bar;  
    - Readable size, duration and expiration.
    
### Foreseeable features

- Different upload output formats:
    - `plain/text` to help in the terminal and for scripting;
    - QRCode in the terminal.
- Archive download (zip/tar);
- Revoke API/button;
- Alias regeneration;
- File refresh;
- Trusted reverse proxies.

### Libraries

- `hyper` as a HTTP backend and `routerify` to help with the routing. Evaluating alternatives like `warp` in the future;
- `Sqlite` and `sqlx` as a metadata storage;
- `tokio` as an async runtime.
