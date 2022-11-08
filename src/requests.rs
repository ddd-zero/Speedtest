use std::error::Error;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use log::debug;
use openssl::ssl::{SslConnector, SslMethod, SslStream};
use rand::prelude::*;
use url::Url;

pub trait GenericStream: Read + Write {}

impl<S: Read + Write> GenericStream for SslStream<S> {}

impl GenericStream for TcpStream {}

pub fn make_connection(
    address: &SocketAddr,
    url: &Url,
    ssl: bool,
) -> Result<Box<dyn GenericStream>, Box<dyn Error>> {
    let stream = TcpStream::connect_timeout(&address, Duration::from_micros(3_000_000))?;
    debug!("TCP connected");
    let _r = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let _r = stream.set_read_timeout(Some(Duration::from_secs(3)));
    if !ssl {
        return Ok(Box::new(stream));
    }

    let connector = SslConnector::builder(SslMethod::tls()).unwrap().build();
    let ssl_stream = connector.connect(url.host_str().unwrap(), stream)?;
    debug!("SSL connected");
    Ok(Box::new(ssl_stream))
}

pub fn request_tcp_ping(address: &SocketAddr) -> Result<u128, Box<dyn Error>> {
    let now = Instant::now();
    let r = TcpStream::connect_timeout(&address, Duration::from_micros(1_000_000));
    let used = now.elapsed().as_micros();
    let used = match r {
        Ok(_) => used,
        Err(e) => {
            debug!("Ping {e}");
            1_000_000
        }
    };
    Ok(used)
}

pub fn request_http_download(
    address: SocketAddr,
    url: Url,
    connection_close: bool,
    ssl: bool,
) -> Result<f64, Box<dyn Error>> {
    let mut counter = 0u128;
    let chunk_count = if connection_close {
        debug!("Enter connection close mode");
        15_000
    } else {
        25
    };
    let data_size = chunk_count * 1024 * 1024 as u128;
    let mut data_counter = data_size;
    let mut buffer = [0; 16384];

    let host_port = format!(
        "{}:{}",
        url.host_str().unwrap(),
        url.port_or_known_default().unwrap()
    );
    let path_str = url.path();

    let mut stream = make_connection(&address, &url, ssl)?;

    let now = Instant::now();
    let mut time_used = 0;
    while time_used < 14_500_000 {
        if data_counter >= data_size {
            let rd = random::<f64>().to_string();
            let path_query = format!(
                "{}?cors=true&r={}&ckSize={}&size={}",
                path_str, rd, chunk_count, data_size
            );
            debug!("Download {path_query}");

            let request_head = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: bim/1.0\r\n\r\n",
                path_query, host_port,
            )
            .into_bytes();

            let r = stream.write_all(&request_head);
            match r {
                Ok(_) => {
                    data_counter = 0;
                    debug!("Downloaded: {}", counter);
                }
                Err(e) => {
                    debug!("Download Error: {}", e);
                    return Err(Box::new(e));
                }
            }
        } else {
            let _r = stream.read_exact(&mut buffer);
            counter += 16384;
            data_counter += 16384;
        }
        time_used = now.elapsed().as_micros();
    }

    let r = (counter * 8) as f64 / time_used as f64;
    debug!("Downloaded {counter} bytes in {time_used} us, speed {r}");

    Ok(r)
}

pub fn request_http_upload(
    address: SocketAddr,
    url: Url,
    connection_close: bool,
    ssl: bool,
) -> Result<f64, Box<dyn Error>> {
    let mut counter = 0u128;
    let chunk_count = if connection_close {
        debug!("Enter connection close mode");
        15_000
    } else {
        25
    };
    let data_size = chunk_count * 1024 * 1024 as u128;
    let mut data_counter = data_size;

    let host_port = format!(
        "{}:{}",
        url.host_str().unwrap(),
        url.port_or_known_default().unwrap()
    );
    let url_path = url.path();
    let request_chunk = "0123456789AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz-="
        .repeat(256)
        .into_bytes();

    let mut stream = make_connection(&address, &url, ssl)?;
    let mut time_used = 0;

    let now = Instant::now();
    while time_used < 14_500_000 {
        if data_counter >= data_size {
            let rd = random::<f64>().to_string();
            let path_query = format!("{}?r={}", url_path, rd);
            debug!("Upload {path_query} size {data_size}");

            let request_head = format!(
                "POST {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: bim/1.0\r\nContent-Length: {}\r\n\r\n",
                path_query, host_port, data_size
            )
            .into_bytes();

            let r = stream.write_all(&request_head);
            match r {
                Ok(_) => {
                    counter += request_head.len() as u128;
                    data_counter = 0;
                    debug!("Uploaded: {}", counter);
                }
                Err(e) => {
                    debug!("Upload Error: {}", e);
                    return Err(Box::new(e));
                }
            }
        } else {
            let _r = stream.write_all(&request_chunk);
            counter += 16384;
            data_counter += 16384;
        }
        time_used = now.elapsed().as_micros();
    }

    let r = (counter * 8) as f64 / time_used as f64;
    debug!("Uploaded {counter} bytes in {time_used} us, speed {r}");
    Ok(r)
}
