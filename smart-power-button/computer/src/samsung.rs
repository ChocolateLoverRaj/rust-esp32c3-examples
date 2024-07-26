use anyhow::anyhow;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE},
    Engine as _,
};
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message, Connector};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct App {
    #[serde(rename = "appId")]
    app_id: String,
    app_type: u8,
    icon: String,
    is_lock: u8,
    name: String,
}

pub struct Samsung {
    pub ip: String,
    pub app_name: String,
    pub token: Option<String>,
}

impl Samsung {
    async fn send_request<Req: Serialize>(
        &self,
        request: &Req,
        event: &str,
    ) -> anyhow::Result<Value> {
        let (mut ws, _) = connect_async_tls_with_config(
            {
                let ip = &self.ip;
                let mut url = Url::parse(&format!(
                    "wss://{ip}:8002//api/v2/channels/samsung.remote.control"
                ))?;
                url.query_pairs_mut()
                    .append_pair("name", &URL_SAFE.encode(&self.app_name));
                if let Some(token) = self.token.as_ref() {
                    url.query_pairs_mut().append_pair("token", token);
                }
                url.to_string()
            },
            None,
            false,
            Some(Connector::NativeTls(
                TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()?,
            )),
        )
        .await?;
        ws.send(Message::Text(serde_json::to_string(request)?))
            .await?;
        Ok(loop {
            if let Message::Text(message) = ws
                .next()
                .await
                .ok_or(anyhow!("Didn't receive correct event"))??
            {
                #[derive(Serialize, Deserialize, Debug)]
                struct ExpectedResponse {
                    event: String,
                    data: Value,
                }
                let message = serde_json::from_str::<ExpectedResponse>(&message)?;
                if message.event == event {
                    break message.data;
                }
            }
        })
    }

    /// Returns `true` if the token was set / updated
    pub async fn send_key(&mut self, key: &str) -> anyhow::Result<bool> {
        let response = self
            .send_request(
                &json!({
                    "method": "ms.remote.control",
                    "params": {
                        "Cmd": "Click",
                        "DataOfCmd": key,
                        "Option": "false",
                        "TypeOfRemote": "SendRemoteKey",
                    }
                }),
                "ms.channel.connect",
            )
            .await?;
        #[derive(Serialize, Deserialize, Debug)]
        struct ExpectedResponseData {
            token: Option<String>,
        }
        let response = serde_json::from_value::<ExpectedResponseData>(response)?;
        if let Some(token) = response.token {
            self.token = Some(token);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn send_text(&self, text: &str) -> anyhow::Result<()> {
        self.send_request(
            &json!({
                "method": "ms.remote.control",
                "params": {
                    "Cmd": STANDARD.encode(text),
                    "DataOfCmd": "base64",
                    "TypeOfRemote": "SendInputString"
                }
            }),
            "ms.channel.connect",
        )
        .await?;
        Ok(())
    }

    pub async fn get_apps_from_tv(&self) -> anyhow::Result<Vec<App>> {
        let response = self
            .send_request(
                &json!({
                    "method": "ms.channel.emit",
                    "params": {
                        "data": "",
                        "event": "ed.installedApp.get",
                        "to": "host",
                    },
                }),
                "ed.installedApp.get",
            )
            .await?;

        #[derive(Serialize, Deserialize, Debug)]
        struct ResponseData {
            data: Vec<App>,
        }
        let response = serde_json::from_value::<ResponseData>(response)?;
        Ok(response.data)
    }

    pub async fn open_app(&self, app_id: &str) -> anyhow::Result<()> {
        self.send_request(
            &json!({
                "method": "ms.channel.emit",
                "params": {
                    "data": {
                        "action_type": "DEEP_LINK",
                        "appId": app_id
                    },
                    "event": "ed.apps.launch",
                    "to": "host"
                }
            }),
            "ms.channel.connect",
        )
        .await?;
        Ok(())
    }
}
