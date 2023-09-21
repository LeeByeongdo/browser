use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use flate2::read::{GzDecoder};
use chunked_transfer::Decoder;

struct URL {
    scheme: String,
    host: String,
    port: u32,
    path: String,
}

struct Header {
    key: String,
    value: String,
}

#[derive(Debug)]
struct Response {
    headers: HashMap<String, String>,
    body: String,
}

fn main() {
    // utf-8 encoded
    // let url = "https://example.org";

    // ISO-8859-1 encoded
    let url = "https://www.google.com";

    //file
    // let url = "file:///Users/byeongdolee/ip_geolocation_2023_04_25_12_42_22.txt";

    //data
    // let url = "data:text/html,Hello world!";

    // view-source
    // let url = "view-source:https://example.org";

    load(url);
}

fn load(url: &str) {
    if url.starts_with("file://") {
        let path = &url[7..];
        let contents = fs::read_to_string(path).expect(format!("Can not read file: {}", path).as_str());
        println!("{}", contents);
        return;
    } else if url.starts_with("data:") {
        let blocks: Vec<&str> = url.splitn(2, ",").collect();
        println!("type: {}\ncontent: {}", blocks[0], blocks[1]);
        return;
    } else if url.starts_with("view-source:") {
        let res = request(url).unwrap();
        show(&transform(&res.body));
        return;
    }

    let res = request(url).unwrap();
    show(&res.body);
}

fn transform(body: &String) -> String {
    let mut result = String::from("<body>");
    result.push_str(body.replace("<", "&lt;").replace(">", "&gt;").as_str());
    result.push_str("</body>");
    return result;
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
    if url.scheme == "http" {
        return Err("no http supported");
    }

    let target = format!("{}:{}", url.host, url.port);

    let mut stream = TcpStream::connect(target.clone()).expect("Couldn't connect to server...");

    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs") {
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

    let request_headers = vec![
        Header { key: String::from("Host"), value: String::from(url.host) },
        Header { key: String::from("Connection"), value: String::from("close") },
        Header { key: String::from("User-Agent"), value: String::from("BDBDBDLEE-BROWSER") },
        Header { key: String::from("Accept-Encoding"), value: String::from("gzip") },
    ];

    let header_part = request_headers.iter().fold(String::new(), |a, b| format!("{}{}: {}\r\n", a, b.key, b.value));

    tls.write(format!("GET {} HTTP/1.1\r\n{}\r\n", url.path, header_part).as_bytes())
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

    println!("version: {}, status: {}, explanation: {}", version, status, explanation);

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

    let mut buffer = Vec::new();

    // 마지막에 unexpected end of file 에러가 나는데, 버퍼는 정상적으로 다 읽힘.
    let _ = reader.read_to_end(&mut buffer);

    if headers.contains_key("transfer-encoding") && headers["transfer-encoding"] == "chunked" {
        let mut chunk_decoded = vec![];
        let mut decoder = Decoder::new(&buffer[..]);
        decoder.read_to_end(&mut chunk_decoded).unwrap();
        buffer = chunk_decoded;
    }

    if headers.contains_key("content-encoding") && headers["content-encoding"] == "gzip" {
        let mut decoder = GzDecoder::new(&buffer[..]);
        let mut buffer2 = Vec::new();
        decoder.read_to_end(&mut buffer2).unwrap();
        let body = String::from_utf8_lossy(&buffer2).to_string();

        return Ok(Response { headers, body });
    }

    let body = String::from_utf8_lossy(&buffer).to_string();

    Ok(Response { headers, body })
}

fn show(body: &String) {
    let mut in_angle = false;
    let mut in_body = false;
    let mut current_tag = String::new();
    let mut result = String::new();

    for (_, c) in body.chars().enumerate() {
        match c {
            '<' => {
                in_angle = true;
                current_tag = String::new();
            }
            '>' => {
                in_angle = false;
            }
            _ => {
                if in_angle {
                    current_tag.push(c);
                }

                if current_tag == "body" {
                    in_body = true;
                }

                if current_tag == "/body" {
                    in_body = false;
                }

                if !in_angle && in_body {
                    result.push(c);
                }
            }
        }
    }
    result = result.replace("&lt;", "<").replace("&gt;", ">");
    print!("{}", result);
}
