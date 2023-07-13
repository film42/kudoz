use kube::CustomResourceExt;
use kudoz_crd::SuperKudo;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let crd = SuperKudo::crd();

    // Write to file.
    let schema = serde_yaml::to_string(&crd).unwrap();
    let crate_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let crd_schema_path = Path::new(&crate_dir)
        .join("..")
        .join("superkudos.kudoz.desh.es.yaml");
    let mut f = File::create(&crd_schema_path).unwrap();
    f.write_all(schema.as_bytes()).unwrap();
}
