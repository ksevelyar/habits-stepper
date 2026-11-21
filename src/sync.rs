use core::fmt::Write as _;

use defmt::{error, info};
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, with_timeout};
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::headers::ContentType;
use reqwless::request::{Method, RequestBuilder};
use static_cell::ConstStaticCell;

use crate::application::Configuration;
use crate::sessions::DailyTotal;
use crate::time::format_date_bytes;

const TLS_SEED: u64 = 0x53_59_4E_43;

pub static SYNC_SIGNAL: Signal<CriticalSectionRawMutex, [DailyTotal; 2]> = Signal::new();

#[embassy_executor::task]
pub async fn run_sync_totals_task(
    stack: embassy_net::Stack<'static>,
    configuration: &'static Configuration,
) {
    static TCP_STATE: ConstStaticCell<TcpClientState<1, 4096, 4096>> =
        ConstStaticCell::new(TcpClientState::new());
    static TLS_READ_BUFFER: ConstStaticCell<[u8; 4096]> = ConstStaticCell::new([0; 4096]);
    static TLS_WRITE_BUFFER: ConstStaticCell<[u8; 4096]> = ConstStaticCell::new([0; 4096]);
    if !configuration.is_backend_configured() {
        info!("sync: disabled (backend is not configured)");
        return;
    }
    stack.wait_config_up().await;
    let tcp_client = TcpClient::new(stack, TCP_STATE.take());
    let dns_socket = DnsSocket::new(stack);
    let mut client = HttpClient::new_with_tls(
        &tcp_client,
        &dns_socket,
        TlsConfig::new(
            TLS_SEED,
            TLS_READ_BUFFER.take(),
            TLS_WRITE_BUFFER.take(),
            TlsVerify::None,
        ),
    );
    sync_totals_to_backend(&mut client, configuration).await;
}

pub async fn sync_totals_to_backend<'a, Tcp, Dns>(
    client: &mut HttpClient<'a, Tcp, Dns>,
    configuration: &Configuration,
) where
    Tcp: embedded_nal_async::TcpConnect + 'a,
    Dns: embedded_nal_async::Dns + 'a,
{
    static AUTHORIZATION: ConstStaticCell<heapless::String<512>> =
        ConstStaticCell::new(heapless::String::new());
    static RESPONSE_BUFFER: ConstStaticCell<[u8; 512]> = ConstStaticCell::new([0; 512]);
    let (Some(_metrics_url), Some(jwt_token), Some(chain_id)) = (
        configuration.metrics_url,
        configuration.jwt_token,
        configuration.chain_id,
    ) else {
        info!("sync: disabled (backend is not configured)");
        return;
    };
    let authorization = AUTHORIZATION.take();
    if write!(authorization, "Bearer {jwt_token}").is_err() {
        error!("sync: authorization header is too long");
        return;
    }

    info!("sync: task started (chain_id={})", chain_id);
    let response_buffer = RESPONSE_BUFFER.take();
    loop {
        let totals = SYNC_SIGNAL.wait().await;
        for total in totals {
            post_total(
                client,
                configuration,
                authorization.as_str(),
                &total,
                response_buffer,
            )
            .await;
        }
    }
}

pub async fn post_total<'a, T, D>(
    client: &mut HttpClient<'a, T, D>,
    configuration: &Configuration,
    authorization: &str,
    total: &DailyTotal,
    response_buffer: &mut [u8],
) -> bool
where
    T: embedded_nal_async::TcpConnect + 'a,
    D: embedded_nal_async::Dns + 'a,
{
    let (Some(metrics_url), Some(chain_id)) = (configuration.metrics_url, configuration.chain_id)
    else {
        return false;
    };

    let bytes = format_date_bytes(total.date);
    let date = core::str::from_utf8(&bytes)
        .expect("formatted date contains only ASCII digits and hyphens");

    let mut body: heapless::Vec<u8, 128> = heapless::Vec::new();
    let _ = write!(
        body,
        "{{\"date\":\"{}\",\"value\":\"{}\",\"chain_id\":{}}}",
        date, total.minutes, chain_id
    );

    let auth_headers = [("authorization", authorization)];

    let status = match with_timeout(Duration::from_secs(30), async {
        client
            .request(Method::POST, metrics_url)
            .await?
            .headers(&auth_headers)
            .content_type(ContentType::ApplicationJson)
            .body(body.as_slice())
            .send(response_buffer)
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
            date, total.minutes, chain_id
        );
        true
    } else {
        error!(
            "sync: post failed status={:?} date={} minutes={} chain_id={}",
            status, date, total.minutes, chain_id
        );
        false
    }
}
