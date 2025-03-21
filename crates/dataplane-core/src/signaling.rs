use std::collections::HashMap;

use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{formats::PreferMany, serde_as, OneOrMany};

#[derive(Debug, Serialize, Deserialize, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowStartMessage {
    agreement_id: String,
    dataset_id: String,
    pub participant_id: String,
    pub process_id: String,
    flow_type: FlowType,
    properties: HashMap<String, Value>,
    pub source_data_address: DataAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlowType {
    Pull,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowResponseMessage {
    pub data_address: Option<DataAddress>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowTerminateMessage {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataFlowSuspendMessage {
    pub reason: Option<String>,
}

impl DataFlowTerminateMessage {}

impl DataFlowResponseMessage {
    pub fn new(data_address: Option<DataAddress>) -> Self {
        Self { data_address }
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Builder, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DataAddress {
    #[serde(rename = "dspace:endpointType")]
    pub endpoint_type: String,
    #[serde_as(deserialize_as = "OneOrMany<_, PreferMany>")]
    #[serde(rename = "dspace:endpointProperties")]
    pub endpoint_properties: Vec<EndpointProperty>,
}

impl DataAddress {
    pub fn get_property(&self, name: &str) -> Option<&str> {
        self.endpoint_properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.value.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder, PartialEq)]
pub struct EndpointProperty {
    #[serde(rename = "dspace:name")]
    #[builder(into)]
    pub name: String,
    #[serde(rename = "dspace:value")]
    #[builder(into)]
    pub value: String,
}
