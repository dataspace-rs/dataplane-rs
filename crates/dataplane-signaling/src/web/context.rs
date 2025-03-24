use edc_dataplane_core::{
    core::model::namespace::{DSPACE_NAMESPACE, EDC_NAMESPACE},
    signaling::DataFlowResponseMessage,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize, Debug)]
pub struct WithContext<T> {
    #[allow(dead_code)]
    #[serde(rename = "@context")]
    context: Value,
    #[allow(dead_code)]
    #[serde(rename = "@type")]
    kind: Option<String>,
    #[serde(flatten)]
    pub(crate) inner: T,
}

impl<T: TypedObject> WithContext<T> {
    pub fn builder(input: T) -> ContextBuilder<T> {
        ContextBuilder {
            inner: input,
            context: default_context(),
            kind: Some(T::get_type().to_string()),
        }
    }
}

pub fn default_context() -> Value {
    json!({ "@vocab": EDC_NAMESPACE.ns(), "dspace": DSPACE_NAMESPACE.ns() })
}

pub struct ContextBuilder<T> {
    inner: T,
    context: Value,
    kind: Option<String>,
}

impl<T> ContextBuilder<T> {
    pub fn build(self) -> anyhow::Result<WithContext<T>> {
        if self.context == Value::Null {
            anyhow::bail!("@context missing");
        }

        Ok(WithContext {
            context: self.context,
            inner: self.inner,
            kind: self.kind,
        })
    }
}

pub trait TypedObject {
    fn get_type() -> &'static str;
}

impl TypedObject for DataFlowResponseMessage {
    fn get_type() -> &'static str {
        "DataFlowResponseMessage"
    }
}
