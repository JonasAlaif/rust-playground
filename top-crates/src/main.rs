#![deny(rust_2018_idioms)]

use cargo::{
    core::{
        package::PackageSet,
        registry::PackageRegistry,
        resolver::{self, Method},
        source::SourceMap,
        Dependency, Package, PackageId, Source, SourceId, TargetKind,
    },
    sources::RegistrySource,
    util::Config,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs::File,
    io::{Read, Write},
};

/// The list of crates from crates.io
#[derive(Debug, Deserialize)]
struct TopCrates {
    crates: Vec<Crate>,
}

/// A single crate from crates.io
#[derive(Debug, Deserialize)]
struct OneCrate {
    #[serde(rename="crate")]
    krate: Crate,
}

/// The shared description of a crate
#[derive(Debug, Deserialize)]
struct Crate {
    #[serde(rename="id")]
    name: String,
}

/// A Cargo.toml file.
#[derive(Serialize)]
struct TomlManifest {
    package: TomlPackage,
    profile: Profiles,
    #[serde(serialize_with = "toml::ser::tables_last")]
    dependencies: BTreeMap<String, DependencySpec>,
}

/// Header of Cargo.toml file.
#[derive(Serialize)]
struct TomlPackage {
    name: String,
    version: String,
    authors: Vec<String>,
}

/// A mapping of a crates name to its identifier used in source code
#[derive(Debug, Serialize)]
struct CrateInformation {
    name: String,
    version: String,
    id: String,
}

/// Hand-curated changes to the crate list
#[derive(Debug, Deserialize)]
struct Modifications {
    #[serde(default)]
    blacklist: Vec<String>,
    #[serde(default)]
    additions: BTreeSet<String>,
}

/// A profile section in a Cargo.toml file
#[derive(Serialize)]
#[serde(rename_all="kebab-case")]
struct Profile {
    codegen_units: u32,
    incremental: bool,
}

/// Available profile types
#[derive(Serialize)]
struct Profiles {
    dev: Profile,
    release: Profile,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
struct DependencySpec {
    #[serde(skip_serializing_if = "String::is_empty")]
    package: String,
    #[serde(serialize_with = "exact_version")]
    version: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    features: Vec<String>,
    #[serde(skip_serializing_if = "is_true")]
    default_features: bool,
}

fn exact_version<S>(version: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    format!("={}", version).serialize(serializer)
}

fn is_true(b: &bool) -> bool {
    *b
}

impl Modifications {
    fn blacklisted(&self, name: &str) -> bool {
        self.blacklist.iter().any(|n| n == name)
    }
}

lazy_static! {
    static ref MODIFICATIONS: Modifications = {
        let mut f = File::open("crate-modifications.toml")
            .expect("unable to open crate modifications file");

        let mut d = Vec::new();
        f.read_to_end(&mut d)
            .expect("unable to read crate modifications file");

        toml::from_slice(&d)
            .expect("unable to parse crate modifications file")
    };
}

impl TopCrates {
    /// List top 100 crates by number of downloads on crates.io.
    fn download() -> TopCrates {
        let resp =
            reqwest::get("https://crates.io/api/v1/crates?page=1&per_page=100&sort=downloads")
            .expect("Could not fetch top crates");
        assert!(resp.status().is_success());

        serde_json::from_reader(resp).expect("Invalid JSON")
    }

