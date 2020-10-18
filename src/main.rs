use std::net::TcpStream;
use std::io::{BufRead, Read, Write, BufReader};

fn main() {
    let host = "cloudflare-2020-general.c0deb0t.workers.dev:80";
    let path = "/links";

    let mut stream = TcpStream::connect(host).unwrap();

    let http_request = format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host);

    stream.write(&http_request.as_bytes()).unwrap();

    let mut reader = BufReader::new(stream);
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        print!("{}", line);

        if line == "\r\n" {
            break;
        } else if line.starts_with("Content-Length:") {
            let len = line.strip_prefix("Content-Length:").unwrap().trim().parse::<usize>().unwrap();
            content_length = Some(len);
        }
    }

    let mut content = vec![0u8; content_length.unwrap()];
    reader.read_exact(&mut content).unwrap();
    println!("{}", String::from_utf8(content).unwrap());
}
