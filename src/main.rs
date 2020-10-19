use std::net::TcpStream;
use std::io::{BufRead, Read, Write, BufReader};
use std::time::Instant;

#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(cloudflare_2020_systems_engineering_assignment =>
        (version: "0.1.0")
        (author: "c0deb0t <daniel.liu02@gmail.com>")
        (about: "Send HTTP Get requests to a URL.")
        (@arg url: --url +takes_value +required "URL to send requests to")
        (@arg profile: --profile +takes_value "Number of requests to send")
    ).get_matches();

    let (host, path) = parse_url(&matches.value_of("url").unwrap());
    let is_profiling = matches.value_of("profile").is_some();
    let profile = matches.value_of("profile").unwrap_or("1").parse::<usize>().unwrap();

    if profile == 0 {
        panic!("Profile count needs to be greater than 0!");
    }

    let mut stream = TcpStream::connect(&host).unwrap();

    // use HTTP/1.1 so we can directly send a plain text request
    let http_request = format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host);

    print!("Sending {} request(s):\n{}", profile, http_request);

    let mut reader = BufReader::new(stream.try_clone().unwrap());

    let mut stat_times = vec![];
    let mut stat_sizes = vec![];
    let mut stat_succeeded = 0usize;

    for _i in 0..profile {
        let start_time = Instant::now();
        stream.write(&http_request.as_bytes()).unwrap();

        // parse header and extract important information
        // also print out the header if not profiling

        let mut content_length: Option<usize> = None;
        let mut chunked = false;
        let mut header_start = false;
        let mut size = 0usize;

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            size += line.len();

            if let Some(code) = line.strip_prefix("HTTP/1.1") {
                header_start = true;
                let status = code.trim();
                let elapsed = start_time.elapsed().as_millis();

                if is_profiling {
                    println!("Received response (in {} ms): {}", elapsed, status);
                } else {
                    println!("Received response (in {} ms):", elapsed);
                }

                if status.starts_with("2") {
                    stat_succeeded += 1;
                }

                stat_times.push(elapsed);
            } else if let Some(len) = line.strip_prefix("Content-Length:") {
                content_length = Some(len.trim().parse::<usize>().unwrap());
            } else if line.starts_with("Transfer-Encoding: chunked") {
                chunked = true;
            } else if header_start && line == "\r\n" {
                break;
            }

            if !is_profiling {
                print!("{}", line);
            }
        }

        if !is_profiling {
            println!();
        }

        // read in the body and display it
        // print out body if not profiling
        if let Some(len) = content_length {
            // easy case: known content length
            let mut content = vec![0u8; len];
            reader.read_exact(&mut content).unwrap();

            if !is_profiling {
                println!("{}", decode_bytes(&content));
            }

            size += content.len();
        } else if chunked {
            // hard case: read in chunks and merge the chunks together
            let mut content = Vec::with_capacity(1024);

            loop {
                let mut len_line = String::new();
                reader.read_line(&mut len_line).unwrap();
                size += len_line.len();
                let chunk_len = usize::from_str_radix(&len_line.trim(), 16).unwrap();
                // add two bytes to include the CR and LF bytes
                let mut buf = vec![0u8; chunk_len + 2];
                reader.read_exact(&mut buf).unwrap();
                content.extend_from_slice(&buf[..buf.len() - 2]);
                size += buf.len();

                if chunk_len == 0 {
                    break;
                }
            }

            if !is_profiling {
                println!("{}", decode_bytes(&content));
            }
        }

        stat_sizes.push(size);
    }

    // output final statistics when profiling
    if is_profiling {
        println!("\nReceived {} ({:.2}%) successful responses", stat_succeeded, (stat_succeeded as f64) / (profile as f64) * 100.0f64);
        let mean_time = (stat_times.iter().sum::<u128>() as f64) / (stat_times.len() as f64);
        let median_time = median(&stat_times);
        let min_time = stat_times.iter().min().unwrap();
        let max_time = stat_times.iter().max().unwrap();
        println!("Response time:\n\tMean: {:.2} ms\n\tMedian: {:.2} ms\n\tMin: {} ms\n\tMax: {} ms", mean_time, median_time, min_time, max_time);
        let min_size = stat_sizes.iter().min().unwrap();
        let max_size = stat_sizes.iter().max().unwrap();
        println!("Response size:\n\tMin: {} bytes\n\tMax: {} bytes", min_size, max_size);
    }
}

fn median(a: &[u128]) -> f64 {
    let mut a = a.to_owned();
    a.sort();

    if a.len() % 2 == 0 {
        ((a[a.len() / 2] + a[a.len() / 2 - 1]) as f64) / 2.0f64
    } else {
        a[a.len() / 2] as f64
    }
}

// try decoding using UTF-8, and if that does not work, then try Latin-1
fn decode_bytes(content: &[u8]) -> String {
    match String::from_utf8(content.to_owned()) {
        Ok(s) => s,
        Err(_) => content.iter().map(|&c| c as char).collect::<String>()
    }
}

// hand-rolled parser to handle simple URL cases
fn parse_url(url: &str) -> (String, String) {
    let url_no_http = if let Some(u) = url.strip_prefix("https://") {
        u.to_owned()
    } else if let Some(u) = url.strip_prefix("http://") {
        u.to_owned()
    } else {
        url.to_owned()
    };

    let slash_idx = url_no_http.find("/");
    let mut host;
    let path;

    if let Some(idx) = slash_idx {
        host = url_no_http[..idx].to_owned();
        path = url_no_http[idx..].to_owned();
    } else {
        host = url_no_http.clone();
        path = "/".to_owned();
    }

    if !host.contains(":") {
        host.push_str(":80");
    }

    return (host, path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median() {
        let a = vec![1, 100, 2, 1000, 3];
        assert_eq!(median(&a), 3.0f64);
        let b = vec![10, 10, 20, 20];
        assert_eq!(median(&b), 15.0f64);
    }

    #[test]
    fn test_decode() {
        let s = vec![0xDFu8];
        assert_eq!(decode_bytes(&s), "\u{00DF}");
    }

    #[test]
    fn test_parse_url() {
        let a = parse_url("example.com");
        assert_eq!(a, ("example.com:80".to_owned(), "/".to_owned()));

        let b = parse_url("example.com:1234/hi");
        assert_eq!(b, ("example.com:1234".to_owned(), "/hi".to_owned()));

        let c = parse_url("http://example.com/hi");
        assert_eq!(c, ("example.com:80".to_owned(), "/hi".to_owned()));

        let d = parse_url("https://example.com/hi");
        assert_eq!(d, ("example.com:80".to_owned(), "/hi".to_owned()));

        let e = parse_url("https://www.example.com:1234/hi/hello");
        assert_eq!(e, ("www.example.com:1234".to_owned(), "/hi/hello".to_owned()));
    }
}
