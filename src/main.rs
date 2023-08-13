use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

#[derive(Debug)]
struct URL {
    scheme: String,
    host: String,
    port: u32,
    path: String,
}

#[derive(Debug)]
struct Response {
    headers: HashMap<String, String>,
    body: String,
}

fn main() {
    let url = "https://example.org";
    load(url);
}

fn load(url: &str) {
    let res = request(url).unwrap();
    show(&res.body);
}

fn parse_url(url: &str) -> URL {
    let blocks: Vec<&str> = url.splitn(2, "://").collect();
    let scheme = String::from(blocks[0]);
    let mut url = String::from(blocks[1]);

    if !url.contains("/") {
        url.push_str("/");
    }

    let blocks: Vec<&str> = url.splitn(2, "/").collect();
    let host = String::from(blocks[0]);
    let port = if host.contains(":") {
        let blocks: Vec<&str> = url.splitn(2, ":").collect();
        blocks[1].parse::<u32>().unwrap()
    } else if scheme == "http" {
        80
    } else {
        443
    };

    let mut path = String::from("/");
    path.push_str(blocks[1]);

    URL {
        scheme,
        host,
        port,
        path,
    }
}

fn request(url: &str) -> Result<Response, &str> {
    let url = parse_url(url);

    let target = format!("{}:{}", url.host, url.port);
    println!("{}", target);
    if let Ok(mut stream) = TcpStream::connect(target.clone()) {
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs")
        {
            roots.add(&rustls::Certificate(cert.0)).unwrap();
        }

        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();

        let mut conn =
            rustls::ClientConnection::new(Arc::new(config), url.host[..].try_into().unwrap())
                .unwrap();
        let mut tls = rustls::Stream::new(&mut conn, &mut stream);

        tls.write(format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url.path, url.host).as_bytes())
            .expect("error to write");

        let mut reader = BufReader::new(tls);
        let mut status = String::new();

        reader
            .read_line(&mut status)
            .expect("error reading status line.");

        let blocks: Vec<&str> = status.splitn(3, " ").collect();
        let version = blocks[0];
        let status = blocks[1];
        let explanation = blocks[2];

        let mut headers = HashMap::new();

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("error reading line.");
            if line.eq("\r\n") {
                break;
            }

            let blocks: Vec<&str> = line.splitn(2, ":").collect();
            headers.insert(
                String::from(blocks[0].to_lowercase()),
                String::from(blocks[1].trim()),
            );
        }

        assert!(!headers.contains_key("transfer-encoding"));
        assert!(!headers.contains_key("content-encoding"));

        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();

        Ok(Response { headers, body })
    } else {
        println!("Couldn't connect to server...");
        Err("connection error")
    }
}

fn show(body: &String) {
    let mut in_angle = false;

    for (_, c) in body.chars().enumerate() {
        match c {
            '<' => {
                in_angle = true;
            }
            '>' => {
                in_angle = false;
            }
            _ => {
                if !in_angle {
                    print!("{}", c);
                }
            }
        }
    }
}
