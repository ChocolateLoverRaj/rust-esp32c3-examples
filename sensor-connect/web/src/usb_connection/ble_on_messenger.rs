use common::{CommandData, GetSet, Message, MessageToEsp, ResponseData};

use crate::usb_connection::usb_characteristic_messenger::UsbCharacteristicMessenger;

#[derive(Clone)]
pub struct BleOnMessenger;

impl UsbCharacteristicMessenger<bool> for BleOnMessenger {
    fn create_get_request() -> CommandData {
        CommandData::BleOn(GetSet::Get)
    }

    fn find_get_response(response_data: ResponseData) -> Option<bool> {
        match response_data {
            ResponseData::GetBleOn(value) => Some(value),
            _ => None,
        }
    }

    fn create_subscribe_request() -> Option<MessageToEsp> {
        None
    }

    fn create_unsubscribe_request() -> Option<MessageToEsp> {
        None
    }

    fn match_event(value: Message) -> bool {
        match value {
            Message::BleOnChange => true,
            _ => false
        }
    }

    fn create_set_request(value: bool) -> CommandData {
        CommandData::BleOn(GetSet::Set(value))
    }
}
