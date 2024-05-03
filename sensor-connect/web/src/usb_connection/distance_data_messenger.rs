use wasm_bindgen_test::console_log;
use common::{CommandData, GetSet, Message, MessageToEsp, ResponseData, Subscribe};
use common::distance_data::DistanceData;

use crate::usb_connection::usb_characteristic_messenger::UsbCharacteristicMessenger;

#[derive(Clone)]
pub struct DistanceDataMessenger;

impl UsbCharacteristicMessenger<DistanceData> for DistanceDataMessenger {
    fn create_get_request() -> CommandData {
        console_log!("Read distance");
        CommandData::ReadDistance
    }

    fn find_get_response(response_data: ResponseData) -> Option<DistanceData> {
        console_log!("response data: {:#?}", response_data);
        match response_data {
            ResponseData::GetDistance(value) => Some(value),
            _ => None,
        }
    }

    fn create_subscribe_request() -> Option<MessageToEsp> {
        MessageToEsp::new(CommandData::Subscribe(Subscribe::Distance)).into()
    }

    fn create_unsubscribe_request() -> Option<MessageToEsp> {
        MessageToEsp::new(CommandData::Unsubscribe(Subscribe::Distance)).into()
    }

    fn match_event(value: Message) -> bool {
        console_log!("Matching event: {:#?}", value);
        match value {
            Message::DistanceChange => true,
            _ => false
        }
    }

    fn create_set_request(value: DistanceData) -> CommandData {
        // FIXME: THis doesn't have a set request!
        CommandData::ReadDistance
    }
}
