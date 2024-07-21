use dotenvy_macro::dotenv;
use esp_idf_svc::ipv4::IpInfo;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi};
use log::info;

const WIFI_SSID: &str = dotenv!("WIFI_SSID");
const WIFI_PASSWORD: &str = dotenv!("WIFI_PASS");

pub struct WifiLoop<'a> {
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiLoop<'a> {
    pub fn new(wifi: AsyncWifi<EspWifi<'a>>) -> Self {
        Self { wifi }
    }

    pub async fn configure(&mut self) -> Result<(), EspError> {
        info!("Setting Wi-Fi credentials...");
        let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
            ssid: WIFI_SSID.parse().unwrap(),
            password: WIFI_PASSWORD.parse().unwrap(),
            auth_method: AuthMethod::WPA2Personal,
            channel: None,
            ..Default::default()
        });
        self.wifi.set_configuration(&wifi_configuration)?;

        info!("Starting Wi-Fi driver...");
        self.wifi.start().await
    }

    pub async fn initial_connect(&mut self) -> Result<(), EspError> {
        self.do_connect_loop(true).await
    }

    pub fn get_ip_info(&self) -> (IpInfo, heapless::String<30>) {
        let netif = self.wifi.wifi().sta_netif();
        (netif.get_ip_info().unwrap(), netif.get_hostname().unwrap())
    }

    pub async fn stay_connected(mut self) -> Result<(), EspError> {
        self.do_connect_loop(false).await
    }

    async fn do_connect_loop(&mut self, exit_after_first_connect: bool) -> Result<(), EspError> {
        let wifi = &mut self.wifi;
        loop {
            // Wait for disconnect before trying to connect again.  This loop ensures
            // we stay connected and is commonly missing from trivial examples as it's
            // way too difficult to showcase the core logic of an example and have
            // a proper Wi-Fi event loop without a robust async runtime.  Fortunately, we can do it
            // now!
            wifi.wifi_wait(|this| this.is_up(), None).await?;

            info!("Connecting to Wi-Fi...");
            wifi.connect().await?;

            info!("Waiting for association...");
            wifi.ip_wait_while(|this| this.is_up().map(|s| !s), None)
                .await?;

            if exit_after_first_connect {
                return Ok(());
            }
        }
    }
}
