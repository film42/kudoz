mod utils;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::ListParams,
    client::Client,
    runtime::{
        controller::{Action, Controller},
        watcher,
        watcher::Event,
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
    generation_cache: Arc<Mutex<BTreeMap<String, i64>>>,
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
                "ERROR: Sending a super kudo to {:?}, received error: {:?}",
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
        .try_for_each(|event| {
            let ctx = ctx.clone();
            async move {
                // Avoid using the build-in applied_objects() method to avoid the Event::Restarted
                // records which show up on start. We only want to subscribe to new changes going
                // forward.
                if let Event::Applied(deployment) = event {
                    if deployment.finished_deploying() {
                        if let Some(generation) = deployment.metadata.generation {
                            let mut generation_cache =
                                ctx.generation_cache.lock().expect("mutex failed");

                            // If the generation cache already has this same generation, then skip it
                            // for now.
                            if generation_cache.insert(deployment.namespaced_name(), generation)
                                == Some(generation)
                            {
                                return Ok(());
                            }
                        }

                        on_deployment_completed(deployment, ctx.clone())
                            .await
                            .unwrap();
                    }
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
        generation_cache: Arc::new(Mutex::new(BTreeMap::new())),
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
