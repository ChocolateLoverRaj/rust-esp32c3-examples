use common::{CommandData, ResponseData};

use crate::usb_connection::usb_characteristic_messenger::UsbCharacteristicMessenger;

#[derive(Clone)]
pub struct NameMessenger;

impl UsbCharacteristicMessenger<String> for NameMessenger {
    fn create_get_request() -> CommandData {
        CommandData::ShortName(common::GetSet::Get)
    }

    fn find_get_response(response_data: ResponseData) -> Option<String> {
        match response_data {
            common::ResponseData::GetShortName(name) => Some(name),
            _ => None,
        }
    }

    fn create_set_request(value: String) -> CommandData {
        CommandData::ShortName(common::GetSet::Set(value))
    }
}
