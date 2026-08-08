#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use core::cell::RefCell;
    use core::net::{IpAddr, Ipv4Addr, SocketAddr};

    use critical_section::Mutex;
    use defmt::assert;
    use embedded_io_async::{ErrorType, Read, Write};
    use embedded_nal_async::AddrType;
    use habits_stepper::sessions::DailyTotal;
    use habits_stepper::sync::post_total;
    use heapless::Vec;
    use reqwless::client::HttpClient;

    const OK_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";
    const SERVER_ERROR_RESPONSE: &[u8] =
        b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n";

    static WRITTEN: Mutex<RefCell<Vec<u8, 2048>>> = Mutex::new(RefCell::new(Vec::new()));

    #[derive(Debug)]
    struct TestError;

    impl core::fmt::Display for TestError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "TestError")
        }
    }

    impl core::error::Error for TestError {}

    impl embedded_io_async::Error for TestError {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            embedded_io_async::ErrorKind::Other
        }
    }

    struct FakeDns;

    impl embedded_nal_async::Dns for FakeDns {
        type Error = TestError;

        async fn get_host_by_name(
            &self,
            _host: &str,
            _addr_type: AddrType,
        ) -> Result<IpAddr, Self::Error> {
            Ok(IpAddr::V4(Ipv4Addr::LOCALHOST))
        }

        async fn get_host_by_address(
            &self,
            _addr: IpAddr,
            _result: &mut [u8],
        ) -> Result<usize, Self::Error> {
            Err(TestError)
        }
    }

    struct FakeTcp {
        response: &'static [u8],
    }

    impl embedded_nal_async::TcpConnect for FakeTcp {
        type Error = TestError;
        type Connection<'m> = FakeConnection<'m>;

        async fn connect<'m>(
            &'m self,
            _remote: SocketAddr,
        ) -> Result<Self::Connection<'m>, Self::Error> {
            Ok(FakeConnection {
                tcp: self,
                read_pos: 0,
            })
        }
    }

    struct FakeConnection<'m> {
        tcp: &'m FakeTcp,
        read_pos: usize,
    }

    impl ErrorType for FakeConnection<'_> {
        type Error = TestError;
    }

    impl Read for FakeConnection<'_> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TestError> {
            let available = &self.tcp.response[self.read_pos..];
            let count = core::cmp::min(buf.len(), available.len());
            buf[..count].copy_from_slice(&available[..count]);
            self.read_pos += count;
            Ok(count)
        }
    }

    impl Write for FakeConnection<'_> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, TestError> {
            critical_section::with(|cs| {
                let _ = WRITTEN.borrow_ref_mut(cs).extend_from_slice(buf);
            });
            Ok(buf.len())
        }

        async fn flush(&mut self) -> Result<(), TestError> {
            Ok(())
        }
    }

    #[init]
    fn init() {
        let peripherals = esp_hal::init(esp_hal::Config::default());
        let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
        rtt_target::rtt_init_defmt!();
    }

    fn report() -> DailyTotal {
        let date = jiff::civil::date(2026, 8, 9)
            .at(14, 30, 0, 0)
            .to_zoned(habits_stepper::time::TIMEZONE)
            .unwrap()
            .timestamp()
            .as_second() as u32;
        DailyTotal { date, minutes: 125 }
    }

    async fn post_to_fake_backend(response: &'static [u8]) -> bool {
        critical_section::with(|cs| WRITTEN.borrow_ref_mut(cs).clear());

        let tcp = FakeTcp { response };
        let dns = FakeDns;
        let mut client = HttpClient::new(&tcp, &dns);
        post_total(&mut client, &report()).await
    }

    #[test]
    #[timeout(5)]
    async fn posts_metric_and_reports_success_on_200() {
        assert!(post_to_fake_backend(OK_RESPONSE).await);

        let written = critical_section::with(|cs| WRITTEN.borrow_ref(cs).clone());
        let request = core::str::from_utf8(written.as_slice()).unwrap();

        assert!(request.contains("POST /metrics HTTP/1.1"));
        assert!(request.contains("authorization: Bearer "));
        assert!(request.contains("Content-Type: application/json"));
        assert!(request.contains("Content-Length: "));
        assert!(request.contains("\"date\":\"2026-08-09\""));
        assert!(request.contains("\"value\":\"125\""));
        assert!(request.contains("\"chain_id\":"));
    }

    #[test]
    #[timeout(5)]
    async fn reports_failure_on_500() {
        assert!(!post_to_fake_backend(SERVER_ERROR_RESPONSE).await);
    }
}
