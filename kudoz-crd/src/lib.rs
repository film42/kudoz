use k8s_openapi::api::apps::v1::Deployment;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::BTreeMap;

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[kube(
    group = "kudoz.desh.es",
    version = "v1",
    kind = "SuperKudo",
    plural = "superkudos",
    derive = "PartialEq",
    namespaced
)]
pub struct SuperKudoSpec {
    pub selector: Selector,
    pub deliver_to: DeliverTo,
    pub payload: Option<JsonMap<String, JsonValue>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Selector {
    pub labels: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeliverTo {
    pub slack: String,
}

impl SuperKudo {
    pub fn namespaced_name(&self) -> String {
        format!(
            "{}/{}",
            self.metadata.namespace.as_ref().expect("always defined"),
            self.metadata.name.as_ref().expect("always defined"),
        )
    }

    pub fn does_target_deployment(&self, deployment: &Deployment) -> bool {
        if let Some(ref labels) = deployment.metadata.labels {
            return self
                .spec
                .selector
                .labels
                .iter()
                .all(|(key, value)| labels.get(key) == Some(value));
        }

        false
    }

    fn payload(&self, deployment: &Deployment) -> JsonValue {
        use serde_json::json;

        if let Some(ref p) = self.spec.payload {
            return JsonValue::Object(p.clone());
        } else {
            let text = format!(
                "Congrats! You just finished deploying {}!",
                deployment
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "<unknown>".to_string())
            );
            json!({ "text": text })
        }
    }

    pub async fn send_super_kudo(
        &self,
        deployment: &Deployment,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        client
            .post(&self.spec.deliver_to.slack)
            .json(&self.payload(&deployment))
            .send()
            .await?;

        Ok(())
    }
}
