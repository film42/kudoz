KUDOZ!
======

Sometimes you just want to get a pat on the back after you deploy something awesome.

I wrote this operator to present to the UT Rust meetup in 2023. Feel free to use this as a refernce but know that I do not plan to keep this up to date.

### Example

I use this yaml example when demoing which sends a webhook to slack when a deployment completes.

```yaml
kind: SuperKudo
apiVersion: kudoz.desh.es/v1
metadata:
  name: super-fun-demo
  namespace: default
spec:
  selector:
    labels:
      app: nginx
  deliverTo:
    slack: https://hooks.slack.com/services/XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```


---


License: MIT
