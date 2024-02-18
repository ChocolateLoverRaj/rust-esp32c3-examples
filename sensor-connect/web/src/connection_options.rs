use leptos::{component, view, Callback, IntoView};

use crate::{
    ble_connection::BleConnectionBuilder, connection::ConnectionBuilder,
    connection_type::ConnectionType, usb_connection::UsbConnectionBuilder,
};

#[component]
pub fn ConnectionOptions(on_click_connect: Callback<ConnectionType>) -> impl IntoView {
    view! {
        <h1>SensorConnect</h1>
        <p>Connect to an ESP32-C3 Super-Mini to read and graph sensor data</p>
        <h1>Connection options</h1>
        <table>
            <tbody>
                <tr>
                    <th>Connection type</th>
                    <td>USB</td>
                    <td>BLE (Bluetooth Low Energy)</td>
                </tr>
                <tr>
                    <th>Chromebook</th>
                    <td>Yes (Chromium)</td>
                    <td>Yes (Chromium)</td>
                </tr>
                <tr>
                    <th>Android</th>
                    <td>No</td>
                    <td>Yes (Chromium)</td>
                </tr>
                <tr>
                    <th>iPhone</th>
                    <td>No</td>
                    <td>
                        Yes
                        <a href="https://apps.apple.com/us/app/bluefy-web-ble-browser/id1492822055">
                            Blueify
                        </a>
                    </td>
                </tr>
                <tr>
                    <th>MacBook</th>
                    <td>Yes (Chromium)</td>
                    <td>Yes (Chromium)</td>
                </tr>
                <tr>
                    <th>Windows</th>
                    <td>Yes (Chromium)</td>
                    <td>Yes (Chromium)</td>
                </tr>
                <tr>
                    <th>Linux Desktop</th>
                    <td>Yes (Chromium)</td>
                    <td>Yes (Chromium)</td>
                </tr>
                <tr>
                    <th>Maximum Computers</th>
                    <td>1</td>
                    <td>9</td>
                </tr>
                <tr>
                    <th>Power Usage</th>
                    <td>Small</td>
                    <td>Medium</td>
                </tr>
                <tr>
                    <th></th>
                    <td>
                        <button
                            disabled=!UsbConnectionBuilder::is_available()
                            on:click=move |_| on_click_connect(ConnectionType::Usb)
                        >
                            Connect
                        </button>
                    </td>
                    <td>
                        <button
                            disabled=!BleConnectionBuilder::is_available()
                            on:click=move |_| on_click_connect(ConnectionType::Ble)
                        >
                            Connect
                        </button>
                    </td>
                </tr>
            </tbody>
        </table>
        <h1>About</h1>
        <a href=env!("CARGO_PKG_REPOSITORY")>GitHub Repository</a>
        <br/>
        <a href=env!("CARGO_PKG_HOMEPAGE")>Source Code</a>
    }
}