    /// Add crates that have been hand-picked
    fn add_curated_crates(&mut self) {
        self.crates.extend({
            MODIFICATIONS
                .additions
                .iter()
                .cloned()
                .map(|name| Crate { name })
        });
    }
}

/// Finds the features specified by the custom metadata of `pkg`.
///
/// Our custom metadata format looks like:
///
///     [package.metadata.playground]
///     default-features = true
///     features = ["std", "extra-traits"]
///     all-features = false
///
/// All fields are optional.
fn playground_metadata_features(pkg: &Package) -> Option<(Vec<String>, bool)> {
    let custom_metadata = pkg.manifest().custom_metadata()?;
    let playground_metadata = custom_metadata.get("playground")?;

    #[derive(Deserialize)]
    #[serde(default, rename_all = "kebab-case")]
    struct Metadata {
        features: Vec<String>,
        default_features: bool,
        all_features: bool,
    }

    impl Default for Metadata {
        fn default() -> Self {
            Metadata {
                features: Vec::new(),
                default_features: true,
                all_features: false,
            }
        }
    }

    let metadata = match playground_metadata.clone().try_into::<Metadata>() {
        Ok(metadata) => metadata,
        Err(err) => {
            eprintln!(
                "Failed to parse custom metadata for {} {}: {}",
                pkg.name(), pkg.version(), err);
            return None;
        }
    };

    // If `all-features` is set then we ignore `features`.
    let summary = pkg.summary();
    let mut enabled_features: BTreeSet<String> = if metadata.all_features {
        summary.features().keys().map(ToString::to_string).collect()
    } else {
        metadata.features.into_iter().collect()
    };

    // If not opting out of default features, remove default features from the
    // explicit features list. This avoids ongoing spurious diffs in our
    // generated Cargo.toml as default features are added to a library.
    if metadata.default_features {
        if let Some(default_feature_names) = summary.features().get("default") {
            enabled_features.remove("default");
            for feature in default_feature_names {
                enabled_features.remove(&feature.to_string(summary));
            }
        }
    }

    if !enabled_features.is_empty() || !metadata.default_features {
        Some((
            enabled_features.into_iter().collect(),
            metadata.default_features,
        ))
    } else {
        None
    }
}

fn write_manifest(manifest: TomlManifest, path: &str) {
    let mut f = File::create(path).expect("Unable to create Cargo.toml");
    let content = toml::to_vec(&manifest).expect("Couldn't serialize TOML");
    f.write_all(&content).expect("Couldn't write Cargo.toml");
}

fn main() {
    // Setup to interact with cargo.
    let config = Config::default().expect("Unable to create default Cargo config");
    let _lock = config.acquire_package_cache_lock();
    let crates_io = SourceId::crates_io(&config).expect("Unable to create crates.io source ID");
    let mut source = RegistrySource::remote(crates_io, &HashSet::new(), &config);
    source.update().expect("Unable to update registry");

    let mut top = TopCrates::download();
    top.add_curated_crates();

    // Find the newest (non-prerelease, non-yanked) versions of all
    // the interesting crates.
    let mut summaries = Vec::new();
    for Crate { ref name } in top.crates {
        if MODIFICATIONS.blacklisted(name) {
            continue;
        }

        // Query the registry for a summary of this crate.
        // Usefully, this doesn't seem to include yanked versions
        let dep = Dependency::parse_no_deprecated(name, None, crates_io)
            .unwrap_or_else(|e| panic!("Unable to parse dependency for {}: {}", name, e));

        let matches = source.query_vec(&dep).unwrap_or_else(|e| {
            panic!("Unable to query registry for {}: {}", name, e);
        });

        // Find the newest non-prelease version
        let summary = matches.into_iter()
            .filter(|summary| !summary.version().is_prerelease())
            .max_by_key(|summary| summary.version().clone())
            .unwrap_or_else(|| panic!("Registry has no viable versions of {}", name));

        // Add a dependency on this crate.
        summaries.push((summary, Method::Required {
            dev_deps: false,
            features: Default::default(),
            uses_default_features: true,
            all_features: false,
        }));
    }

    // Resolve transitive dependencies.
    let mut registry = PackageRegistry::new(&config)
        .expect("Unable to create package registry");
    registry.lock_patches();
    let try_to_use = HashSet::new();
    let resolve = resolver::resolve(&summaries, &[], &mut registry, &try_to_use, None, true)
        .expect("Unable to resolve dependencies");

    // Get the package information for all dependencies.
    let package_ids: Vec<_> = resolve
        .iter()
        .filter(|pkg| !MODIFICATIONS.blacklisted(pkg.name().as_str()))
        .map(|pkg| {
            PackageId::new(&pkg.name(), pkg.version(), crates_io).unwrap_or_else(|e| {
                panic!(
                    "Unable to build PackageId for {} {}: {}",
                    pkg.name(),
                    pkg.version(),
                    e
                )
            })
        })
        .collect();

    let mut sources = SourceMap::new();
    sources.insert(Box::new(source));

    let package_set =
        PackageSet::new(&package_ids, sources, &config).expect("Unable to create a PackageSet");

    let mut packages = package_set
        .get_many(package_set.package_ids())
        .expect("Unable to download packages");

    // Sort all packages by name then version (descending), so that
    // when we group them we know we get all the same crates together
    // and the newest version first.
    packages.sort_by(|a, b| {
        a.name()
            .cmp(&b.name())
            .then(a.version().cmp(&b.version()).reverse())
    });

    let mut dependencies = BTreeMap::new();
    let mut infos = Vec::new();

    for (name, pkgs) in &packages.into_iter().group_by(|pkg| pkg.name()) {
        let mut first = true;

        for pkg in pkgs {
            let version = pkg.version();

            let crate_name = pkg
                .targets()
                .iter()
                .flat_map(|target| match target.kind() {
                    TargetKind::Lib(_) => Some(target.crate_name()),
                    _ => None,
                })
                .next()
                .unwrap_or_else(|| panic!("{} did not have a library", name));

            // We see the newest version first. Any subsequent
            // versions will have their version appended so that they
            // are uniquely named
            let exposed_name = if first {
                crate_name.clone()
            } else {
                format!(
                    "{}_{}_{}_{}",
                    crate_name, version.major, version.minor, version.patch
                )
            };

            let (features, default_features) =
                playground_metadata_features(&pkg).unwrap_or_else(|| (Vec::new(), true));

            dependencies.insert(
                exposed_name.clone(),
                DependencySpec {
                    package: name.to_string(),
                    version: version.to_string(),
                    features,
                    default_features,
                },
            );

            infos.push(CrateInformation {
                name: name.to_string(),
                version: version.to_string(),
                id: exposed_name,
            });

            first = false;
        }
    }

    // Construct playground's Cargo.toml.
    let manifest = TomlManifest {
        package: TomlPackage {
            name: "playground".to_owned(),
            version: "0.0.1".to_owned(),
            authors: vec!["The Rust Playground".to_owned()],
        },
        profile: Profiles {
            dev: Profile { codegen_units: 1, incremental: false },
            release: Profile { codegen_units: 1, incremental: false },
        },
        dependencies,
    };

    // Write manifest file.
    let cargo_toml = "../compiler/base/Cargo.toml";
    write_manifest(manifest, cargo_toml);
    println!("wrote {}", cargo_toml);

    let path = "../compiler/base/crate-information.json";
    let mut f = File::create(path)
        .unwrap_or_else(|e| panic!("Unable to create {}: {}", path, e));
    serde_json::to_writer_pretty(&mut f, &infos)
        .unwrap_or_else(|e| panic!("Unable to write {}: {}", path, e));
    println!("Wrote {}", path);
}
