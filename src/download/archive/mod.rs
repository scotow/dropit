async fn archive(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (w, r) = io::duplex(1024);

    tokio::spawn(async move {
        let mut ar = Builder::new(w.compat());
        ar.mode(HeaderMode::Deterministic);

        let mut header = Header::new_gnu();
        header.set_path("README.md").unwrap();
        header.set_mode(0o400);
        header.set_size(12);
        header.set_cksum();
        ar.append(&mut header, &b"Hello, World"[..]).await.unwrap();

        ar.finish().await.unwrap();
    });

    Ok(Response::new(Body::wrap_stream(ReaderStream::new(r))))
}