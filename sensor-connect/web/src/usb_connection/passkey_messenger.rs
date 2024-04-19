use common::{CommandData, GetSet, ResponseData};

use crate::usb_connection::usb_characteristic_messenger::UsbCharacteristicMessenger;

#[derive(Clone)]
pub struct PasskeyMessenger;

impl UsbCharacteristicMessenger<u32> for PasskeyMessenger {
    fn create_get_request() -> CommandData {
        CommandData::Passkey(common::GetSet::Get)
    }

    fn find_get_response(response_data: ResponseData) -> Option<u32> {
        match response_data {
            common::ResponseData::GetPasskey(passkey) => Some(passkey),
            _ => None,
        }
    }

    fn create_set_request(value: u32) -> CommandData {
        CommandData::Passkey(GetSet::Set(value))
    }
}
