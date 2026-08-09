use core::fmt::Write as _;

use defmt::{error, info};
use embassy_net::Stack;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_time::{Duration, with_timeout};
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::headers::ContentType;
use reqwless::request::{Method, RequestBuilder};

use crate::sessions::DailyTotal;
use crate::time::date_bytes;

const TLS_SEED: u64 = 0x53_59_4E_43;
const METRICS_URL: &str = env!("METRICS_URL");
const AUTH_HEADER: &str = concat!("Bearer ", env!("JWT_TOKEN"));
const CHAIN_ID: &str = env!("CHAIN_ID");

#[embassy_executor::task]
pub async fn sync_task(stack: Stack<'static>) {
    info!("sync: task started (chain_id={})", CHAIN_ID);
    stack.wait_config_up().await;

    let state = TcpClientState::<1, 4096, 4096>::new();
    let tcp_client = TcpClient::new(stack, &state);
    let dns_socket = DnsSocket::new(stack);

    let mut tls_read = [0u8; 4096];
    let mut tls_write = [0u8; 4096];
    let mut client = HttpClient::new_with_tls(
        &tcp_client,
        &dns_socket,
        TlsConfig::new(TLS_SEED, &mut tls_read, &mut tls_write, TlsVerify::None),
    );

    loop {
        let totals = crate::SYNC_SIGNAL.wait().await;
        for total in totals {
            post_total(&mut client, &total).await;
        }
    }
}

pub async fn post_total<'a, T, D>(client: &mut HttpClient<'a, T, D>, total: &DailyTotal) -> bool
where
    T: embedded_nal_async::TcpConnect + 'a,
    D: embedded_nal_async::Dns + 'a,
{
    let bytes = date_bytes(total.date);
    let date = core::str::from_utf8(&bytes).unwrap();

    let mut body: heapless::Vec<u8, 128> = heapless::Vec::new();
    let _ = write!(
        body,
        "{{\"date\":\"{}\",\"value\":\"{}\",\"chain_id\":{}}}",
        date, total.minutes, CHAIN_ID
    );

    let mut rx_buf = [0u8; 512];
    let auth_headers = [("authorization", AUTH_HEADER)];

    let status = match with_timeout(Duration::from_secs(30), async {
        client
            .request(Method::POST, METRICS_URL)
            .await?
            .headers(&auth_headers)
            .content_type(ContentType::ApplicationJson)
            .body(body.as_slice())
            .send(&mut rx_buf)
            .await
            .map(|response| response.status.0)
    })
    .await
    {
        Ok(Ok(status)) => Some(status),
        _ => None,
    };

    if status == Some(200) {
        info!(
            "sync: post succeeded status=200 date={} minutes={} chain_id={}",
            date, total.minutes, CHAIN_ID
        );
        true
    } else {
        error!(
            "sync: post failed status={:?} date={} minutes={} chain_id={}",
            status, date, total.minutes, CHAIN_ID
        );
        false
    }
}
