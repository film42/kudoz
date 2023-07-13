mod utils;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::ListParams,
    client::Client,
    //core::WatchEvent,
    runtime::{
        controller::{Action, Controller},
        watcher, WatchStreamExt,
    },
    Api,
};
use kudoz_crd::SuperKudo;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use utils::DeploymentExt;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown")]
    Unknown,
}

// Context and state holder of the operator. We'll pass this around to lots of places.
struct Ctx {
    client: Client,
    super_kudos: Arc<Mutex<BTreeMap<String, SuperKudo>>>,
}

// Add and remove super kudos to our super kudos cache. This makes it easier
// to quickly match deployments when they finish.
async fn reconcile(super_kudo: Arc<SuperKudo>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let mut super_kudos = ctx.super_kudos.lock().unwrap();

    if super_kudo.metadata.deletion_timestamp.is_some() {
        super_kudos.remove(&super_kudo.namespaced_name());
    } else {
        let sk = (*super_kudo).clone();
        super_kudos.insert(sk.namespaced_name(), sk);
    }

    Ok(Action::await_change())
}

fn error_policy(_super_kudo: Arc<SuperKudo>, _error: &Error, _ctx: Arc<Ctx>) -> Action {
    Action::requeue(Duration::from_secs(1))
}

async fn on_deployment_completed(
    deployment: Deployment,
    ctx: Arc<Ctx>,
) -> Result<(), Box<dyn std::error::Error>> {
    // HACK: Cloning to skirt around the awkward async not send problem.
    let super_kudos = ctx.super_kudos.lock().expect("fix mutex deref").clone();

    for super_kudo in super_kudos
        .values()
        .filter(|super_kudo| super_kudo.does_target_deployment(&deployment))
    {
        println!(
            "Sending a kudo to {:?} because {:?} finished deploying!",
            super_kudo.metadata.name, deployment.metadata.name
        );

        if let Err(err) = super_kudo.send_super_kudo(&deployment).await {
            println!(
                "ERROR: Sending a super kudo to {}, received error: {:?}",
                super_kudo.spec.deliver_to, err
            );
        }
    }

    Ok(())
}

// Watch all deployment activity and look for ones that have finished deploying.
// Notify this operator that the deployment finished by calling the
// "on_deployment_completed" fn.
async fn watch_for_deployment_changes(ctx: Arc<Ctx>) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Only watch the most recent deployment changes, forget anything before right now.
    let deployments: Api<Deployment> = Api::all(ctx.client.clone());

    watcher(deployments, ListParams::default())
        .applied_objects()
        .try_for_each(|deployment| {
            let ctx = ctx.clone();
            async move {
                if deployment.finished_deploying() {
                    on_deployment_completed(deployment, ctx.clone())
                        .await
                        .unwrap();
                }

                Ok(())
            }
        })
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    let super_kudos: Api<SuperKudo> = Api::all(client.clone());

    let super_kudos_cache: BTreeMap<String, SuperKudo> = super_kudos
        .list(&ListParams::default())
        .await?
        .items
        .iter()
        .map(|super_kudo| (super_kudo.namespaced_name(), super_kudo.clone()))
        .collect();

    let ctx = Arc::new(Ctx {
        client,
        super_kudos: Arc::new(Mutex::new(super_kudos_cache)),
    });

    tokio::spawn({
        let ctx = ctx.clone();

        async move {
            loop {
                if let Err(err) = watch_for_deployment_changes(ctx.clone()).await {
                    println!("ERROR: While watch for deployments: {:?}", err);
                }
            }
        }
    });

    Controller::new(super_kudos, ListParams::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => println!("reconciled {:?}", o),
                Err(e) => println!("reconcile failed: {:?}", e),
            }
        })
        .await;

    Ok(())
}
