use core::cell::RefCell;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use habits_stepper::application::Configuration;

pub(super) static CONFIGURATION_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static REQUESTS: Mutex<
    CriticalSectionRawMutex,
    RefCell<heapless::Vec<heapless::Vec<u8, 1024>, 8>>,
> = Mutex::new(RefCell::new(heapless::Vec::new()));

pub fn take_requests() -> heapless::Vec<heapless::Vec<u8, 1024>, 8> {
    REQUESTS.lock(|requests| core::mem::take(&mut *requests.borrow_mut()))
}

#[embassy_executor::task]
pub async fn run_sync_totals_task(configuration: &'static Configuration) {
    if !configuration.is_backend_configured() {
        return;
    }
    CONFIGURATION_READY.wait().await;
    let tcp = Tcp;
    let dns = Dns;
    let mut client = reqwless::client::HttpClient::new(&tcp, &dns);
    habits_stepper::sync::sync_totals_to_backend(&mut client, configuration).await;
}

#[derive(Debug)]
pub struct NetworkError;

impl core::fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("mock network error")
    }
}

impl core::error::Error for NetworkError {}

impl embedded_io_async::Error for NetworkError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Other
    }
}

pub struct Dns;

impl embedded_nal_async::Dns for Dns {
    type Error = NetworkError;

    fn get_host_by_name(
        &self,
        _host: &str,
        _address_type: embedded_nal_async::AddrType,
    ) -> impl core::future::Future<Output = Result<core::net::IpAddr, Self::Error>> {
        core::future::ready(Ok(core::net::IpAddr::V4(core::net::Ipv4Addr::LOCALHOST)))
    }

    fn get_host_by_address(
        &self,
        _address: core::net::IpAddr,
        _result: &mut [u8],
    ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
        core::future::ready(Err(NetworkError))
    }
}

pub struct Tcp;

pub struct Connection {
    position: usize,
    request: heapless::Vec<u8, 1024>,
    recorded: bool,
}

impl embedded_nal_async::TcpConnect for Tcp {
    type Error = NetworkError;
    type Connection<'connection> = Connection;

    fn connect(
        &self,
        _remote: core::net::SocketAddr,
    ) -> impl core::future::Future<Output = Result<Self::Connection<'_>, Self::Error>> {
        core::future::ready(Ok(Connection {
            position: 0,
            request: heapless::Vec::new(),
            recorded: false,
        }))
    }
}

impl embedded_io_async::ErrorType for Connection {
    type Error = NetworkError;
}

impl embedded_io_async::Read for Connection {
    fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
        if !self.recorded {
            REQUESTS.lock(|requests| {
                requests
                    .borrow_mut()
                    .push(self.request.clone())
                    .expect("backend request capacity exceeded");
            });
            self.recorded = true;
        }
        let response = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";
        let remaining = &response[self.position..];
        let count = remaining.len().min(buffer.len());
        buffer[..count].copy_from_slice(&remaining[..count]);
        self.position += count;
        core::future::ready(Ok(count))
    }
}

impl embedded_io_async::Write for Connection {
    fn write(
        &mut self,
        buffer: &[u8],
    ) -> impl core::future::Future<Output = Result<usize, Self::Error>> {
        self.request
            .extend_from_slice(buffer)
            .expect("backend request too long");
        core::future::ready(Ok(buffer.len()))
    }

    fn flush(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        core::future::ready(Ok(()))
    }
}
