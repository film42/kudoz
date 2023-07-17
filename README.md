KUDOZ!
======

Sometimes you just want to get a pat on the back after you deploy something awesome.

I wrote this operator to present to the UT Rust meetup in 2023. Feel free to use this as a refernce but know that I do not plan to keep this up to date.

### Demo!

First, load the `SuperKudo` CRD into your cluster and list your crds to make sure it registered successfully.

```
$ kubectl apply -f superkudos.kudoz.desh.es.yaml 
customresourcedefinition.apiextensions.k8s.io/superkudos.kudoz.desh.es configured

$ kubectl get crds                              
NAME                              CREATED AT
...
superkudos.kudoz.desh.es          2023-07-14T00:04:38Z
```

Next, load the example `SuperKudo` resource into your cluster in the default namespace.

```
$ cat superkudo.example.yaml    
kind: SuperKudo
apiVersion: kudoz.desh.es/v1
metadata:
  name: super-fun-example
  namespace: default
spec:
  selector:
    labels:
      app: nginx
  deliverTo:
    slack: http://localhost:8000/test/my/hook

$ kubectl apply -f superkudo.example.yaml       
superkudo.kudoz.desh.es/super-fun-example created
```

Then, make sure you have a deployment that matches the labels `app: nginx`. There is an example one in the `fixtures/` directory.

```
$ kubectl apply -f fixtures/deployment.example.yaml 
deployment.apps/nginx-deployment-example created
```

And finally, start the kudoz operator. In the output below I swapped the `.spec.deliverTo.slack` URL to a real one. The above is totally fine for testing, though.

```
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.35s
     Running `target/debug/kudoz-controller`

Sending a kudo to Some("super-fun-demo") because Some("nginx-deployment-example") finished deploying!

reconciled (ObjectRef { dyntype: (), name: "super-fun-demo", namespace: Some("default"), extra: Extra { resource_version: Some("1787"), uid: Some("6a36a7dc-441e-4904-ba1c-8eb0873f0c9d") } }, Action { requeue_after: None })
```

Nice! Looked like it all worked!


### License

License: MIT
